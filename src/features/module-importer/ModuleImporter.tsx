import { useEffect, useRef, useState } from "react";
import { invoke } from "@tauri-apps/api/core";
import { listen } from "@tauri-apps/api/event";
import { open } from "@tauri-apps/plugin-dialog";
import { Button } from "@/components/ui/button";
import { Search, CheckCircle2, AlertTriangle, Info, FolderOpen, Image as ImageIcon, Trash2 } from "lucide-react";
import { SUBTYPES_BY_TYPE, WEAR_FLAGS } from "@/features/item-editor/itemFlags";

// Same shape as the local (unexported) ItemProtoInput in ItemEditor.tsx -
// duplicated rather than imported, matching that file's own convention of
// keeping this interface private to whoever builds create_item_proto
// payloads.
interface ItemProtoInput {
  vnum: number;
  vnum_range: number;
  name: string;
  locale_name: string;
  type: number;
  subtype: number;
  weight: number;
  size: number;
  antiflag: number;
  flag: number;
  wearflag: number;
  immuneflag: number;
  gold: number;
  shop_buy_price: number;
  refined_vnum: number;
  refine_set: number;
  magic_pct: number;
  limittype0: number;
  limitvalue0: number;
  limittype1: number;
  limitvalue1: number;
  applytype0: number;
  applyvalue0: number;
  applytype1: number;
  applyvalue1: number;
  applytype2: number;
  applyvalue2: number;
  applytype3: number;
  applyvalue3: number;
  value0: number;
  value1: number;
  value2: number;
  value3: number;
  value4: number;
  value5: number;
  socket0: number;
  socket1: number;
  socket2: number;
  socket3: number;
  socket4: number;
  socket5: number;
  specular: number;
  socket_pct: number;
  addon_type: number;
}

function emptyItem(vnum: number, type: number): ItemProtoInput {
  return {
    vnum,
    vnum_range: 0,
    name: "",
    locale_name: "",
    type,
    subtype: 0,
    weight: 0,
    size: 1,
    antiflag: 0,
    flag: 0,
    wearflag: 0,
    immuneflag: 0,
    gold: 0,
    shop_buy_price: 0,
    refined_vnum: 0,
    refine_set: 0,
    magic_pct: 0,
    limittype0: 0,
    limitvalue0: 0,
    limittype1: 0,
    limitvalue1: 0,
    applytype0: 0,
    applyvalue0: 0,
    applytype1: 0,
    applyvalue1: 0,
    applytype2: 0,
    applyvalue2: 0,
    applytype3: 0,
    applyvalue3: 0,
    value0: 0,
    value1: 0,
    value2: 0,
    value3: 0,
    value4: 0,
    value5: 0,
    socket0: 0,
    socket1: 0,
    socket2: 0,
    socket3: 0,
    socket4: 0,
    socket5: 0,
    specular: 0,
    socket_pct: 0,
    addon_type: 0,
  };
}

interface ItemSearchResult {
  vnum: number;
  name: string;
}

// Mirrors modulescan.rs's Rust structs (Serialize) 1:1.
interface WeaponVariant {
  key: string;
  label: string;
  subtype: number; // -1 = unrecognized, manual pick needed
  is_sura_model: boolean;
  model_source_abs: string;
  texture_sources_abs: string[];
  icon_source_abs: string | null;
}

interface ArmorPiece {
  race: string; // warrior | assassin | sura | shaman
  label: string;
  male_model_abs: string | null;
  male_textures_abs: string[];
  female_model_abs: string | null;
  female_textures_abs: string[];
  icon_source_abs: string | null;
}

interface ScannedModule {
  module_name: string;
  weapons: WeaponVariant[];
  armors: ArmorPiece[];
  icon_pool: string[];
  effect_dirs: string[];
  notices: string[];
}

const RACE_LABELS: Record<string, string> = {
  warrior: "Krieger",
  assassin: "Ninja",
  sura: "Sura",
  shaman: "Schamane",
};

// Friendlier German labels for known suffixes - falls back to the raw key
// (still readable, e.g. "custom_glaive") for anything unrecognized.
const VARIANT_LABELS: Record<string, string> = {
  sword: "Schwert",
  blade: "Schwert",
  saber: "Schwert",
  sabre: "Schwert",
  katana: "Schwert",
  "1h": "Einhand",
  sura: "Sura-Variante",
  dagger: "Dolch",
  knife: "Dolch",
  bow: "Bogen",
  "2hand": "Zweihand",
  twohand: "Zweihand",
  "2h": "Zweihand",
  spear: "Zweihand",
  glaive: "Zweihand",
  bell: "Glocke",
  staff: "Glocke (Stab)",
  fan: "Fächer",
  arrow: "Pfeil",
  claw: "Kralle",
};

function fileNameOf(absPath: string): string {
  return absPath.split(/[\\/]/).pop() ?? absPath;
}

function labelForVariant(v: WeaponVariant): string {
  for (const [needle, label] of Object.entries(VARIANT_LABELS)) {
    if (v.label.includes(needle)) return v.is_sura_model && needle !== "sura" ? `${label} (Sura)` : label;
  }
  return v.label;
}

// item_proto.name/.locale_name are a fixed-width varbinary column
// (ITEM_NAME_MAX_LEN = 24 in the client's ItemData.h) - MySQL rejects the
// insert outright ("Data too long") rather than truncating, so anything
// built from user-editable base name + suffix must be clamped client-side
// before it ever reaches create_item_proto. Byte length matters, not char
// length, in case the base name contains multi-byte UTF-8 characters.
const ITEM_NAME_MAX_BYTES = 24;

function fitToByteLimit(s: string, maxBytes = ITEM_NAME_MAX_BYTES): string {
  const encoder = new TextEncoder();
  if (encoder.encode(s).length <= maxBytes) return s;
  let truncated = s;
  while (truncated.length > 0 && encoder.encode(truncated).length > maxBytes) {
    truncated = truncated.slice(0, -1);
  }
  return truncated;
}

type StepStatus = "pending" | "running" | "done" | "error";
const WEAPON_WEARFLAG = 1 << 4; // "Waffe" bit, see itemFlags.ts WEAR_FLAGS

// item_proto.antiflag bits, see itemFlags.ts ANTI_FLAGS (source-verified).
const ANTIFLAG_NO_WARRIOR = 1 << 2;
const ANTIFLAG_NO_NINJA = 1 << 3;
const ANTIFLAG_NO_SURA = 1 << 4;
const ANTIFLAG_NO_SHAMAN = 1 << 5;
const ANTIFLAG_NO_WOLFMAN = 1 << 18;
const CLASS_ANTIFLAG_MASK =
  ANTIFLAG_NO_WARRIOR | ANTIFLAG_NO_NINJA | ANTIFLAG_NO_SURA | ANTIFLAG_NO_SHAMAN | ANTIFLAG_NO_WOLFMAN;
const RACE_ANTIFLAG: Record<string, number> = {
  warrior: ANTIFLAG_NO_WARRIOR,
  assassin: ANTIFLAG_NO_NINJA,
  sura: ANTIFLAG_NO_SURA,
  shaman: ANTIFLAG_NO_SHAMAN,
};

// Which class(es) can equip a weapon depends only on its subtype (and, for
// swords specifically, whether it's the dedicated Sura reskin) - a "Bell"
// must never be Warrior-equippable just because nothing said otherwise.
// These exact bit combinations were verified live against this server's
// real item_proto (grouped `antiflag` per `subtype` on ~2000 stock weapons,
// 2026-08-04), not guessed. item_proto.size = inventory grid footprint
// (1-3 slots tall, always 1 slot wide), depends only on weapon subtype,
// verified the same way (2026-08-05).
function sizeForSubtype(subtype: number): number {
  switch (subtype) {
    case 0: // Sword
    case 2: // Bow
      return 2;
    case 3: // Two-Handed
    case 7: // Mount Spear
      return 3;
    default: // Dagger, Bell, Fan, Arrow, Claw, ...
      return 1;
  }
}

function classAntiflagForWeapon(subtype: number, isSuraModel: boolean): number {
  switch (subtype) {
    case 0: // Sword
      return isSuraModel
        ? ANTIFLAG_NO_WARRIOR | ANTIFLAG_NO_NINJA | ANTIFLAG_NO_SHAMAN | ANTIFLAG_NO_WOLFMAN // Sura only
        : ANTIFLAG_NO_SHAMAN | ANTIFLAG_NO_WOLFMAN; // Warrior + Ninja + Sura
    case 1: // Dagger
    case 2: // Bow
    case 6: // Arrow
      return ANTIFLAG_NO_WARRIOR | ANTIFLAG_NO_SURA | ANTIFLAG_NO_SHAMAN | ANTIFLAG_NO_WOLFMAN; // Ninja only
    case 3: // Two-Handed
      return ANTIFLAG_NO_NINJA | ANTIFLAG_NO_SURA | ANTIFLAG_NO_SHAMAN | ANTIFLAG_NO_WOLFMAN; // Warrior only
    case 4: // Bell
    case 5: // Fan
      return ANTIFLAG_NO_WARRIOR | ANTIFLAG_NO_NINJA | ANTIFLAG_NO_SURA | ANTIFLAG_NO_WOLFMAN; // Shaman only
    case 8: // Claw
      return ANTIFLAG_NO_WARRIOR | ANTIFLAG_NO_NINJA | ANTIFLAG_NO_SURA | ANTIFLAG_NO_SHAMAN; // Wolfman only
    default:
      return 0;
  }
}

function classLabelFor(blockedMask: number): string {
  const classes = [
    { bit: ANTIFLAG_NO_WARRIOR, label: "Krieger" },
    { bit: ANTIFLAG_NO_NINJA, label: "Ninja" },
    { bit: ANTIFLAG_NO_SURA, label: "Sura" },
    { bit: ANTIFLAG_NO_SHAMAN, label: "Schamane" },
  ]
    .filter((c) => (blockedMask & c.bit) === 0)
    .map((c) => c.label);
  return classes.length > 0 ? classes.join(" + ") : "alle Klassen";
}

function pad5(vnum: number) {
  return String(vnum).padStart(5, "0");
}

function StepRow({ label, status }: { label: string; status?: StepStatus }) {
  return (
    <div className="flex items-center gap-2">
      {status === "done" && <CheckCircle2 className="size-4 text-green-600" />}
      {status === "error" && <AlertTriangle className="size-4 text-destructive" />}
      {status === "running" && (
        <span className="size-4 animate-spin rounded-full border-2 border-muted-foreground border-t-transparent" />
      )}
      {(!status || status === "pending") && <span className="size-4 shrink-0" />}
      <span className={status === "error" ? "text-destructive" : ""}>{label}</span>
    </div>
  );
}

function Field({ label, children }: { label: string; children: React.ReactNode }) {
  return (
    <label className="flex flex-col gap-1 text-xs text-muted-foreground">
      {label}
      {children}
    </label>
  );
}

function IconPicker({
  pool,
  value,
  onChange,
}: {
  pool: string[];
  value: string | null;
  onChange: (v: string | null) => void;
}) {
  return (
    <select
      value={value ?? ""}
      onChange={(e) => onChange(e.target.value || null)}
      className="max-w-[12rem] rounded-md border border-border bg-background px-2 py-1 text-xs"
    >
      <option value="">— kein Icon —</option>
      {pool.map((p) => (
        <option key={p} value={p}>
          {fileNameOf(p)}
        </option>
      ))}
    </select>
  );
}

function ReferenceItemPicker({ onPick }: { onPick: (item: ItemProtoInput) => void }) {
  const [query, setQuery] = useState("");
  const [results, setResults] = useState<ItemSearchResult[]>([]);
  const [searching, setSearching] = useState(false);
  const [pickError, setPickError] = useState<string | null>(null);

  async function run() {
    if (!query.trim()) return;
    setSearching(true);
    try {
      setResults(await invoke<ItemSearchResult[]>("search_items", { query: query.trim() }));
    } finally {
      setSearching(false);
    }
  }

  // "Übernehmen" must never fail silently: the user relies on these values
  // ending up in the import, so a null result or invoke error has to be
  // visible immediately - live-tested 2026-08-05, an import ran with all
  // values 0 because the picked reference was never actually applied.
  async function pick(vnum: number) {
    setPickError(null);
    try {
      const full = await invoke<ItemProtoInput | null>("get_item_proto", { vnum });
      if (!full) {
        setPickError(`Item ${vnum} konnte nicht geladen werden (nicht gefunden).`);
        return;
      }
      onPick(full);
      setResults([]);
      setQuery("");
    } catch (e) {
      setPickError(String(e));
    }
  }

  return (
    <div className="space-y-2">
      <div className="flex gap-2">
        <input
          value={query}
          onChange={(e) => setQuery(e.target.value)}
          onKeyDown={(e) => e.key === "Enter" && run()}
          placeholder="Item nach Name oder VNUM suchen…"
          className="flex-1 rounded-md border border-border bg-background px-2 py-1 text-sm"
        />
        <Button variant="outline" onClick={run} disabled={searching}>
          <Search className="size-4" />
        </Button>
      </div>
      {results.length > 0 && (
        <div className="max-h-40 space-y-1 overflow-y-auto">
          {results.map((r) => (
            <div key={r.vnum} className="flex items-center justify-between rounded-md px-2 py-1 text-sm hover:bg-muted">
              <span>
                {r.name} <span className="text-muted-foreground">#{r.vnum}</span>
              </span>
              <Button size="sm" variant="outline" onClick={() => pick(r.vnum)}>
                Übernehmen
              </Button>
            </div>
          ))}
        </div>
      )}
      {pickError && <p className="text-sm text-destructive">{pickError}</p>}
    </div>
  );
}

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

function PackageImporter({ onImported }: { onImported: () => void }) {
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
  const [vnumRangeStart, setVnumRangeStart] = useState(500000);
  const [includeEffects, setIncludeEffects] = useState(true);
  const [refItem, setRefItem] = useState<ItemProtoInput | null>(null);

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

  function buildWeaponItem(vnum: number, variant: WeaponVariant, row: WeaponRowState): ItemProtoInput {
    const template = refItem ?? emptyItem(vnum, 1);
    // Class restriction always comes from the variant's own subtype/model,
    // never from the reference item - a reference sword's antiflag would
    // otherwise silently make a Bell Warrior-equippable too. Non-class bits
    // (gender/empire/GET/DROP/SELL/...) from the reference item are kept.
    const antiflag = (template.antiflag & ~CLASS_ANTIFLAG_MASK) | classAntiflagForWeapon(row.subtype, row.isSura);
    return {
      ...template,
      vnum,
      vnum_range: 0,
      name: fitToByteLimit(`${baseName || "item"}_${variant.key}`),
      locale_name: fitToByteLimit(`${baseLocaleName || baseName} (${labelForVariant(variant)})`),
      type: 1,
      subtype: row.subtype,
      antiflag,
      size: sizeForSubtype(row.subtype),
      wearflag: template.wearflag || WEAPON_WEARFLAG,
      refined_vnum: 0,
    };
  }

  function buildArmorItem(vnum: number, shapeIndex: number, selectedRaceKeys: string[]): ItemProtoInput {
    const template = refItem ?? emptyItem(vnum, 2);
    const blockedMask = selectedRaceKeys.length
      ? Object.entries(RACE_ANTIFLAG)
          .filter(([race]) => !selectedRaceKeys.includes(race))
          .reduce((acc, [, bit]) => acc | bit, 0)
      : 0;
    const antiflag = (template.antiflag & ~CLASS_ANTIFLAG_MASK) | blockedMask;
    return {
      ...template,
      vnum,
      vnum_range: 0,
      name: fitToByteLimit(`${baseName || "item"}_armor`),
      locale_name: fitToByteLimit(baseLocaleName || baseName),
      type: 2,
      subtype: armorSubtype,
      antiflag,
      wearflag: armorWearflag || template.wearflag,
      value3: shapeIndex,
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

      for (const variant of selectedWeapons) {
        const row = weaponRows[variant.key];
        setWeaponStatus((prev) => ({ ...prev, [variant.key]: "running" }));
        const vnum = await invoke<number>("next_free_item_vnum", { rangeStart });
        rangeStart = vnum + 1;

        const item = buildWeaponItem(vnum, variant, row);
        await invoke("create_item_proto", { item });
        createdVnumsRef.current.push(vnum);

        if (row.icon) {
          await invoke("write_item_icon", { vnum, sourcePath: row.icon });
        }
        const [, virtualModelPath] = await invoke<[string, string]>("import_weapon_model", {
          moduleName: scanResult!.module_name,
          sourceAbs: variant.model_source_abs,
          textureSources: variant.texture_sources_abs,
        });
        await invoke("write_item_list_entry", {
          vnum,
          itemType: 1,
          iconRelPath: `icon/item/${pad5(vnum)}.tga`,
          modelRelPath: virtualModelPath,
        });
        setWeaponStatus((prev) => ({ ...prev, [variant.key]: "done" }));
      }

      if (selectedArmorRaces.length > 0) {
        setArmorStatus({ armor: "running" });
        const vnum = await invoke<number>("next_free_item_vnum", { rangeStart });
        rangeStart = vnum + 1;

        const shapeIndex =
          armorFemaleRaces.length > 0 ? await invoke<number>("next_free_shape_index") : refItem?.value3 ?? 0;

        const item = buildArmorItem(
          vnum,
          shapeIndex,
          selectedArmorRaces.map((a) => a.race),
        );
        await invoke("create_item_proto", { item });
        createdVnumsRef.current.push(vnum);

        const icon = armorRows[selectedArmorRaces[0].race]?.icon;
        if (icon) {
          await invoke("write_item_icon", { vnum, sourcePath: icon });
        }
        await invoke("write_item_list_entry", {
          vnum,
          itemType: 2,
          iconRelPath: `icon/item/${pad5(vnum)}.tga`,
          modelRelPath: null,
        });

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
            await invoke("delete_item_proto", { vnum });
          } catch {
            failed.push(vnum);
          }
        }
        message +=
          failed.length === 0
            ? `\n\nAlle ${createdVnumsRef.current.length} bereits angelegten Datenbankeinträge wurden automatisch zurückgenommen.`
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
            <h2 className="text-sm font-medium text-muted-foreground">Importieren</h2>
            <Button disabled={!canImport || running} onClick={() => setConfirm(true)}>
              {running
                ? "Importiere…"
                : `${selectedWeapons.length + (selectedArmorRaces.length > 0 ? 1 : 0)} Item(s) importieren`}
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
              {selectedWeapons.length + (selectedArmorRaces.length > 0 ? 1 : 0)} Item(s) für „{baseLocaleName}" jetzt
              anlegen?
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

interface ImportBatch {
  id: number;
  module_name: string;
  item_type: number;
  created_at: string;
  vnums: number[];
  had_effects: boolean;
}

function ImportHistory({ refreshKey }: { refreshKey: number }) {
  const [batches, setBatches] = useState<ImportBatch[]>([]);
  const [undoingId, setUndoingId] = useState<number | null>(null);
  const [confirmId, setConfirmId] = useState<number | null>(null);
  const [error, setError] = useState<string | null>(null);

  async function refresh() {
    try {
      setBatches(await invoke<ImportBatch[]>("list_import_batches"));
    } catch {
      // No history yet / not reachable - just show the empty state.
    }
  }

  useEffect(() => {
    refresh();
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, [refreshKey]);

  async function undo(id: number) {
    setConfirmId(null);
    setUndoingId(id);
    setError(null);
    try {
      await invoke("undo_import_batch", { id });
      await refresh();
    } catch (e) {
      setError(String(e));
    } finally {
      setUndoingId(null);
    }
  }

  const confirmBatch = batches.find((b) => b.id === confirmId);

  return (
    <section className="space-y-2 rounded-lg border border-border p-4">
      <h2 className="text-sm font-medium text-muted-foreground">Importierte Pakete (Verlauf)</h2>
      {batches.length === 0 ? (
        <p className="text-sm text-muted-foreground">Noch keine Importe in diesem Verlauf.</p>
      ) : (
        <div className="space-y-2">
          {batches.map((b) => (
            <div
              key={b.id}
              className="flex flex-wrap items-center justify-between gap-3 rounded-md border border-border p-2 text-sm"
            >
              <div>
                <p className="font-medium">
                  {b.module_name}{" "}
                  <span className="text-xs font-normal text-muted-foreground">
                    ({b.item_type === 1 ? "Waffen" : "Rüstung/gemischt"})
                  </span>
                </p>
                <p className="text-xs text-muted-foreground">
                  {new Date(b.created_at).toLocaleString()} · {b.vnums.length} Item(s): {b.vnums.join(", ")}
                </p>
              </div>
              <Button
                variant="destructive"
                size="sm"
                disabled={undoingId === b.id}
                onClick={() => setConfirmId(b.id)}
              >
                <Trash2 className="size-3.5" />
                {undoingId === b.id ? "Entferne…" : "Rückgängig machen"}
              </Button>
            </div>
          ))}
        </div>
      )}
      {error && <p className="whitespace-pre-wrap text-sm text-destructive">{error}</p>}

      {confirmBatch && (
        <div className="fixed inset-0 flex items-center justify-center bg-black/50">
          <div className="w-[26rem] space-y-3 rounded-lg border border-border bg-card p-4">
            <p className="text-sm font-medium">
              {confirmBatch.vnums.length} Item(s) aus „{confirmBatch.module_name}" endgültig entfernen?
            </p>
            <ul className="list-disc space-y-1 pl-5 text-sm text-muted-foreground">
              <li>Datenbankeinträge werden gelöscht</li>
              <li>Icons und <code>item_list.txt</code>-Einträge werden entfernt, <code>icon.epk</code> neu gepackt</li>
              <li><code>item_proto</code> wird neu erzeugt und im Client ersetzt</li>
              <li>Kopierte 3D-Modell-/Effekt-/msm-Dateien bleiben liegen (harmlos ohne DB-Eintrag)</li>
            </ul>
            <div className="flex justify-end gap-2">
              <Button variant="outline" onClick={() => setConfirmId(null)}>
                Abbrechen
              </Button>
              <Button variant="destructive" onClick={() => undo(confirmBatch.id)}>
                Endgültig entfernen
              </Button>
            </div>
          </div>
        </div>
      )}
    </section>
  );
}

function QuickRemoveItem() {
  const [query, setQuery] = useState("");
  const [results, setResults] = useState<ItemSearchResult[]>([]);
  const [searching, setSearching] = useState(false);
  const [removingVnum, setRemovingVnum] = useState<number | null>(null);
  const [confirmVnum, setConfirmVnum] = useState<number | null>(null);
  const [lastRemoved, setLastRemoved] = useState<number | null>(null);
  const [error, setError] = useState<string | null>(null);

  async function run() {
    if (!query.trim()) return;
    setSearching(true);
    try {
      setResults(await invoke<ItemSearchResult[]>("search_items", { query: query.trim() }));
    } finally {
      setSearching(false);
    }
  }

  async function remove(vnum: number) {
    setConfirmVnum(null);
    setRemovingVnum(vnum);
    setError(null);
    try {
      await invoke("remove_single_item", { vnum });
      setResults((prev) => prev.filter((r) => r.vnum !== vnum));
      setLastRemoved(vnum);
    } catch (e) {
      setError(String(e));
    } finally {
      setRemovingVnum(null);
    }
  }

  return (
    <section className="space-y-2 rounded-lg border border-border p-4">
      <h2 className="text-sm font-medium text-muted-foreground">Einzelnes Item entfernen</h2>
      <p className="text-xs text-muted-foreground">
        Für Items, die nicht (mehr) im Verlauf oben stehen. Löscht Datenbankeintrag, Icon und{" "}
        <code>item_list.txt</code>-Eintrag - genau wie „Rückgängig machen" oben, nur für ein einzelnes Item.
      </p>
      <div className="flex gap-2">
        <input
          value={query}
          onChange={(e) => setQuery(e.target.value)}
          onKeyDown={(e) => e.key === "Enter" && run()}
          placeholder="Item nach Name oder VNUM suchen…"
          className="flex-1 rounded-md border border-border bg-background px-2 py-1 text-sm"
        />
        <Button variant="outline" onClick={run} disabled={searching}>
          <Search className="size-4" />
        </Button>
      </div>
      {results.length > 0 && (
        <div className="max-h-40 space-y-1 overflow-y-auto">
          {results.map((r) => (
            <div key={r.vnum} className="flex items-center justify-between rounded-md px-2 py-1 text-sm hover:bg-muted">
              <span>
                {r.name} <span className="text-muted-foreground">#{r.vnum}</span>
              </span>
              <Button
                size="sm"
                variant="destructive"
                disabled={removingVnum === r.vnum}
                onClick={() => setConfirmVnum(r.vnum)}
              >
                <Trash2 className="size-3.5" />
                {removingVnum === r.vnum ? "Entferne…" : "Entfernen"}
              </Button>
            </div>
          ))}
        </div>
      )}
      {lastRemoved !== null && (
        <p className="flex items-center gap-1 text-sm text-green-600">
          <CheckCircle2 className="size-4" /> vnum {lastRemoved} vollständig entfernt.
        </p>
      )}
      {error && <p className="whitespace-pre-wrap text-sm text-destructive">{error}</p>}

      {confirmVnum !== null && (
        <div className="fixed inset-0 flex items-center justify-center bg-black/50">
          <div className="w-[24rem] space-y-3 rounded-lg border border-border bg-card p-4">
            <p className="text-sm font-medium">Item vnum {confirmVnum} endgültig entfernen?</p>
            <p className="text-xs text-muted-foreground">
              Datenbankeintrag, Icon und <code>item_list.txt</code>-Eintrag werden entfernt, <code>item_proto</code>{" "}
              wird neu erzeugt und im Client ersetzt.
            </p>
            <div className="flex justify-end gap-2">
              <Button variant="outline" onClick={() => setConfirmVnum(null)}>
                Abbrechen
              </Button>
              <Button variant="destructive" onClick={() => remove(confirmVnum)}>
                Endgültig entfernen
              </Button>
            </div>
          </div>
        </div>
      )}
    </section>
  );
}

export function ModuleImporter() {
  const [historyVersion, setHistoryVersion] = useState(0);

  return (
    <div className="max-w-3xl space-y-6 pb-10">
      <h1 className="text-2xl font-semibold">Modul-Importer</h1>
      <p className="text-sm text-muted-foreground">
        Erkennt fertige Ausrüstungs-Pakete (Icons + 3D-Modelle, z. B. von Grafikern gelieferte Waffen-/
        Rüstungs-Sets - auch gemischt in einem Ordner) automatisch und legt daraus Items mit Beispielwerten an -
        inklusive Icon, 3D-Modell und aller nötigen Client-Dateien.
      </p>

      <PackageImporter onImported={() => setHistoryVersion((v) => v + 1)} />

      <ImportHistory refreshKey={historyVersion} />
      <QuickRemoveItem />
    </div>
  );
}
