import { useEffect, useRef, useState } from "react";
import { invoke } from "@tauri-apps/api/core";
import { listen } from "@tauri-apps/api/event";
import { logActivity } from "@/lib/logActivity";
import { open } from "@tauri-apps/plugin-dialog";
import { Button } from "@/components/ui/button";
import { CheckCircle2, AlertTriangle, Info, FolderOpen, Image as ImageIcon } from "lucide-react";
import { SUBTYPES_BY_TYPE, WEAR_FLAGS } from "@/features/item-editor/itemFlags";
import {
  type ItemProtoInput,
  type ScannedModule,
  type StepStatus,
  type WeaponVariant,
  emptyItem,
  fileNameOf,
  fitToByteLimit,
  fitToByteLimitWithSuffix,
  findConsecutiveFreeVnums,
  labelForVariant,
  pad5,
  refineCostForStep,
  refineProbForStep,
  scaledValue,
  sizeForSubtype,
  classAntiflagForWeapon,
  classLabelFor,
  CLASS_ANTIFLAG_MASK,
  RACE_ANTIFLAG,
  RACE_LABELS,
  WEAPON_WEARFLAG,
  ITEM_NAME_MAX_BYTES,
} from "../shared";
import { StepRow, Field } from "./shared";
import { IconPicker } from "./IconPicker";
import { ReferenceItemPicker } from "./ReferenceItemPicker";

interface WeaponRowState {
  selected: boolean;
  subtype: number; // -1 = not yet chosen
  isSura: boolean;
  icon: string | null;
}

interface ArmorRowState {
  selected: boolean;
  icon: string | null;
}

export function PackageImporter({ onImported }: { onImported: () => void }) {
  const [folderPath, setFolderPath] = useState("");
  const [scanning, setScanning] = useState(false);
  const [scanResult, setScanResult] = useState<ScannedModule | null>(null);
  const [scanError, setScanError] = useState<string | null>(null);

  const [weaponRows, setWeaponRows] = useState<Record<string, WeaponRowState>>({});
  const [armorRows, setArmorRows] = useState<Record<string, ArmorRowState>>({});
  const [armorSubtype, setArmorSubtype] = useState(0);
  const [armorWearflag, setArmorWearflag] = useState(1 << 0); // "Rüstung (Körper)"

  const [baseName, setBaseName] = useState("");
  const [baseLocaleName, setBaseLocaleName] = useState("");
  const [description, setDescription] = useState("");
  const [summary, setSummary] = useState("");
  const [vnumRangeStart, setVnumRangeStart] = useState(500000);
  const [includeEffects, setIncludeEffects] = useState(true);
  const [refItem, setRefItem] = useState<ItemProtoInput | null>(null);

  const [chainEnabled, setChainEnabled] = useState(false);
  const [chainMaxLevel, setChainMaxLevel] = useState(9);
  const [chainGrowthPercent, setChainGrowthPercent] = useState(10);

  const [running, setRunning] = useState(false);
  const [weaponStatus, setWeaponStatus] = useState<Record<string, StepStatus>>({});
  const [armorStatus, setArmorStatus] = useState<Record<string, StepStatus>>({});
  const [otherSteps, setOtherSteps] = useState<Record<string, StepStatus>>({});
  const [log, setLog] = useState<string[]>([]);
  const [batchError, setBatchError] = useState<string | null>(null);
  const [batchDone, setBatchDone] = useState<number[] | null>(null);
  const [confirm, setConfirm] = useState(false);

  const createdVnumsRef = useRef<number[]>([]);
  const logRef = useRef<HTMLPreElement>(null);

  useEffect(() => {
    invoke<string | null>("get_setting", { key: "item_vnum_range_start" })
      .then((v) => setVnumRangeStart(Number(v ?? "500000")))
      .catch(() => {});
  }, []);

  useEffect(() => {
    const unlisten = listen<string>("item-editor-output", (event) => {
      setLog((prev) => [...prev, event.payload]);
    });
    return () => {
      unlisten.then((fn) => fn());
    };
  }, []);

  useEffect(() => {
    logRef.current?.scrollTo({ top: logRef.current.scrollHeight });
  }, [log]);

  async function pickFolder() {
    const selectedPath = await open({ directory: true, title: "Paket-Ordner wählen" });
    if (typeof selectedPath === "string") {
      setFolderPath(selectedPath);
      setScanResult(null);
      setScanError(null);
    }
  }

  async function scan() {
    if (!folderPath) return;
    setScanning(true);
    setScanError(null);
    try {
      const result = await invoke<ScannedModule>("scan_module", { path: folderPath });
      setScanResult(result);

      const wRows: Record<string, WeaponRowState> = {};
      for (const v of result.weapons) {
        wRows[v.key] = { selected: v.icon_source_abs !== null, subtype: v.subtype, isSura: v.is_sura_model, icon: v.icon_source_abs };
      }
      setWeaponRows(wRows);

      const aRows: Record<string, ArmorRowState> = {};
      for (const a of result.armors) {
        aRows[a.race] = { selected: a.female_model_abs !== null || a.male_model_abs !== null, icon: a.icon_source_abs };
      }
      setArmorRows(aRows);

      if (!baseName) setBaseName(result.module_name);
      if (!baseLocaleName) setBaseLocaleName(result.module_name);
      setWeaponStatus({});
      setArmorStatus({});
      setOtherSteps({});
      setBatchError(null);
      setBatchDone(null);
    } catch (e) {
      setScanError(String(e));
      setScanResult(null);
    } finally {
      setScanning(false);
    }
  }

  function buildWeaponItem(vnum: number, variant: WeaponVariant, row: WeaponRowState, level: number): ItemProtoInput {
    const template = refItem ?? emptyItem(vnum, 1);
    // Class restriction always comes from the variant's own subtype/model,
    // never from the reference item - a reference sword's antiflag would
    // otherwise silently make a Bell Warrior-equippable too. Non-class bits
    // (gender/empire/GET/DROP/SELL/...) from the reference item are kept.
    const antiflag = (template.antiflag & ~CLASS_ANTIFLAG_MASK) | classAntiflagForWeapon(row.subtype, row.isSura);
    const suffix = level > 0 ? `+${level}` : "";
    const grow = (base: number) => scaledValue(base, level, chainGrowthPercent);
    return {
      ...template,
      vnum,
      vnum_range: 0,
      name: fitToByteLimitWithSuffix(`${baseName || "item"}_${variant.key}`, suffix),
      locale_name: fitToByteLimitWithSuffix(
        `${baseLocaleName || baseName} (${labelForVariant(variant)})`,
        suffix ? ` ${suffix}` : "",
      ),
      type: 1,
      subtype: row.subtype,
      antiflag,
      size: sizeForSubtype(row.subtype),
      wearflag: template.wearflag || WEAPON_WEARFLAG,
      // Schaden min/max + Zusatz-Angriffskraft - die "Kampfwerte" laut
      // itemFlags.ts VALUE_HINTS. Alle drei fließen in die im Client
      // angezeigte/berechnete Schadensspanne ein (weaponDisplayDamage),
      // deshalb wachsen alle drei gleichmäßig statt nur einer davon.
      value3: grow(template.value3),
      value4: grow(template.value4),
      value5: grow(template.value5),
      // Jede Stufe startet ohne Aufwertungs-Verknüpfung - runImport
      // verdrahtet die Kette danach explizit über set_item_refine_link,
      // statt hier versehentlich die Verknüpfung eines evtl. bereits
      // aufgewerteten Referenz-Items mit zu übernehmen (die Ziel-Vnum
      // würde sonst ins Leere zeigen, "No advancement possible").
      refine_set: 0,
      refined_vnum: 0,
    };
  }

  function buildArmorItem(vnum: number, shapeIndex: number, selectedRaceKeys: string[], level: number): ItemProtoInput {
    const template = refItem ?? emptyItem(vnum, 2);
    const blockedMask = selectedRaceKeys.length
      ? Object.entries(RACE_ANTIFLAG)
          .filter(([race]) => !selectedRaceKeys.includes(race))
          .reduce((acc, [, bit]) => acc | bit, 0)
      : 0;
    const antiflag = (template.antiflag & ~CLASS_ANTIFLAG_MASK) | blockedMask;
    const suffix = level > 0 ? ` +${level}` : "";
    const grow = (base: number) => scaledValue(base, level, chainGrowthPercent);
    return {
      ...template,
      vnum,
      vnum_range: 0,
      name: fitToByteLimitWithSuffix(`${baseName || "item"}_armor`, level > 0 ? `+${level}` : ""),
      locale_name: fitToByteLimitWithSuffix(baseLocaleName || baseName, suffix),
      type: 2,
      subtype: armorSubtype,
      antiflag,
      // Anders als bei Waffen (size hängt vom Subtyp ab, siehe
      // sizeForSubtype) ist Rüstung im Inventar immer 1 Slot groß - das
      // MUSS hier explizit gesetzt werden statt vom Referenz-Item geerbt
      // zu werden (`...template` oben), sonst übernimmt ein z.B. als
      // Referenz gewähltes Schwert (size 2/3) versehentlich dessen
      // Grid-Höhe und die Rüstung überlappt Nachbar-Slots im Inventar
      // (realer Bug, live gemeldet 2026-08-10).
      size: 1,
      wearflag: armorWearflag || template.wearflag,
      // Magie-Verteidigung + Verteidigung (Grade) + Zusatz-Verteidigung -
      // die "Kampfwerte" für Rüstung laut VALUE_HINTS. value3 ist hier
      // NICHT dabei: das ist bei Rüstung der 3D-Modell-Shape-Index
      // (siehe msm.rs/[[m2manager_module_importer]]), live verifiziert
      // dass er über eine ganze echte Stock-Kette (11200-11209) konstant
      // bleibt - würde er hier mitwachsen, würde jede Stufe ein anderes
      // (meist nicht existierendes) 3D-Modell erwarten.
      value0: grow(template.value0),
      value1: grow(template.value1),
      value5: grow(template.value5),
      value3: shapeIndex,
      refine_set: 0,
      refined_vnum: 0,
    };
  }

  const selectedWeapons = (scanResult?.weapons ?? []).filter((v) => weaponRows[v.key]?.selected);
  const selectedArmorRaces = (scanResult?.armors ?? []).filter((a) => armorRows[a.race]?.selected);
  const armorFemaleRaces = selectedArmorRaces.filter((a) => a.female_model_abs !== null);
  const armorMaleOnlyRaces = selectedArmorRaces.filter((a) => a.female_model_abs === null && a.male_model_abs !== null);

  const canImport =
    !!scanResult &&
    (selectedWeapons.length > 0 || selectedArmorRaces.length > 0) &&
    baseName.trim().length > 0 &&
    baseLocaleName.trim().length > 0 &&
    selectedWeapons.every((v) => weaponRows[v.key]?.subtype !== -1);

  async function runImport() {
    setConfirm(false);
    setRunning(true);
    setBatchError(null);
    setBatchDone(null);
    setLog([]);
    createdVnumsRef.current = [];
    const wStatus: Record<string, StepStatus> = {};
    for (const v of selectedWeapons) wStatus[v.key] = "pending";
    setWeaponStatus(wStatus);
    const aStatus: Record<string, StepStatus> = {};
    if (selectedArmorRaces.length > 0) aStatus.armor = "pending";
    setArmorStatus(aStatus);
    setOtherSteps({ setup: "pending", pack: "pending", proto: "pending" });

    const touchedRacePacks = new Set<string>();

    try {
      setOtherSteps((prev) => ({ ...prev, setup: "running" }));
      await invoke("validate_item_editor_setup", { requireIconTool: true });
      setOtherSteps((prev) => ({ ...prev, setup: "done" }));

      let rangeStart = vnumRangeStart;
      const levels = chainEnabled ? chainMaxLevel + 1 : 1;

      for (const variant of selectedWeapons) {
        const row = weaponRows[variant.key];
        setWeaponStatus((prev) => ({ ...prev, [variant.key]: "running" }));

        // A whole chain needs a *contiguous* vnum block (matching stock
        // chains, e.g. 10-19) - the base vnum's own icon/model are shared
        // by every level via item_list.txt, exactly like stock does, so
        // they're only written once here, not once per level.
        const baseVnum = await findConsecutiveFreeVnums(rangeStart, levels);
        rangeStart = baseVnum + levels;

        if (row.icon) {
          await invoke("write_item_icon", { vnum: baseVnum, sourcePath: row.icon });
        }
        const [, virtualModelPath] = await invoke<[string, string]>("import_weapon_model", {
          moduleName: scanResult!.module_name,
          vnum: baseVnum,
          sourceAbs: variant.model_source_abs,
          textureSources: variant.texture_sources_abs,
        });
        const iconRelPath = `icon/item/${pad5(baseVnum)}.tga`;

        const levelVnums: number[] = [];
        for (let level = 0; level < levels; level++) {
          const vnum = baseVnum + level;
          levelVnums.push(vnum);
          const item = buildWeaponItem(vnum, variant, row, level);
          await invoke("create_item_proto", { item });
          createdVnumsRef.current.push(vnum);
          if (description.trim() || summary.trim()) {
            await invoke("write_item_desc", { vnum, description, summary });
          }
          await invoke("write_item_list_entry", { vnum, itemType: 1, iconRelPath, modelRelPath: virtualModelPath });
        }

        for (let level = 0; level < levels - 1; level++) {
          const recipeId = await invoke<number>("save_refine_recipe", {
            id: null,
            cost: refineCostForStep(level),
            prob: refineProbForStep(level),
            materials: [],
          });
          await invoke("set_item_refine_link", {
            vnum: levelVnums[level],
            refineSet: recipeId,
            refinedVnum: levelVnums[level + 1],
          });
        }

        setWeaponStatus((prev) => ({ ...prev, [variant.key]: "done" }));
      }

      if (selectedArmorRaces.length > 0) {
        setArmorStatus({ armor: "running" });

        const baseVnum = await findConsecutiveFreeVnums(rangeStart, levels);
        rangeStart = baseVnum + levels;

        const shapeIndex =
          armorFemaleRaces.length > 0 ? await invoke<number>("next_free_shape_index") : refItem?.value3 ?? 0;

        const icon = armorRows[selectedArmorRaces[0].race]?.icon;
        if (icon) {
          await invoke("write_item_icon", { vnum: baseVnum, sourcePath: icon });
        }
        const iconRelPath = `icon/item/${pad5(baseVnum)}.tga`;
        const raceKeys = selectedArmorRaces.map((a) => a.race);

        const levelVnums: number[] = [];
        for (let level = 0; level < levels; level++) {
          const vnum = baseVnum + level;
          levelVnums.push(vnum);
          const item = buildArmorItem(vnum, shapeIndex, raceKeys, level);
          await invoke("create_item_proto", { item });
          createdVnumsRef.current.push(vnum);
          if (description.trim() || summary.trim()) {
            await invoke("write_item_desc", { vnum, description, summary });
          }
          await invoke("write_item_list_entry", { vnum, itemType: 2, iconRelPath, modelRelPath: null });
        }

        for (let level = 0; level < levels - 1; level++) {
          const recipeId = await invoke<number>("save_refine_recipe", {
            id: null,
            cost: refineCostForStep(level),
            prob: refineProbForStep(level),
            materials: [],
          });
          await invoke("set_item_refine_link", {
            vnum: levelVnums[level],
            refineSet: recipeId,
            refinedVnum: levelVnums[level + 1],
          });
        }

        // The 3D-Modell is wired to the *base* vnum's shapeIndex only -
        // every other level shares the same ShapeData entry (see
        // buildArmorItem's own comment: value3 stays constant across the
        // whole chain, matching the real stock convention).
        for (const armor of armorFemaleRaces) {
          await invoke("import_armor_model", {
            moduleName: scanResult!.module_name,
            race: armor.race,
            sourceAbs: armor.female_model_abs,
            textureSources: armor.female_textures_abs,
            shapeIndex,
          });
          touchedRacePacks.add(`pc_${armor.race}`);
        }
        setArmorStatus({ armor: "done" });
      }

      setOtherSteps((prev) => ({ ...prev, pack: "running" }));
      if (selectedWeapons.length > 0) {
        await invoke("pack_item_icons");
        await invoke("pack_item_models");
      } else if (selectedArmorRaces.length > 0) {
        await invoke("pack_item_icons");
      }
      for (const folder of touchedRacePacks) {
        await invoke("pack_folder", { folderName: folder });
      }
      if (includeEffects && scanResult!.effect_dirs.length > 0) {
        for (const dir of scanResult!.effect_dirs) {
          await invoke("import_effect_bundle", { moduleName: scanResult!.module_name, sourceDir: dir });
        }
        await invoke("pack_item_effects");
      }
      setOtherSteps((prev) => ({ ...prev, pack: "done" }));

      setOtherSteps((prev) => ({ ...prev, proto: "running" }));
      const generated = await invoke<string>("regenerate_item_proto");
      await invoke("deploy_item_proto", { generatedProtoPath: generated });
      setOtherSteps((prev) => ({ ...prev, proto: "done" }));

      setBatchDone([...createdVnumsRef.current]);
      try {
        await invoke("record_import_batch", {
          moduleName: scanResult!.module_name,
          itemType: selectedWeapons.length > 0 && selectedArmorRaces.length === 0 ? 1 : 2,
          vnums: [...createdVnumsRef.current],
          hadEffects: includeEffects && scanResult!.effect_dirs.length > 0,
        });
        onImported();
      } catch {
        // History recording is best-effort - the import itself already
        // succeeded, so a failure here shouldn't look like an import error.
      }
    } catch (e) {
      let message = String(e);
      if (createdVnumsRef.current.length > 0) {
        const failed: number[] = [];
        for (const vnum of createdVnumsRef.current) {
          try {
            await invoke("rollback_created_item", { vnum });
            logActivity(
              "module-importer",
              "rollback-item",
              `Neu angelegtes Item zurückgenommen (vnum ${vnum}, Rollback nach fehlgeschlagenem Import)`,
              "item",
              String(vnum),
            );
          } catch {
            failed.push(vnum);
          }
        }
        message +=
          failed.length === 0
            ? `\n\nAlle ${createdVnumsRef.current.length} bereits angelegten Einträge (Datenbank, Icon, item_list.txt) wurden automatisch zurückgenommen.`
            : `\n\nRollback teilweise fehlgeschlagen - bitte vnums ${failed.join(", ")} manuell im DB Explorer prüfen.`;
      }
      setBatchError(message);
    } finally {
      setRunning(false);
    }
  }

  return (
    <div className="space-y-6">
      <section className="space-y-2 rounded-lg border border-border p-4">
        <h2 className="text-sm font-medium text-muted-foreground">1. Paket-Ordner wählen</h2>
        <p className="text-xs text-muted-foreground">
          Beliebiger Ordner mit Waffen- und/oder Rüstungs-Dateien - die Ordnerstruktur darf variieren, wird
          automatisch durchsucht.
        </p>
        <div className="flex items-center gap-2">
          <Button variant="outline" onClick={pickFolder}>
            <FolderOpen className="size-4" />
            Ordner wählen…
          </Button>
          <span className="truncate text-sm text-muted-foreground">{folderPath || "Kein Ordner gewählt"}</span>
          <Button onClick={scan} disabled={!folderPath || scanning}>
            {scanning ? "Scanne…" : "Scannen"}
          </Button>
        </div>
        {scanError && <p className="text-sm text-destructive">{scanError}</p>}
      </section>

      {scanResult && (
        <>
          {scanResult.notices.length > 0 && (
            <div className="space-y-1 rounded-lg border border-amber-500/40 bg-amber-500/10 p-3 text-sm text-amber-700 dark:text-amber-400">
              {scanResult.notices.map((n, i) => (
                <p key={i} className="flex items-start gap-2">
                  <Info className="mt-0.5 size-4 shrink-0" /> {n}
                </p>
              ))}
            </div>
          )}

          {scanResult.weapons.length === 0 && scanResult.armors.length === 0 && (
            <p className="text-sm text-muted-foreground">
              Keine Waffen-Modelle (<code>.gr2</code> außerhalb von <code>pc/pc2/pc3</code>) und keine
              Rüstungs-Körpermodelle gefunden.
            </p>
          )}

          {scanResult.weapons.length > 0 && (
            <section className="space-y-3 rounded-lg border border-border p-4">
              <h2 className="text-sm font-medium text-muted-foreground">
                2. Erkannte Waffen ({scanResult.weapons.length})
              </h2>
              <div className="space-y-2">
                {scanResult.weapons.map((v) => {
                  const row = weaponRows[v.key];
                  if (!row) return null;
                  const wantedLocaleName = `${baseLocaleName || baseName} (${labelForVariant(v)})`;
                  const finalLocaleName = fitToByteLimit(wantedLocaleName);
                  const truncated = finalLocaleName !== wantedLocaleName;
                  return (
                    <div key={v.key} className="flex flex-wrap items-center gap-3 rounded-md border border-border p-2 text-sm">
                      <input
                        type="checkbox"
                        checked={row.selected}
                        onChange={(e) =>
                          setWeaponRows((prev) => ({ ...prev, [v.key]: { ...prev[v.key], selected: e.target.checked } }))
                        }
                      />
                      <span className="w-40 font-medium">{labelForVariant(v)}</span>
                      <IconPicker
                        pool={scanResult.icon_pool}
                        value={row.icon}
                        onChange={(icon) => setWeaponRows((prev) => ({ ...prev, [v.key]: { ...prev[v.key], icon } }))}
                      />
                      {!row.icon && (
                        <span className="flex items-center gap-1 text-xs text-destructive">
                          <ImageIcon className="size-3.5" /> Kein Icon gewählt
                        </span>
                      )}
                      <Field label="Subtyp">
                        <select
                          value={row.subtype}
                          onChange={(e) =>
                            setWeaponRows((prev) => ({
                              ...prev,
                              [v.key]: { ...prev[v.key], subtype: Number(e.target.value) },
                            }))
                          }
                          className="rounded-md border border-border bg-background px-2 py-1 text-xs"
                        >
                          <option value={-1}>— wählen —</option>
                          {SUBTYPES_BY_TYPE[1].map((t) => (
                            <option key={t.value} value={t.value}>
                              {t.label}
                            </option>
                          ))}
                        </select>
                      </Field>
                      {row.subtype === 0 && (
                        <label className="flex items-center gap-1 text-xs text-muted-foreground">
                          <input
                            type="checkbox"
                            checked={row.isSura}
                            onChange={(e) =>
                              setWeaponRows((prev) => ({ ...prev, [v.key]: { ...prev[v.key], isSura: e.target.checked } }))
                            }
                          />
                          Sura-exklusiv
                        </label>
                      )}
                      {row.subtype !== -1 && (
                        <span className="text-xs text-muted-foreground">
                          Klasse: <strong>{classLabelFor(classAntiflagForWeapon(row.subtype, row.isSura))}</strong>
                        </span>
                      )}
                      <span className="ml-auto truncate text-xs text-muted-foreground" title={v.model_source_abs}>
                        {fileNameOf(v.model_source_abs)}
                        {v.texture_sources_abs.length > 0 && ` (+${v.texture_sources_abs.length} Textur(en))`}
                      </span>
                      <span className={`w-full text-xs ${truncated ? "text-amber-600" : "text-muted-foreground"}`}>
                        Anzeigename: <code>{finalLocaleName}</code>
                        {truncated && ` (gekürzt, max. ${ITEM_NAME_MAX_BYTES} Zeichen im Client)`}
                      </span>
                    </div>
                  );
                })}
              </div>
            </section>
          )}

          {scanResult.armors.length > 0 && (
            <section className="space-y-3 rounded-lg border border-border p-4">
              <h2 className="text-sm font-medium text-muted-foreground">
                {scanResult.weapons.length > 0 ? "3" : "2"}. Erkannte Rüstung ({scanResult.armors.length} Rasse(n))
              </h2>
              <p className="text-xs text-muted-foreground">
                Alle ausgewählten Rassen bilden <strong>ein gemeinsames Item</strong> (eine vnum), das je Rasse mit
                deren eigenem 3D-Modell erscheint - nicht ausgewählte Klassen können es nicht anlegen.
              </p>
              <div className="space-y-2">
                {scanResult.armors.map((a) => {
                  const row = armorRows[a.race];
                  if (!row) return null;
                  const hasFemale = a.female_model_abs !== null;
                  const hasMale = a.male_model_abs !== null;
                  return (
                    <div key={a.race} className="flex flex-wrap items-center gap-3 rounded-md border border-border p-2 text-sm">
                      <input
                        type="checkbox"
                        checked={row.selected}
                        onChange={(e) =>
                          setArmorRows((prev) => ({ ...prev, [a.race]: { ...prev[a.race], selected: e.target.checked } }))
                        }
                      />
                      <span className="w-28 font-medium">{RACE_LABELS[a.race] ?? a.race}</span>
                      <IconPicker
                        pool={scanResult.icon_pool}
                        value={row.icon}
                        onChange={(icon) => setArmorRows((prev) => ({ ...prev, [a.race]: { ...prev[a.race], icon } }))}
                      />
                      {hasFemale ? (
                        <span className="flex items-center gap-1 text-xs text-green-600">
                          <CheckCircle2 className="size-3.5" /> 3D-Modell wird automatisch verknüpft
                        </span>
                      ) : hasMale ? (
                        <span className="flex items-center gap-1 text-xs text-amber-600" title="Männliche Körpermodelle sind clientseitig fest kodiert (kein Datei-basiertes Mapping vorhanden) - nur Icon + Item werden angelegt.">
                          <AlertTriangle className="size-3.5" /> Nur männliches Modell - 3D nicht automatisierbar
                        </span>
                      ) : (
                        <span className="text-xs text-muted-foreground">Kein 3D-Modell gefunden</span>
                      )}
                    </div>
                  );
                })}
              </div>
              {armorMaleOnlyRaces.length > 0 && (
                <p className="flex items-start gap-2 text-xs text-muted-foreground">
                  <Info className="mt-0.5 size-3.5 shrink-0" />
                  Männliche Rüstungs-Körpermodelle sind im Client fest verdrahtet (keine Daten-Datei wie bei der
                  weiblichen Seite existiert dafür) - für {armorMaleOnlyRaces.map((a) => RACE_LABELS[a.race] ?? a.race).join(", ")}{" "}
                  wird nur das Item + Icon angelegt, das 3D-Modell bleibt wie bisher manuelle Arbeit.
                </p>
              )}
              <div className="flex flex-wrap gap-3 pt-1">
                <Field label="Subtyp">
                  <select
                    value={armorSubtype}
                    onChange={(e) => setArmorSubtype(Number(e.target.value))}
                    className="rounded-md border border-border bg-background px-2 py-1 text-xs"
                  >
                    {SUBTYPES_BY_TYPE[2].map((t) => (
                      <option key={t.value} value={t.value}>
                        {t.label}
                      </option>
                    ))}
                  </select>
                </Field>
                <Field label="Trageort">
                  <select
                    value={armorWearflag}
                    onChange={(e) => setArmorWearflag(Number(e.target.value))}
                    className="rounded-md border border-border bg-background px-2 py-1 text-xs"
                  >
                    {WEAR_FLAGS.map((t) => (
                      <option key={t.value} value={t.value}>
                        {t.label}
                      </option>
                    ))}
                  </select>
                </Field>
              </div>
            </section>
          )}

          <section className="space-y-3 rounded-lg border border-border p-4">
            <h2 className="text-sm font-medium text-muted-foreground">Basisdaten</h2>
            <div className="flex flex-wrap gap-3">
              <Field label="Name (intern, ASCII)">
                <input
                  value={baseName}
                  onChange={(e) => setBaseName(e.target.value)}
                  className="w-56 rounded-md border border-border bg-background px-2 py-1 text-sm"
                />
              </Field>
              <Field label="Anzeigename">
                <input
                  value={baseLocaleName}
                  onChange={(e) => setBaseLocaleName(e.target.value)}
                  className="w-56 rounded-md border border-border bg-background px-2 py-1 text-sm"
                />
              </Field>
              <Field label="VNUM-Bereich ab">
                <input
                  type="number"
                  value={vnumRangeStart}
                  onChange={(e) => setVnumRangeStart(Number(e.target.value) || 0)}
                  className="w-32 rounded-md border border-border bg-background px-2 py-1 text-sm"
                />
              </Field>
            </div>
            <div className="flex flex-wrap gap-3">
              <Field label="Beschreibung (Tooltip-Text, optional)">
                <textarea
                  value={description}
                  onChange={(e) => setDescription(e.target.value)}
                  className="h-16 w-72 rounded-md border border-border bg-background px-2 py-1 text-sm"
                />
              </Field>
              <Field label="Kurzbeschreibung (optional)">
                <textarea
                  value={summary}
                  onChange={(e) => setSummary(e.target.value)}
                  className="h-16 w-56 rounded-md border border-border bg-background px-2 py-1 text-sm"
                />
              </Field>
            </div>
            <p className="text-xs text-muted-foreground">
              Gilt für alle importierten Varianten/Stufen dieses Durchlaufs. Rein clientseitiger Tooltip-Text
              (<code>locale/&lt;lang&gt;/itemdesc.txt</code>) - ohne Eintrag bleibt das Item im Client ohne
              Beschreibung.
            </p>
            {scanResult.effect_dirs.length > 0 && (
              <label className="flex items-center gap-2 text-sm">
                <input type="checkbox" checked={includeEffects} onChange={(e) => setIncludeEffects(e.target.checked)} />
                {scanResult.effect_dirs.length} Glow-Effekt-Ordner mitkopieren (Best-Effort - danach im Client
                visuell prüfen)
              </label>
            )}
          </section>

          <section className="space-y-2 rounded-lg border border-border p-4">
            <h2 className="text-sm font-medium text-muted-foreground">Beispielwerte (optional): Referenz-Item übernehmen</h2>
            <p className="text-xs text-muted-foreground">
              Übernimmt Werte (value0-5, Preise, Flags) eines bestehenden Items als Ausgangsbasis für{" "}
              <strong>alle</strong> ausgewählten Waffen/Rüstung - Typ/Subtyp/VNUM/Name/Klassenbeschränkung werden
              trotzdem korrekt je Variante gesetzt. Ohne Auswahl bleiben die Werte auf 0 (bewusst kein Erraten von
              Balance-Zahlen).
            </p>
            <ReferenceItemPicker onPick={setRefItem} />
            {refItem && (
              <p className="text-xs text-green-600">
                Übernommen von <strong>{refItem.locale_name || refItem.name}</strong> (vnum {refItem.vnum}).{" "}
                <button className="underline" onClick={() => setRefItem(null)}>
                  Entfernen
                </button>
              </p>
            )}
          </section>

          <section className="space-y-3 rounded-lg border border-border p-4">
            <h2 className="text-sm font-medium text-muted-foreground">Aufwertungs-Kette (optional)</h2>
            <label className="flex items-center gap-2 text-sm">
              <input type="checkbox" checked={chainEnabled} onChange={(e) => setChainEnabled(e.target.checked)} />
              Jedes Item bekommt eine eigene Aufwertungs-Kette (+0 bis +N), aufrüstbar wie bei Stock-Items
            </label>
            {chainEnabled && (
              <>
                <div className="flex flex-wrap gap-3">
                  <Field label="Maximale Stufe (+N)">
                    <input
                      type="number"
                      min={1}
                      max={30}
                      value={chainMaxLevel}
                      onChange={(e) => setChainMaxLevel(Math.max(1, Number(e.target.value) || 1))}
                      className="w-24 rounded-md border border-border bg-background px-2 py-1 text-sm"
                    />
                  </Field>
                  <Field label="Werte-Wachstum pro Stufe (%)">
                    <input
                      type="number"
                      min={0}
                      value={chainGrowthPercent}
                      onChange={(e) => setChainGrowthPercent(Math.max(0, Number(e.target.value) || 0))}
                      className="w-24 rounded-md border border-border bg-background px-2 py-1 text-sm"
                    />
                  </Field>
                </div>
                <p className="text-xs text-muted-foreground">
                  Waffe: Schaden min/max + Zusatz-Angriffskraft wachsen. Rüstung: Magie-/Verteidigung + Zusatz-
                  Verteidigung wachsen (das 3D-Modell bleibt über die ganze Kette gleich). Ohne gewähltes
                  Referenz-Item bleibt die Basis 0 - dann wächst auch nichts.{" "}
                  {!refItem && <span className="text-amber-600">Aktuell kein Referenz-Item gewählt.</span>}
                </p>
                <p className="text-xs text-muted-foreground">
                  Aufwertungs-Kosten/-Chance je Stufe folgen der echten Stock-Vorlage dieses Servers (600 Gold/90%
                  für Stufe 1, steigend bis 75.000 Gold/60% ab Stufe 9 - danach bleiben Kosten/Chance von +9
                  gleich). Keine Materialien nötig - lässt sich danach jederzeit im Aufwertungs-Editor ergänzen.
                </p>
              </>
            )}
          </section>

          <section className="space-y-3 rounded-lg border border-border p-4">
            <h2 className="text-sm font-medium text-muted-foreground">Importieren</h2>
            <Button disabled={!canImport || running} onClick={() => setConfirm(true)}>
              {running
                ? "Importiere…"
                : `${(selectedWeapons.length + (selectedArmorRaces.length > 0 ? 1 : 0)) * (chainEnabled ? chainMaxLevel + 1 : 1)} Item(s) importieren`}
            </Button>

            {(Object.keys(weaponStatus).length > 0 || Object.keys(armorStatus).length > 0) && (
              <div className="space-y-1 text-sm">
                <StepRow label="Voraussetzungen prüfen (Tool-Pfade)" status={otherSteps.setup} />
                {selectedWeapons.map((v) => (
                  <StepRow key={v.key} label={`Waffe: ${labelForVariant(v)}`} status={weaponStatus[v.key]} />
                ))}
                {selectedArmorRaces.length > 0 && (
                  <StepRow
                    label={`Rüstung: ${selectedArmorRaces.map((a) => RACE_LABELS[a.race] ?? a.race).join(", ")}`}
                    status={armorStatus.armor}
                  />
                )}
                <StepRow label="Paket-Dateien neu packen (icon/item/pc_*/effect)" status={otherSteps.pack} />
                <StepRow label="item_proto aus DB erzeugen & in Client einspielen" status={otherSteps.proto} />
              </div>
            )}

            {batchError && <p className="whitespace-pre-wrap text-sm text-destructive">{batchError}</p>}
            {batchDone && (
              <p className="flex items-center gap-1 text-sm text-green-600">
                <CheckCircle2 className="size-4" /> {batchDone.length} Item(s) angelegt: vnums{" "}
                {batchDone.join(", ")}
              </p>
            )}
            {log.length > 0 && (
              <pre ref={logRef} className="max-h-48 overflow-y-auto rounded-md bg-muted/40 p-2 text-xs">
                {log.join("\n")}
              </pre>
            )}
          </section>
        </>
      )}

      {confirm && (
        <div className="fixed inset-0 flex items-center justify-center bg-black/50">
          <div className="w-[28rem] space-y-3 rounded-lg border border-border bg-card p-4">
            <p className="text-sm font-medium">
              {(selectedWeapons.length + (selectedArmorRaces.length > 0 ? 1 : 0)) * (chainEnabled ? chainMaxLevel + 1 : 1)}{" "}
              Item(s) für „{baseLocaleName}" jetzt anlegen?
            </p>
            <ul className="list-disc space-y-1 pl-5 text-sm text-muted-foreground">
              {selectedWeapons.length > 0 && (
                <li>{selectedWeapons.length} Waffen-Item(s): {selectedWeapons.map((v) => labelForVariant(v)).join(", ")}</li>
              )}
              {selectedArmorRaces.length > 0 && (
                <li>
                  1 Rüstungs-Item für {selectedArmorRaces.map((a) => RACE_LABELS[a.race] ?? a.race).join(", ")}
                  {armorFemaleRaces.length > 0 &&
                    ` (3D-Modell für ${armorFemaleRaces.map((a) => RACE_LABELS[a.race] ?? a.race).join(", ")} verknüpft)`}
                </li>
              )}
              {chainEnabled && (
                <li>
                  Jedes Item bekommt eine Aufwertungs-Kette bis +{chainMaxLevel} ({chainGrowthPercent}% Werte-
                  Wachstum pro Stufe) - wirkt erst nach einem Server-Neustart, da <code>refine_proto</code> nur
                  beim Serverstart aus der DB geladen wird
                </li>
              )}
              <li className={description.trim() || summary.trim() ? "" : "text-amber-600"}>
                {description.trim() || summary.trim()
                  ? "Beschreibung wird als itemdesc.txt-Eintrag geschrieben"
                  : "Keine Beschreibung eingetragen - Item bleibt im Client ohne Tooltip-Text"}
              </li>
              <li className={refItem ? "" : "text-amber-600"}>
                {refItem
                  ? `Werte werden von "${refItem.locale_name || refItem.name}" (vnum ${refItem.vnum}) übernommen`
                  : "Kein Referenz-Item gewählt - alle Werte (Schaden/Verteidigung, Preise, …) bleiben 0"}
              </li>
              <li>Icons werden geschrieben, <code>icon.epk</code> neu gepackt</li>
              {selectedWeapons.length > 0 && <li>Waffen-Modelle werden kopiert, <code>item.epk</code> neu gepackt</li>}
              {armorFemaleRaces.length > 0 && (
                <li>Rüstungs-Modelle werden kopiert, betroffene <code>pc_*.epk</code> neu gepackt</li>
              )}
              {includeEffects && scanResult!.effect_dirs.length > 0 && (
                <li>Glow-Effekte werden kopiert, <code>effect.epk</code> neu gepackt</li>
              )}
              <li><code>item_list.txt</code> wird ergänzt (Backups werden vorher angelegt)</li>
              <li><code>item_proto</code> wird aus der DB neu erzeugt und im Client ersetzt (Backup wird vorher angelegt)</li>
            </ul>
            <div className="flex justify-end gap-2">
              <Button variant="outline" onClick={() => setConfirm(false)}>
                Abbrechen
              </Button>
              <Button onClick={runImport}>Ausführen</Button>
            </div>
          </div>
        </div>
      )}
    </div>
  );
}
