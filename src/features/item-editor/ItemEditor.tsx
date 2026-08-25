import { useEffect, useRef, useState } from "react";
import { invoke } from "@tauri-apps/api/core";
import { listen } from "@tauri-apps/api/event";
import { open } from "@tauri-apps/plugin-dialog";
import { Button } from "@/components/ui/button";
import { Image as ImageIcon, AlertTriangle, CheckCircle2, Copy, Images, HelpCircle, Link2 } from "lucide-react";
import { IconBrowserModal } from "@/features/icon-browser/IconBrowser";
import { EntityBrowser } from "@/features/shared/EntityBrowser";
import { openManual } from "@/lib/manual";
import { logActivity } from "@/lib/logActivity";
import { useNavigationStore, reportSectionDirty } from "@/store/navigation";
import { useSaveShortcut } from "@/lib/useSaveShortcut";
import { ItemUsageModal } from "@/features/shared/ItemUsageModal";
import { ITEM_TYPES, SUBTYPES_BY_TYPE } from "./itemFlags";
import { type ItemProtoInput, type ItemDescEntry, type StepStatus, type Mode, emptyItem } from "./types";
import { Field, StepRow } from "./components/shared";
import { ItemFlagsSection } from "./components/ItemFlagsSection";
import { ConfirmPipelineDialog } from "./components/ConfirmPipelineDialog";
import { ItemPresetsSection } from "./components/ItemPresetsSection";

export function ItemEditor() {
  const [mode, setMode] = useState<Mode>("create");
  const [item, setItem] = useState<ItemProtoInput>(emptyItem(0));
  const [originalItem, setOriginalItem] = useState<ItemProtoInput | null>(null);
  const [vnumTaken, setVnumTaken] = useState<boolean | null>(null);
  const [checkingVnum, setCheckingVnum] = useState(false);
  const [usageVnum, setUsageVnum] = useState<number | null>(null);

  const [iconSourcePath, setIconSourcePath] = useState("");
  const [iconPreview, setIconPreview] = useState<string | null>(null);

  const [refModelVnum, setRefModelVnum] = useState<number | null>(null);
  const [copyModel, setCopyModel] = useState(false);

  const [iconBrowserOpen, setIconBrowserOpen] = useState(false);

  // locale/<lang>/itemdesc.txt (client tooltip text) - no item_proto column
  // at all, so tracked separately from `item` and written in its own
  // pipeline step. See src-tauri/src/itemdesc.rs for the file format.
  const [description, setDescription] = useState("");
  const [summary, setSummary] = useState("");
  const [origDescription, setOrigDescription] = useState("");
  const [origSummary, setOrigSummary] = useState("");

  const [confirmPipeline, setConfirmPipeline] = useState(false);
  const [steps, setSteps] = useState<Record<string, StepStatus>>({});
  const [log, setLog] = useState<string[]>([]);
  const [pipelineError, setPipelineError] = useState<string | null>(null);
  const [createdInDb, setCreatedInDb] = useState(false);
  const [updatedInDb, setUpdatedInDb] = useState(false);
  const [done, setDone] = useState(false);

  const [dirty, setDirty] = useState(false);
  useEffect(() => {
    reportSectionDirty("item-editor", dirty);
    return () => reportSectionDirty("item-editor", false);
  }, [dirty]);

  const logRef = useRef<HTMLPreElement>(null);

  useEffect(() => {
    if (mode !== "create") return;
    invoke<string | null>("get_setting", { key: "item_vnum_range_start" })
      .then((v) => {
        const start = Number(v ?? "500000");
        invoke<number>("next_free_item_vnum", { rangeStart: start })
          .then((vnum) => setItem((prev) => ({ ...prev, vnum })))
          .catch(() => {});
      })
      .catch(() => {});
    // Only intended to run once for the initial suggested vnum on first
    // mount in create mode, not every time the user switches back to it.
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, []);

  // Globale Suche (Strg+Umschalt+F) springt hierher mit einer vnum als
  // pendingSelection - Konsumieren-beim-Lesen verhindert erneutes Auslösen,
  // wenn man später wieder zu diesem Bereich zurückwechselt.
  useEffect(() => {
    const targetRef = useNavigationStore.getState().consumePendingSelection("item-editor");
    if (targetRef) loadForEdit(Number(targetRef));
    // eslint-disable-next-line react-hooks/exhaustive-deps
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

  function set<K extends keyof ItemProtoInput>(key: K, value: ItemProtoInput[K]) {
    setItem((prev) => ({ ...prev, [key]: value }));
    setDirty(true);
  }

  function num(value: string) {
    const n = Number(value);
    return Number.isFinite(n) ? n : 0;
  }

  async function suggestNextVnum() {
    const start = Number(
      (await invoke<string | null>("get_setting", { key: "item_vnum_range_start" }).catch(
        () => null,
      )) ?? "500000",
    );
    const vnum = await invoke<number>("next_free_item_vnum", { rangeStart: start });
    set("vnum", vnum);
    setVnumTaken(false);
  }

  async function checkVnum(vnum: number) {
    setCheckingVnum(true);
    try {
      const exists = await invoke<boolean>("item_vnum_exists", { vnum });
      setVnumTaken(exists);
    } catch {
      setVnumTaken(null);
    } finally {
      setCheckingVnum(false);
    }
  }

  async function startCreateMode() {
    setMode("create");
    setOriginalItem(null);
    setIconSourcePath("");
    setIconPreview(null);
    setRefModelVnum(null);
    setCopyModel(false);
    setDone(false);
    setPipelineError(null);
    setSteps({});
    const start = Number(
      (await invoke<string | null>("get_setting", { key: "item_vnum_range_start" }).catch(
        () => null,
      )) ?? "500000",
    );
    const vnum = await invoke<number>("next_free_item_vnum", { rangeStart: start }).catch(
      () => 0,
    );
    setItem(emptyItem(vnum));
    setVnumTaken(vnum > 0 ? false : null);
    setDescription("");
    setSummary("");
    setOrigDescription("");
    setOrigSummary("");
    setDirty(false);
  }

  async function loadForEdit(vnum: number) {
    const full = await invoke<ItemProtoInput | null>("get_item_proto", { vnum });
    if (!full) return;
    setMode("edit");
    setItem(full);
    setOriginalItem(full);
    setVnumTaken(null);
    setIconSourcePath("");
    setDone(false);
    setPipelineError(null);
    setSteps({});
    const preview = await invoke<string | null>("get_item_icon", { vnum }).catch(() => null);
    setIconPreview(preview);
    const desc = await invoke<ItemDescEntry | null>("get_item_desc", { vnum }).catch(() => null);
    setDescription(desc?.description ?? "");
    setSummary(desc?.summary ?? "");
    setOrigDescription(desc?.description ?? "");
    setOrigSummary(desc?.summary ?? "");
    setDirty(false);
  }

  async function pickIcon() {
    const selected = await open({
      multiple: false,
      title: "Icon auswählen",
      filters: [{ name: "Bild", extensions: ["tga", "png", "jpg", "jpeg", "bmp"] }],
    });
    if (typeof selected === "string") {
      setIconSourcePath(selected);
      setIconPreview(null);
      setDirty(true);
    }
  }

  async function duplicateAsNew() {
    const start = Number(
      (await invoke<string | null>("get_setting", { key: "item_vnum_range_start" }).catch(
        () => null,
      )) ?? "500000",
    );
    const vnum = await invoke<number>("next_free_item_vnum", { rangeStart: start }).catch(
      () => 0,
    );
    setItem((prev) => ({ ...prev, vnum }));
    setMode("create");
    setOriginalItem(null);
    setIconSourcePath("");
    setIconPreview(null);
    setRefModelVnum(null);
    setCopyModel(false);
    setDone(false);
    setPipelineError(null);
    setSteps({});
    setVnumTaken(vnum > 0 ? false : null);
    // Description/summary text is kept as a starting draft (matches how
    // every other field is duplicated), but the "original" baseline resets
    // to empty since the new vnum has no itemdesc.txt row of its own yet.
    setOrigDescription("");
    setOrigSummary("");
    // A pre-filled but never-yet-saved draft - counts as unsaved work.
    setDirty(true);
  }

  async function loadReference(vnum: number) {
    const full = await invoke<ItemProtoInput | null>("get_item_proto", { vnum });
    if (full) {
      setItem((prev) => ({ ...full, vnum: prev.vnum }));
    }
    // Weapon 3D models are plain vnum-named files client-side (not anything
    // in item_proto), so "reuse this item's model" only makes sense for
    // type 1 (weapon) - armor models live in the much larger per-race
    // pc_* character trees and aren't supported here.
    setRefModelVnum(full && full.type === 1 ? vnum : null);
    setCopyModel(false);
    setDirty(true);
  }

  // A preset has no real vnum/model on disk to reuse (unlike a reference
  // item), so it only ever prefills field values - same "keep the current
  // vnum" behavior as loadReference above.
  function loadPreset(preset: ItemProtoInput) {
    setItem((prev) => ({ ...preset, vnum: prev.vnum }));
    setRefModelVnum(null);
    setCopyModel(false);
    setDirty(true);
  }

  function toggleFlag(key: "wearflag" | "antiflag" | "immuneflag" | "flag", bit: number) {
    set(key, (item[key] & bit) !== 0 ? item[key] & ~bit : item[key] | bit);
  }

  const hasNewIcon = !!iconSourcePath;
  const hasModelCopy = copyModel && refModelVnum !== null && item.type === 1;
  const hasDescChange = description !== origDescription || summary !== origSummary;

  const canCreate =
    item.vnum > 0 &&
    item.name.trim().length > 0 &&
    item.locale_name.trim().length > 0 &&
    (mode === "edit" || (hasNewIcon && vnumTaken !== true));

  useSaveShortcut("item-editor", dirty && canCreate && Object.keys(steps).length === 0, () =>
    setConfirmPipeline(true),
  );

  async function runStep(key: string, fn: () => Promise<void>) {
    setSteps((prev) => ({ ...prev, [key]: "running" }));
    try {
      await fn();
      setSteps((prev) => ({ ...prev, [key]: "done" }));
    } catch (e) {
      setSteps((prev) => ({ ...prev, [key]: "error" }));
      throw e;
    }
  }

  async function runPipeline() {
    setConfirmPipeline(false);
    setPipelineError(null);
    setLog([]);
    setSteps({});
    setDone(false);

    let dbCreated = false;
    let dbUpdated = false;
    try {
      // Checked first, before anything destructive (including the DB
      // write): a wrong tool path must fail loudly here, not leave a
      // half-finished item behind after the DB step already ran.
      await runStep("setup", async () => {
        await invoke("validate_item_editor_setup", {
          requireIconTool: hasNewIcon || hasModelCopy,
        });
      });
      await runStep("db", async () => {
        if (mode === "create") {
          await invoke("create_item_proto", { item });
          dbCreated = true;
          setCreatedInDb(true);
        } else {
          await invoke("update_item_proto", { item });
          dbUpdated = true;
          setUpdatedInDb(true);
        }
      });
      if (hasDescChange) {
        await runStep("desc", async () => {
          await invoke("write_item_desc", { vnum: item.vnum, description, summary });
          setOrigDescription(description);
          setOrigSummary(summary);
        });
      }
      if (hasNewIcon) {
        await runStep("icon", async () => {
          await invoke("write_item_icon", { vnum: item.vnum, sourcePath: iconSourcePath });
          const preview = await invoke<string | null>("get_item_icon", {
            vnum: item.vnum,
          }).catch(() => null);
          setIconPreview(preview);
        });
        await runStep("pack", async () => {
          await invoke("pack_item_icons");
        });
      }
      if (hasModelCopy) {
        await runStep("model", async () => {
          await invoke("write_item_model", { vnum: item.vnum, sourceVnum: refModelVnum });
        });
        await runStep("packModel", async () => {
          await invoke("pack_item_models");
        });
      }
      if (hasNewIcon) {
        // The client doesn't guess icon/model filenames from the vnum like
        // this tool's own preview does - it needs an explicit row in every
        // locale/<lang>/item_list.txt or the icon (and model) silently
        // fail to render even though the .tga/.gr2/.epk files are correct.
        await runStep("itemList", async () => {
          await invoke("write_item_list_entry", {
            vnum: item.vnum,
            itemType: item.type,
            iconRelPath: `icon/item/${String(item.vnum).padStart(5, "0")}.tga`,
            modelRelPath: hasModelCopy
              ? `d:/ymir work/item/weapon/${String(item.vnum).padStart(5, "0")}.gr2`
              : null,
          });
        });
      }
      await runStep("proto", async () => {
        const generated = await invoke<string>("regenerate_item_proto");
        await invoke("deploy_item_proto", { generatedProtoPath: generated });
      });
      setDone(true);
      setDirty(false);
      if (mode === "edit") setOriginalItem(item);
      logActivity(
        "item-editor",
        mode === "create" ? "create" : "update",
        `Item ${item.vnum} ('${item.locale_name || item.name}') ${mode === "create" ? "angelegt" : "aktualisiert"}`,
        "item",
        String(item.vnum),
      );
    } catch (e) {
      let message = String(e);
      // A failure after the DB write would otherwise leave a half-finished
      // item behind (create: occupies the vnum forever; edit: DB and client
      // proto disagree) - undo it automatically rather than relying on the
      // user noticing a button.
      if (dbCreated) {
        try {
          await invoke("delete_item_proto", { vnum: item.vnum });
          setCreatedInDb(false);
          message += "\n\nDatenbankeintrag wurde automatisch zurückgenommen.";
        } catch (rollbackError) {
          message += `\n\nDatenbankeintrag konnte NICHT automatisch zurückgenommen werden: ${String(rollbackError)}. Bitte manuell prüfen (vnum ${item.vnum}).`;
        }
      } else if (dbUpdated && originalItem) {
        try {
          await invoke("update_item_proto", { item: originalItem });
          setUpdatedInDb(false);
          message += "\n\nÄnderungen wurden automatisch zurückgenommen (alter Stand wiederhergestellt).";
        } catch (rollbackError) {
          message += `\n\nÄnderungen konnten NICHT automatisch zurückgenommen werden: ${String(rollbackError)}. Bitte vnum ${item.vnum} manuell im DB Explorer prüfen.`;
        }
      }
      setPipelineError(message);
    }
  }

  async function rollbackDb() {
    try {
      if (mode === "create") {
        await invoke("delete_item_proto", { vnum: item.vnum });
        setCreatedInDb(false);
      } else if (originalItem) {
        await invoke("update_item_proto", { item: originalItem });
        setUpdatedInDb(false);
      }
      setSteps((prev) => ({ ...prev, db: "pending" }));
    } catch (e) {
      setPipelineError(String(e));
    }
  }

  return (
    <div className="max-w-3xl space-y-6 pb-10">
      <div className="flex items-center gap-2">
        <h1 className="text-2xl font-semibold">Item Editor</h1>
        <Button variant="ghost" size="icon-sm" title="Hilfe zu diesem Modul" onClick={() => openManual("item-editor")}>
          <HelpCircle className="size-4" />
        </Button>
      </div>
      <p className="text-sm text-muted-foreground">
        Legt neue Items an oder bearbeitet bestehende. Waffen können beim Anlegen das 3D-Modell eines
        bestehenden Waffen-Items übernehmen (Referenz-Item übernehmen unten). Komplett neue,
        selbst erstellte 3D-Modelle sowie Rüstungen mit übernommenem Modell werden noch nicht
        unterstützt.
      </p>

      {/* Modus */}
      <section className="space-y-2 rounded-lg border border-border p-4">
        <div className="flex overflow-hidden rounded-md border border-border text-sm w-fit">
          <button
            onClick={startCreateMode}
            className={`px-3 py-1 ${mode === "create" ? "bg-primary text-primary-foreground" : ""}`}
          >
            Neues Item
          </button>
          <button
            onClick={() => setMode("edit")}
            className={`px-3 py-1 ${mode === "edit" ? "bg-primary text-primary-foreground" : ""}`}
          >
            Bestehendes Item bearbeiten
          </button>
        </div>

        {mode === "edit" && !originalItem && (
          <EntityBrowser kind="item" pickLabel="Laden" onPick={(r) => loadForEdit(r.vnum)} />
        )}

        {mode === "edit" && originalItem && (
          <div className="flex items-center justify-between gap-2">
            <p className="text-sm text-muted-foreground">
              Bearbeite <strong>{item.locale_name || item.name}</strong> (vnum {item.vnum}).{" "}
              <button className="underline" onClick={() => setOriginalItem(null)}>
                Anderes Item wählen
              </button>
            </p>
            <div className="flex gap-2">
              <Button variant="outline" size="sm" onClick={() => setUsageVnum(item.vnum)}>
                <Link2 className="size-3.5" />
                Verwendung prüfen
              </Button>
              <Button variant="outline" size="sm" onClick={duplicateAsNew}>
                <Copy className="size-3.5" />
                Als neues Item duplizieren
              </Button>
            </div>
          </div>
        )}
        {usageVnum !== null && <ItemUsageModal vnum={usageVnum} onClose={() => setUsageVnum(null)} />}
      </section>

      {mode === "create" || originalItem ? (
        <>
      {/* Basisdaten */}
      <section className="space-y-3 rounded-lg border border-border p-4">
        <h2 className="text-sm font-medium text-muted-foreground">Basisdaten</h2>

        <div className="flex flex-wrap gap-3">
          <Field label="VNUM">
            <div className="flex items-center gap-1">
              <input
                type="number"
                value={item.vnum || ""}
                disabled={mode === "edit"}
                onChange={(e) => {
                  set("vnum", num(e.target.value));
                  setVnumTaken(null);
                }}
                onBlur={() => item.vnum > 0 && checkVnum(item.vnum)}
                className="w-28 rounded-md border border-border bg-background px-2 py-1 text-sm disabled:opacity-60"
              />
              {mode === "create" && (
                <Button variant="outline" size="sm" onClick={suggestNextVnum}>
                  Nächste freie
                </Button>
              )}
            </div>
            {mode === "create" && checkingVnum && (
              <p className="text-xs text-muted-foreground">Prüfe…</p>
            )}
            {mode === "create" && vnumTaken === true && (
              <p className="flex items-center gap-1 text-xs text-destructive">
                <AlertTriangle className="size-3.5" /> VNUM bereits vergeben
              </p>
            )}
            {mode === "create" && vnumTaken === false && (
              <p className="flex items-center gap-1 text-xs text-green-600">
                <CheckCircle2 className="size-3.5" /> frei
              </p>
            )}
          </Field>

          <Field label="Typ">
            <select
              value={item.type}
              onChange={(e) => set("type", num(e.target.value))}
              className="rounded-md border border-border bg-background px-2 py-1 text-sm"
            >
              {ITEM_TYPES.map((t) => (
                <option key={t.value} value={t.value}>
                  {t.label} ({t.value})
                </option>
              ))}
            </select>
          </Field>

          <Field label="Subtyp">
            {SUBTYPES_BY_TYPE[item.type] ? (
              <select
                value={item.subtype}
                onChange={(e) => set("subtype", num(e.target.value))}
                className="w-40 rounded-md border border-border bg-background px-2 py-1 text-sm"
              >
                {SUBTYPES_BY_TYPE[item.type].map((t) => (
                  <option key={t.value} value={t.value}>
                    {t.label} ({t.value})
                  </option>
                ))}
              </select>
            ) : (
              <input
                type="number"
                value={item.subtype}
                onChange={(e) => set("subtype", num(e.target.value))}
                className="w-20 rounded-md border border-border bg-background px-2 py-1 text-sm"
              />
            )}
          </Field>

          <Field label="Größe (1-3)">
            <input
              type="number"
              min={1}
              max={3}
              value={item.size}
              onChange={(e) => set("size", num(e.target.value))}
              className="w-20 rounded-md border border-border bg-background px-2 py-1 text-sm"
            />
          </Field>

          <Field label="Gewicht">
            <input
              type="number"
              value={item.weight}
              onChange={(e) => set("weight", num(e.target.value))}
              className="w-20 rounded-md border border-border bg-background px-2 py-1 text-sm"
            />
          </Field>
        </div>

        <div className="flex flex-wrap gap-3">
          <Field label="Name (intern, ASCII)">
            <input
              value={item.name}
              onChange={(e) => set("name", e.target.value)}
              className="w-56 rounded-md border border-border bg-background px-2 py-1 text-sm"
            />
          </Field>
          <Field label="Anzeigename">
            <input
              value={item.locale_name}
              onChange={(e) => set("locale_name", e.target.value)}
              className="w-56 rounded-md border border-border bg-background px-2 py-1 text-sm"
            />
          </Field>
        </div>

        <div className="flex flex-wrap gap-3">
          <Field label="Beschreibung (Tooltip-Text)">
            <textarea
              value={description}
              onChange={(e) => {
                setDescription(e.target.value);
                setDirty(true);
              }}
              className="h-16 w-72 rounded-md border border-border bg-background px-2 py-1 text-sm"
            />
          </Field>
          <Field label="Kurzbeschreibung">
            <textarea
              value={summary}
              onChange={(e) => {
                setSummary(e.target.value);
                setDirty(true);
              }}
              className="h-16 w-56 rounded-md border border-border bg-background px-2 py-1 text-sm"
            />
          </Field>
        </div>
        <p className="text-xs text-muted-foreground">
          Rein clientseitiger Tooltip-Text (<code>locale/&lt;lang&gt;/itemdesc.txt</code>) - keine
          Datenbank-Spalte, keine Server-Beteiligung. Wird beim Speichern lokal in die Client-Datei
          geschrieben (mit Backup), nicht über SFTP - ein Client-Neustart/-Relog reicht, um die neue
          Beschreibung zu sehen, kein Server-Neustart nötig.
        </p>

        <div className="flex flex-wrap gap-3">
          <Field label="Kaufpreis (NPC → Spieler)">
            <input
              type="number"
              value={item.gold}
              onChange={(e) => set("gold", num(e.target.value))}
              className="w-28 rounded-md border border-border bg-background px-2 py-1 text-sm"
            />
          </Field>
          <Field label="Verkaufspreis (Spieler → NPC)">
            <input
              type="number"
              value={item.shop_buy_price}
              onChange={(e) => set("shop_buy_price", num(e.target.value))}
              className="w-28 rounded-md border border-border bg-background px-2 py-1 text-sm"
            />
          </Field>
        </div>
      </section>

      {/* Icon */}
      <section className="space-y-2 rounded-lg border border-border p-4">
        <h2 className="text-sm font-medium text-muted-foreground">Icon</h2>
        <div className="flex items-center gap-3">
          <div className="flex size-12 items-center justify-center rounded-md border border-border bg-muted/40">
            {iconPreview ? (
              <img src={iconPreview} alt="" className="size-10 object-contain [image-rendering:pixelated]" />
            ) : (
              <ImageIcon className="size-5 text-muted-foreground" />
            )}
          </div>
          <Button variant="outline" onClick={pickIcon}>
            Bilddatei wählen…
          </Button>
          <Button variant="outline" onClick={() => setIconBrowserOpen(true)}>
            <Images className="size-4" />
            Aus Icon-Browser wählen…
          </Button>
          <span className="truncate text-sm text-muted-foreground">
            {iconSourcePath || (mode === "edit" ? "Unverändert" : "Keine Datei gewählt")}
          </span>
        </div>
        {mode === "edit" && !iconSourcePath && (
          <p className="text-xs text-green-600">
            {iconPreview
              ? "Vorhandenes Icon wird beibehalten - nichts wird beim Speichern neu geschrieben oder gepackt."
              : "Kein Icon gefunden. Nur auswählen, falls du wirklich ein neues setzen willst."}
          </p>
        )}
        {(mode === "create" || iconSourcePath) && (
          <p className="text-xs text-muted-foreground">
            Wird als <code>{`${String(item.vnum).padStart(5, "0")}.tga`}</code> in{" "}
            <code>pack/icon/icon/item</code> gespeichert und <code>icon.epk</code> neu gepackt.
          </p>
        )}
      </section>

      {/* Vorlagen */}
      {mode === "create" && <ItemPresetsSection currentItem={item} onLoad={loadPreset} />}

      {/* Referenz-Item */}
      {mode === "create" && (
        <section className="space-y-2 rounded-lg border border-border p-4">
          <h2 className="text-sm font-medium text-muted-foreground">
            Referenz-Item übernehmen (optional)
          </h2>
          <p className="text-xs text-muted-foreground">
            Übernimmt alle Werte (Typ, Flags, Werte) eines bestehenden Items als Ausgangsbasis — die
            VNUM bleibt unverändert.
          </p>
          <EntityBrowser kind="item" pickLabel="Übernehmen" onPick={(r) => loadReference(r.vnum)} />
          {refModelVnum !== null && item.type === 1 && (
            <label className="flex items-center gap-2 text-sm">
              <input
                type="checkbox"
                checked={copyModel}
                onChange={(e) => {
                  setCopyModel(e.target.checked);
                  setDirty(true);
                }}
              />
              3D-Modell von vnum {refModelVnum} übernehmen (Waffe sieht dann identisch aus)
            </label>
          )}
        </section>
      )}

      {/* Flags & Werte */}
      <ItemFlagsSection item={item} set={set} toggleFlag={toggleFlag} />

      {/* Zusammenfassung / Ausführen */}
      <section className="space-y-3 rounded-lg border border-border p-4">
        <h2 className="text-sm font-medium text-muted-foreground">
          {mode === "create" ? "Anlegen" : "Speichern"}
        </h2>
        <Button disabled={!canCreate || Object.keys(steps).length > 0} onClick={() => setConfirmPipeline(true)}>
          {mode === "create" ? "Item anlegen" : "Änderungen speichern"}
        </Button>

        {Object.keys(steps).length > 0 && (
          <div className="space-y-1 text-sm">
            <StepRow label="Voraussetzungen prüfen (Tool-Pfade)" status={steps.setup} />
            <StepRow label="Datenbankeintrag" status={steps.db} />
            {hasDescChange && <StepRow label="Beschreibung schreiben (itemdesc.txt)" status={steps.desc} />}
            {hasNewIcon && <StepRow label="Icon schreiben" status={steps.icon} />}
            {hasNewIcon && <StepRow label="icon.epk neu packen" status={steps.pack} />}
            {hasModelCopy && <StepRow label="3D-Modell kopieren" status={steps.model} />}
            {hasModelCopy && <StepRow label="item.epk neu packen" status={steps.packModel} />}
            {hasNewIcon && (
              <StepRow label="item_list.txt aktualisieren (Icon-/Modell-Zuordnung)" status={steps.itemList} />
            )}
            <StepRow label="item_proto aus DB erzeugen & in Client einspielen" status={steps.proto} />
          </div>
        )}

        {pipelineError && (
          <div className="space-y-2">
            <p className="whitespace-pre-wrap text-sm text-destructive">{pipelineError}</p>
            {((mode === "create" && createdInDb) || (mode === "edit" && updatedInDb)) && !done && (
              <Button variant="destructive" size="sm" onClick={rollbackDb}>
                {mode === "create" ? "Datenbankeintrag zurücknehmen" : "Änderungen zurücknehmen"}
              </Button>
            )}
          </div>
        )}

        {done && (
          <p className="flex items-center gap-1 text-sm text-green-600">
            <CheckCircle2 className="size-4" />{" "}
            {mode === "create"
              ? `Item ${item.vnum} wurde vollständig angelegt.`
              : `Item ${item.vnum} wurde gespeichert.`}
          </p>
        )}

        {log.length > 0 && (
          <pre
            ref={logRef}
            className="max-h-48 overflow-y-auto rounded-md bg-muted/40 p-2 text-xs"
          >
            {log.join("\n")}
          </pre>
        )}
      </section>

      {confirmPipeline && (
        <ConfirmPipelineDialog
          mode={mode}
          itemVnum={item.vnum}
          hasNewIcon={hasNewIcon}
          hasModelCopy={hasModelCopy}
          refModelVnum={refModelVnum}
          onCancel={() => setConfirmPipeline(false)}
          onConfirm={runPipeline}
        />
      )}
      </>
      ) : null}

      {iconBrowserOpen && (
        <IconBrowserModal
          onPick={(path) => {
            setIconSourcePath(path);
            setIconPreview(null);
            setDirty(true);
          }}
          onClose={() => setIconBrowserOpen(false)}
        />
      )}
    </div>
  );
}
