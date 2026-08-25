import { useEffect, useRef, useState } from "react";
import { invoke } from "@tauri-apps/api/core";
import { open } from "@tauri-apps/plugin-dialog";
import { logActivity } from "@/lib/logActivity";
import { reportSectionDirty } from "@/store/navigation";
import { useSaveShortcut } from "@/lib/useSaveShortcut";
import { Button } from "@/components/ui/button";
import { AlertTriangle, CheckCircle2, FolderOpen, Image as ImageIcon, Info } from "lucide-react";
import { SUBTYPES_BY_TYPE, WEAR_FLAGS, ITEM_TYPES, VALUE_LABELS_BY_TYPE, VALUE_HINTS } from "@/features/item-editor/itemFlags";
import {
  type ItemProtoInput,
  type StepStatus,
  emptyItem,
  fileNameOf,
  fitToByteLimitWithSuffix,
  findConsecutiveFreeVnums,
  pad5,
  refineCostForStep,
  refineProbForStep,
  scaledValue,
} from "../shared";
import { StepRow, Field } from "./shared";
import { ReferenceItemPicker } from "./ReferenceItemPicker";

// ---- Icon-Item-Importer: Zubehör ohne 3D-Modell (Schuhe, Ketten, Schilder,
// Ohrringe, Armbänder, Verbrauchsgegenstände, ...) ----

function itemTypeOptionLabel(value: number, label: string) {
  return `${label} (${value})`;
}

interface IconItemRow {
  name: string;
  localeName: string;
  description: string;
  summary: string;
  type: number;
  subtype: number;
  wearflag: number;
  values: [number, number, number, number, number, number];
  /** Which of value0-5 grows per Aufwertungs-Stufe - deliberately per-field
   * checkboxes here (not a fixed weapon/armor convention like
   * PackageImporter's chain) since this importer covers arbitrary item
   * types whose value0-5 meaning varies by type *and* subtype (see
   * itemFlags.ts's own VALUE_HINTS comment) - there is no single safe
   * default to bake in generically, so the user marks explicitly. */
  growFlags: [boolean, boolean, boolean, boolean, boolean, boolean];
  refItem: ItemProtoInput | null;
  chainEnabled: boolean;
  chainMaxLevel: number;
  chainGrowthPercent: number;
}

function defaultIconItemRow(suggestedName: string): IconItemRow {
  return {
    name: suggestedName,
    localeName: suggestedName,
    description: "",
    summary: "",
    type: 2,
    subtype: 0,
    wearflag: 0,
    values: [0, 0, 0, 0, 0, 0],
    growFlags: [false, false, false, false, false, false],
    refItem: null,
    chainEnabled: false,
    chainMaxLevel: 9,
    chainGrowthPercent: 10,
  };
}

const VALUE_KEYS = ["value0", "value1", "value2", "value3", "value4", "value5"] as const;

export function IconItemImporter({ onImported }: { onImported: () => void }) {
  const [folderPath, setFolderPath] = useState("");
  const [scanning, setScanning] = useState(false);
  const [scanError, setScanError] = useState<string | null>(null);
  const [iconPaths, setIconPaths] = useState<string[] | null>(null);
  const [selected, setSelected] = useState<Record<string, boolean>>({});
  const [rows, setRows] = useState<Record<string, IconItemRow>>({});
  const [previews, setPreviews] = useState<Record<string, string | null>>({});
  const [vnumRangeStart, setVnumRangeStart] = useState(500000);

  const [running, setRunning] = useState(false);
  const [itemStatus, setItemStatus] = useState<Record<string, StepStatus>>({});
  const [otherSteps, setOtherSteps] = useState<Record<string, StepStatus>>({});
  const [batchError, setBatchError] = useState<string | null>(null);
  const [batchDone, setBatchDone] = useState<number[] | null>(null);
  const [confirm, setConfirm] = useState(false);

  // Same "unsaved work-in-progress" proxy as PackageImporter: a scanned
  // folder with rows configured but not yet imported.
  const dirty = iconPaths !== null && batchDone === null;
  useEffect(() => {
    reportSectionDirty("module-importer", dirty);
    return () => reportSectionDirty("module-importer", false);
  }, [dirty]);
  const createdVnumsRef = useRef<number[]>([]);

  useEffect(() => {
    invoke<string | null>("get_setting", { key: "item_vnum_range_start" })
      .then((v) => setVnumRangeStart(Number(v ?? "500000")))
      .catch(() => {});
  }, []);

  async function pickFolder() {
    const selectedPath = await open({ directory: true, title: "Icon-Ordner wählen" });
    if (typeof selectedPath !== "string") return;
    setFolderPath(selectedPath);
    setIconPaths(null);
    setScanError(null);
  }

  async function scan() {
    if (!folderPath) return;
    setScanning(true);
    setScanError(null);
    try {
      const found = await invoke<string[]>("scan_icon_folder", { path: folderPath });
      setIconPaths(found);
      const sel: Record<string, boolean> = {};
      const newRows: Record<string, IconItemRow> = {};
      for (const p of found) {
        sel[p] = true;
        const stem = fileNameOf(p).replace(/\.[^.]+$/, "");
        const suggested = stem.replace(/[_-]+/g, " ").trim() || stem;
        newRows[p] = defaultIconItemRow(suggested);
      }
      setSelected(sel);
      setRows(newRows);
      setItemStatus({});
      setBatchError(null);
      setBatchDone(null);
      found.forEach((p) => {
        invoke<string>("preview_image_file", { path: p })
          .then((url) => setPreviews((prev) => ({ ...prev, [p]: url })))
          .catch(() => setPreviews((prev) => ({ ...prev, [p]: null })));
      });
    } catch (e) {
      setScanError(String(e));
      setIconPaths(null);
    } finally {
      setScanning(false);
    }
  }

  function updateRow(path: string, patch: Partial<IconItemRow>) {
    setRows((prev) => ({ ...prev, [path]: { ...prev[path], ...patch } }));
  }

  function applyReference(path: string, item: ItemProtoInput) {
    updateRow(path, {
      type: item.type,
      subtype: item.subtype,
      wearflag: item.wearflag,
      values: [item.value0, item.value1, item.value2, item.value3, item.value4, item.value5],
      refItem: item,
    });
  }

  const selectedPaths = (iconPaths ?? []).filter((p) => selected[p]);
  const canImport =
    selectedPaths.length > 0 &&
    selectedPaths.every((p) => rows[p]?.name.trim() && rows[p]?.localeName.trim());

  useSaveShortcut("module-importer", dirty && canImport && !running, () => setConfirm(true));

  async function runImport() {
    setConfirm(false);
    setRunning(true);
    setBatchError(null);
    setBatchDone(null);
    createdVnumsRef.current = [];
    const status: Record<string, StepStatus> = {};
    for (const p of selectedPaths) status[p] = "pending";
    setItemStatus(status);
    setOtherSteps({ setup: "pending", pack: "pending", proto: "pending" });

    try {
      setOtherSteps((prev) => ({ ...prev, setup: "running" }));
      await invoke("validate_item_editor_setup", { requireIconTool: true });
      setOtherSteps((prev) => ({ ...prev, setup: "done" }));

      let rangeStart = vnumRangeStart;

      for (const path of selectedPaths) {
        const row = rows[path];
        setItemStatus((prev) => ({ ...prev, [path]: "running" }));

        const levels = row.chainEnabled ? row.chainMaxLevel + 1 : 1;
        const baseVnum = await findConsecutiveFreeVnums(rangeStart, levels);
        rangeStart = baseVnum + levels;

        await invoke("write_item_icon", { vnum: baseVnum, sourcePath: path });
        const iconRelPath = `icon/item/${pad5(baseVnum)}.tga`;

        const levelVnums: number[] = [];
        for (let level = 0; level < levels; level++) {
          const vnum = baseVnum + level;
          levelVnums.push(vnum);
          const template = row.refItem ?? emptyItem(vnum, row.type);
          const suffix = level > 0 ? ` +${level}` : "";
          const values = row.values.map((v, i) =>
            row.growFlags[i] ? scaledValue(v, level, row.chainGrowthPercent) : v,
          );
          const item: ItemProtoInput = {
            ...template,
            vnum,
            vnum_range: 0,
            name: fitToByteLimitWithSuffix(row.name, level > 0 ? `_${level}` : ""),
            locale_name: fitToByteLimitWithSuffix(row.localeName, suffix),
            type: row.type,
            subtype: row.subtype,
            // Wie bei buildArmorItem: explizit statt vom Referenz-Item
            // geerbt (`...template` oben) - Zubehör ohne 3D-Modell (Schuhe,
            // Ketten, ...) ist im Inventar immer 1 Slot groß, ein als
            // Referenz gewähltes größeres Item (z.B. eine Waffe) darf seine
            // Grid-Höhe nicht durchreichen.
            size: 1,
            wearflag: row.wearflag,
            value0: values[0],
            value1: values[1],
            value2: values[2],
            value3: values[3],
            value4: values[4],
            value5: values[5],
            refine_set: 0,
            refined_vnum: 0,
          };
          await invoke("create_item_proto", { item });
          createdVnumsRef.current.push(vnum);
          if (row.description.trim() || row.summary.trim()) {
            await invoke("write_item_desc", { vnum, description: row.description, summary: row.summary });
          }
          await invoke("write_item_list_entry", { vnum, itemType: row.type, iconRelPath, modelRelPath: null });
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

        setItemStatus((prev) => ({ ...prev, [path]: "done" }));
      }

      setOtherSteps((prev) => ({ ...prev, pack: "running" }));
      await invoke("pack_item_icons");
      setOtherSteps((prev) => ({ ...prev, pack: "done" }));

      setOtherSteps((prev) => ({ ...prev, proto: "running" }));
      const generated = await invoke<string>("regenerate_item_proto");
      await invoke("deploy_item_proto", { generatedProtoPath: generated });
      setOtherSteps((prev) => ({ ...prev, proto: "done" }));

      setBatchDone([...createdVnumsRef.current]);
      try {
        await invoke("record_import_batch", {
          moduleName: folderPath.split(/[\\/]/).pop() ?? "Icon-Items",
          itemType: 2,
          vnums: [...createdVnumsRef.current],
          hadEffects: false,
        });
        onImported();
      } catch {
        // History recording is best-effort.
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
            ? `\n\nAlle ${createdVnumsRef.current.length} bereits angelegten Einträge wurden automatisch zurückgenommen.`
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
        <h2 className="text-sm font-medium text-muted-foreground">1. Icon-Ordner wählen</h2>
        <p className="text-xs text-muted-foreground">
          Für Items ohne eigenes 3D-Modell: Schuhe, Ketten, Schilder, Ohrringe, Armbänder, Verbrauchsgegenstände
          usw. - jede Bilddatei im gewählten Ordner (auch in Unterordnern) wird als eigenes Item angeboten.
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

      {iconPaths && (
        <>
          <section className="space-y-2 rounded-lg border border-border p-4">
            <h2 className="text-sm font-medium text-muted-foreground">
              2. Gefundene Icons ({iconPaths.length})
            </h2>
            <Field label="VNUM-Bereich ab">
              <input
                type="number"
                value={vnumRangeStart}
                onChange={(e) => setVnumRangeStart(Number(e.target.value) || 0)}
                className="w-32 rounded-md border border-border bg-background px-2 py-1 text-sm"
              />
            </Field>
            <div className="space-y-2">
              {iconPaths.map((path) => {
                const row = rows[path];
                if (!row) return null;
                return (
                  <div key={path} className="rounded-md border border-border p-2">
                    <div className="flex items-center gap-3">
                      <input
                        type="checkbox"
                        checked={!!selected[path]}
                        onChange={(e) => setSelected((prev) => ({ ...prev, [path]: e.target.checked }))}
                      />
                      {previews[path] ? (
                        <img src={previews[path]!} alt="" className="size-10 rounded border border-border object-contain" />
                      ) : (
                        <div className="flex size-10 items-center justify-center rounded border border-border bg-muted/40">
                          <ImageIcon className="size-4 text-muted-foreground" />
                        </div>
                      )}
                      <span className="w-40 truncate text-sm" title={path}>
                        {fileNameOf(path)}
                      </span>
                      <input
                        value={row.localeName}
                        onChange={(e) => updateRow(path, { localeName: e.target.value, name: e.target.value })}
                        placeholder="Anzeigename"
                        className="flex-1 rounded-md border border-border bg-background px-2 py-1 text-sm"
                      />
                      <span className="text-xs text-muted-foreground">
                        {ITEM_TYPES.find((t) => t.value === row.type)?.label ?? row.type} ({row.type}) ·{" "}
                        {SUBTYPES_BY_TYPE[row.type]?.find((s) => s.value === row.subtype)?.label ?? `Subtyp ${row.subtype}`}
                      </span>
                    </div>

                    <details className="mt-2">
                      <summary className="cursor-pointer text-xs text-muted-foreground">
                        Typ, Werte, Referenz-Item, Aufwertungs-Kette bearbeiten…
                      </summary>
                      <div className="mt-2 space-y-3 border-t border-border pt-2">
                        <div className="flex flex-wrap gap-3">
                          <Field label="Interner Name (ASCII)">
                            <input
                              value={row.name}
                              onChange={(e) => updateRow(path, { name: e.target.value })}
                              className="w-48 rounded-md border border-border bg-background px-2 py-1 text-sm"
                            />
                          </Field>
                          <Field label="Item-Typ">
                            <select
                              value={row.type}
                              onChange={(e) => updateRow(path, { type: Number(e.target.value), subtype: 0 })}
                              className="rounded-md border border-border bg-background px-2 py-1 text-sm"
                            >
                              {ITEM_TYPES.map((t) => (
                                <option key={t.value} value={t.value}>
                                  {itemTypeOptionLabel(t.value, t.label)}
                                </option>
                              ))}
                            </select>
                          </Field>
                          {SUBTYPES_BY_TYPE[row.type] ? (
                            <Field label="Subtyp">
                              <select
                                value={row.subtype}
                                onChange={(e) => updateRow(path, { subtype: Number(e.target.value) })}
                                className="rounded-md border border-border bg-background px-2 py-1 text-sm"
                              >
                                {SUBTYPES_BY_TYPE[row.type].map((s) => (
                                  <option key={s.value} value={s.value}>
                                    {itemTypeOptionLabel(s.value, s.label)}
                                  </option>
                                ))}
                              </select>
                            </Field>
                          ) : (
                            <Field label="Subtyp (Rohwert, für diesen Typ nicht abgebildet)">
                              <input
                                type="number"
                                value={row.subtype}
                                onChange={(e) => updateRow(path, { subtype: Number(e.target.value) || 0 })}
                                className="w-24 rounded-md border border-border bg-background px-2 py-1 text-sm"
                              />
                            </Field>
                          )}
                          <Field label="Trageort (falls zutreffend)">
                            <select
                              value={row.wearflag}
                              onChange={(e) => updateRow(path, { wearflag: Number(e.target.value) })}
                              className="rounded-md border border-border bg-background px-2 py-1 text-sm"
                            >
                              <option value={0}>— keiner —</option>
                              {WEAR_FLAGS.map((w) => (
                                <option key={w.value} value={w.value}>
                                  {w.label}
                                </option>
                              ))}
                            </select>
                          </Field>
                        </div>

                        <div className="flex flex-wrap gap-3">
                          <Field label="Beschreibung (Tooltip-Text, optional)">
                            <textarea
                              value={row.description}
                              onChange={(e) => updateRow(path, { description: e.target.value })}
                              className="h-16 w-64 rounded-md border border-border bg-background px-2 py-1 text-sm"
                            />
                          </Field>
                          <Field label="Kurzbeschreibung (optional)">
                            <textarea
                              value={row.summary}
                              onChange={(e) => updateRow(path, { summary: e.target.value })}
                              className="h-16 w-48 rounded-md border border-border bg-background px-2 py-1 text-sm"
                            />
                          </Field>
                        </div>
                        {!row.description.trim() && !row.summary.trim() && (
                          <p className="flex items-start gap-2 text-xs text-amber-600">
                            <Info className="mt-0.5 size-3.5 shrink-0" />
                            Ohne Beschreibung bleibt das Item im Client ohne Tooltip-Text.
                          </p>
                        )}

                        <div className="space-y-1">
                          <p className="text-xs text-muted-foreground">
                            Beispielwerte (optional): Referenz-Item übernehmen - Bedeutung von value0-5 hängt vom
                            Typ/Subtyp ab, daher hier bewusst pro Item einzeln.
                          </p>
                          <ReferenceItemPicker onPick={(item) => applyReference(path, item)} />
                          {row.refItem && (
                            <p className="text-xs text-green-600">
                              Übernommen von <strong>{row.refItem.locale_name || row.refItem.name}</strong> (vnum{" "}
                              {row.refItem.vnum}).
                            </p>
                          )}
                        </div>

                        <div className="space-y-1">
                          <p className="text-xs text-muted-foreground">
                            <strong>Wichtig:</strong> Ein Wert ändert sich pro Aufwertungs-Stufe nur, wenn
                            „wächst mit Stufe" angehakt ist - ungehakte Werte bleiben für die komplette Kette
                            exakt so, wie hier eingetragen (auch wenn unten eine Kette aktiviert ist).
                          </p>
                          {VALUE_HINTS[row.type] ? (
                            <p className="flex items-start gap-2 rounded-md bg-muted/40 p-2 text-xs text-muted-foreground">
                              <Info className="mt-0.5 size-3.5 shrink-0" />
                              {VALUE_HINTS[row.type]}
                            </p>
                          ) : (
                            <p className="flex items-start gap-2 rounded-md bg-amber-500/10 p-2 text-xs text-amber-700 dark:text-amber-400">
                              <AlertTriangle className="mt-0.5 size-3.5 shrink-0" />
                              Für diesen Item-Typ ist die Bedeutung von value0-5 (noch) nicht verifiziert -
                              hängt vom Typ und Subtyp ab. Am zuverlässigsten: Werte über „Referenz-Item
                              übernehmen" oben von einem existierenden Item desselben Typs holen, statt zu
                              raten.
                            </p>
                          )}
                          <div className="flex flex-wrap gap-3">
                            {VALUE_KEYS.map((key, i) => (
                              <div key={key} className="flex flex-col gap-1">
                                <label className="text-xs text-muted-foreground" title={key}>
                                  {VALUE_LABELS_BY_TYPE[row.type]?.[i] ?? key}
                                </label>
                                <div className="flex items-center gap-1">
                                  <input
                                    type="number"
                                    value={row.values[i]}
                                    onChange={(e) => {
                                      const values = [...row.values] as IconItemRow["values"];
                                      values[i] = Number(e.target.value) || 0;
                                      updateRow(path, { values });
                                    }}
                                    className="w-20 rounded-md border border-border bg-background px-2 py-1 text-xs"
                                  />
                                  <label className="flex items-center gap-1 text-xs text-muted-foreground">
                                    <input
                                      type="checkbox"
                                      checked={row.growFlags[i]}
                                      onChange={(e) => {
                                        const growFlags = [...row.growFlags] as IconItemRow["growFlags"];
                                        growFlags[i] = e.target.checked;
                                        updateRow(path, { growFlags });
                                      }}
                                    />
                                    wächst mit Stufe
                                  </label>
                                </div>
                              </div>
                            ))}
                          </div>
                        </div>

                        <div className="space-y-1">
                          <label className="flex items-center gap-2 text-sm">
                            <input
                              type="checkbox"
                              checked={row.chainEnabled}
                              onChange={(e) => updateRow(path, { chainEnabled: e.target.checked })}
                            />
                            Eigene Aufwertungs-Kette für dieses Item
                          </label>
                          {row.chainEnabled && (
                            <div className="flex flex-wrap gap-3">
                              <Field label="Maximale Stufe (+N)">
                                <input
                                  type="number"
                                  min={1}
                                  max={30}
                                  value={row.chainMaxLevel}
                                  onChange={(e) => updateRow(path, { chainMaxLevel: Math.max(1, Number(e.target.value) || 1) })}
                                  className="w-24 rounded-md border border-border bg-background px-2 py-1 text-sm"
                                />
                              </Field>
                              <Field label="Wachstum pro Stufe (%)">
                                <input
                                  type="number"
                                  min={0}
                                  value={row.chainGrowthPercent}
                                  onChange={(e) => updateRow(path, { chainGrowthPercent: Math.max(0, Number(e.target.value) || 0) })}
                                  className="w-24 rounded-md border border-border bg-background px-2 py-1 text-sm"
                                />
                              </Field>
                            </div>
                          )}
                        </div>
                      </div>
                    </details>
                  </div>
                );
              })}
            </div>
          </section>

          <section className="space-y-3 rounded-lg border border-border p-4">
            <h2 className="text-sm font-medium text-muted-foreground">3. Importieren</h2>
            <Button disabled={!canImport || running} onClick={() => setConfirm(true)}>
              {running ? "Importiere…" : `${selectedPaths.length} Item(s) importieren`}
            </Button>

            {Object.keys(itemStatus).length > 0 && (
              <div className="space-y-1 text-sm">
                <StepRow label="Voraussetzungen prüfen (Tool-Pfade)" status={otherSteps.setup} />
                {selectedPaths.map((p) => (
                  <StepRow key={p} label={rows[p]?.localeName || fileNameOf(p)} status={itemStatus[p]} />
                ))}
                <StepRow label="icon.epk neu packen" status={otherSteps.pack} />
                <StepRow label="item_proto aus DB erzeugen & in Client einspielen" status={otherSteps.proto} />
              </div>
            )}

            {batchError && <p className="whitespace-pre-wrap text-sm text-destructive">{batchError}</p>}
            {batchDone && (
              <p className="flex items-center gap-1 text-sm text-green-600">
                <CheckCircle2 className="size-4" /> {batchDone.length} Item(s) angelegt: vnums {batchDone.join(", ")}
              </p>
            )}
          </section>
        </>
      )}

      {confirm && (
        <div className="fixed inset-0 flex items-center justify-center bg-black/50">
          <div className="w-[28rem] space-y-3 rounded-lg border border-border bg-card p-4">
            <p className="text-sm font-medium">{selectedPaths.length} Item(s) jetzt anlegen?</p>
            <ul className="list-disc space-y-1 pl-5 text-sm text-muted-foreground">
              <li>Ein Datenbankeintrag + Icon pro gewähltem Bild, kein 3D-Modell</li>
              <li>Items mit aktivierter Aufwertungs-Kette bekommen zusätzliche Stufen (eigenes Icon geteilt)</li>
              <li><code>icon.epk</code> neu gepackt, <code>item_list.txt</code> ergänzt (Backups vorher)</li>
              <li><code>item_proto</code> wird aus der DB neu erzeugt und im Client ersetzt (Backup vorher)</li>
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
