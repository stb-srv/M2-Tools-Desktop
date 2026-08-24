import { useEffect, useMemo, useState } from "react";
import { invoke } from "@tauri-apps/api/core";
import { open } from "@tauri-apps/plugin-dialog";
import { Button } from "@/components/ui/button";
import { RefreshCw, AlertTriangle, CheckCircle2, FolderOpen, Crosshair, HelpCircle, Download, Copy } from "lucide-react";
import { formatRealDropChance, realDropChancePercent } from "./dropChance";
import { findDuplicateItemsInMobs, findDuplicateMobs } from "./duplicates";
import { openManual } from "@/lib/manual";
import { logActivity } from "@/lib/logActivity";
import type { EntityRow } from "@/features/shared/EntityBrowser";
import { exportRowsAsCsv } from "@/lib/exportCsv";
import type { MobDropGroup, MobDropItem } from "./types";
import { clampPercent } from "./components/shared";
import { ServerInfoPopover } from "./components/ServerInfoPopover";
import { GroupSidebar } from "./components/GroupSidebar";
import { MobDetailPanel } from "./components/MobDetailPanel";
import { BulkEditPanel, type BulkEditParams } from "./components/BulkEditPanel";
import { ItemPickerModal } from "./components/ItemPickerModal";
import { ReverseLookupModal } from "./components/ReverseLookupModal";
import { CreateMobModal } from "./components/CreateMobModal";
import { DuplicatesModal } from "./components/DuplicatesModal";

type Source = "server" | "local";

export function MobDropEditor() {
  const [source, setSource] = useState<Source>("server");
  const [groups, setGroups] = useState<MobDropGroup[] | null>(null);
  const [loading, setLoading] = useState(false);
  const [loadError, setLoadError] = useState<string | null>(null);

  const [localPath, setLocalPath] = useState<string | null>(null);
  const [rawRecovery, setRawRecovery] = useState<{ content: string; error: string } | null>(
    null,
  );
  const [rawChecking, setRawChecking] = useState(false);

  const [groupSearch, setGroupSearch] = useState("");
  const [selectedIndex, setSelectedIndex] = useState<number | null>(null);

  const [icons, setIcons] = useState<Record<number, string | null>>({});

  const [itemPicker, setItemPicker] = useState(false);
  const [creating, setCreating] = useState(false);
  const [deleteConfirm, setDeleteConfirm] = useState<number | null>(null);

  const [saving, setSaving] = useState(false);
  const [saveConfirm, setSaveConfirm] = useState(false);
  const [saveError, setSaveError] = useState<string | null>(null);
  const [saveOk, setSaveOk] = useState<string | null>(null);

  const [exporting, setExporting] = useState(false);
  const [exportError, setExportError] = useState<string | null>(null);

  const [reverseLookupOpen, setReverseLookupOpen] = useState(false);
  const [duplicatesOpen, setDuplicatesOpen] = useState(false);

  const duplicateItems = useMemo(() => findDuplicateItemsInMobs(groups ?? []), [groups]);
  const duplicateMobs = useMemo(() => findDuplicateMobs(groups ?? []), [groups]);

  useEffect(() => {
    loadFromServer();
  }, []);

  function switchSource(next: Source) {
    if (next === source) return;
    setSource(next);
    setGroups(null);
    setLoadError(null);
    setSelectedIndex(null);
    setLocalPath(null);
    setRawRecovery(null);
    setSaveOk(null);
    setSaveError(null);
    if (next === "server") loadFromServer();
  }

  async function loadFromServer() {
    setLoading(true);
    setLoadError(null);
    setSelectedIndex(null);
    setSaveOk(null);
    try {
      const result = await invoke<MobDropGroup[]>("read_mob_drop_file");
      setGroups(result);
    } catch (e) {
      setLoadError(String(e));
    } finally {
      setLoading(false);
    }
  }

  async function pickLocalFile() {
    const selected = await open({
      multiple: false,
      title: "mob_drop_item.txt auswählen",
      filters: [{ name: "Textdatei", extensions: ["txt"] }],
    });
    if (typeof selected !== "string") return;
    setLocalPath(selected);
    await loadLocalContent(selected);
  }

  async function loadLocalContent(path: string) {
    setLoading(true);
    setLoadError(null);
    setSelectedIndex(null);
    setSaveOk(null);
    setSaveError(null);
    setGroups(null);
    setRawRecovery(null);
    try {
      const content = await invoke<string>("read_local_text_file", { path });
      try {
        const parsed = await invoke<MobDropGroup[]>("parse_mob_drop_text", { content });
        setGroups(parsed);
      } catch (parseError) {
        setRawRecovery({ content, error: String(parseError) });
      }
    } catch (e) {
      setLoadError(String(e));
    } finally {
      setLoading(false);
    }
  }

  async function retryParseRaw() {
    if (!rawRecovery) return;
    setRawChecking(true);
    try {
      const parsed = await invoke<MobDropGroup[]>("parse_mob_drop_text", {
        content: rawRecovery.content,
      });
      setGroups(parsed);
      setRawRecovery(null);
    } catch (e) {
      setRawRecovery({ content: rawRecovery.content, error: String(e) });
    } finally {
      setRawChecking(false);
    }
  }

  function ensureIcons(vnums: number[]) {
    const missing = [...new Set(vnums)].filter((v) => !(v in icons));
    if (missing.length === 0) return;
    missing.forEach((vnum) => {
      invoke<string | null>("get_item_icon", { vnum })
        .then((dataUrl) => setIcons((prev) => ({ ...prev, [vnum]: dataUrl })))
        .catch(() => setIcons((prev) => ({ ...prev, [vnum]: null })));
    });
  }

  useEffect(() => {
    if (!groups || selectedIndex === null) return;
    ensureIcons(groups[selectedIndex].items.map((i) => i.item_vnum));
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, [groups, selectedIndex]);

  function updateGroup(index: number, patch: Partial<MobDropGroup>) {
    setGroups((prev) => prev!.map((g, i) => (i === index ? { ...g, ...patch } : g)));
  }

  function updateItem(groupIndex: number, itemIndex: number, patch: Partial<MobDropItem>) {
    setGroups((prev) =>
      prev!.map((g, gi) =>
        gi !== groupIndex
          ? g
          : { ...g, items: g.items.map((it, ii) => (ii === itemIndex ? { ...it, ...patch } : it)) },
      ),
    );
  }

  function removeItem(groupIndex: number, itemIndex: number) {
    setGroups((prev) =>
      prev!.map((g, gi) =>
        gi !== groupIndex ? g : { ...g, items: g.items.filter((_, ii) => ii !== itemIndex) },
      ),
    );
  }

  function addItem(item: EntityRow) {
    if (selectedIndex === null) return;
    setGroups((prev) =>
      prev!.map((g, gi) =>
        gi !== selectedIndex
          ? g
          : { ...g, items: [...g.items, { item_vnum: item.vnum, count: 1, percent: 0 }] },
      ),
    );
    setItemPicker(false);
  }

  function handleMobCreated(group: MobDropGroup) {
    setGroups((prev) => [...(prev ?? []), group]);
    setSelectedIndex(groups ? groups.length : 0);
    setCreating(false);
  }

  function navigateToDuplicateGroup(groupIndex: number) {
    setSelectedIndex(groupIndex);
    setDuplicatesOpen(false);
  }

  function deleteDuplicateGroup(groupIndex: number) {
    setDuplicatesOpen(false);
    setDeleteConfirm(groupIndex);
  }

  function confirmDeleteGroup() {
    if (deleteConfirm === null) return;
    setGroups((prev) => prev!.filter((_, i) => i !== deleteConfirm));
    if (selectedIndex === deleteConfirm) setSelectedIndex(null);
    else if (selectedIndex !== null && selectedIndex > deleteConfirm) {
      setSelectedIndex(selectedIndex - 1);
    }
    setDeleteConfirm(null);
  }

  // See dropChance.ts for the full derivation against the server source and
  // the regression this fixes (previously off by a factor of 100).
  const realChance = formatRealDropChance;

  function applyBulkEdit(params: BulkEditParams) {
    setGroups((prev) =>
      prev!.map((g, gi) => {
        if (params.scope === "current" && gi !== selectedIndex) return g;
        return {
          ...g,
          items: g.items.map((item) => {
            if (params.mode === "specific-item") {
              if (!Number.isFinite(params.specificVnum) || item.item_vnum !== params.specificVnum) return item;
              return { ...item, percent: clampPercent(params.specificPercent) };
            }
            let percent = item.percent;
            if (params.mode === "delta") percent = item.percent + params.delta;
            else if (params.mode === "fixed") percent = params.fixed;
            else if (params.mode === "random")
              percent = params.randomMin + Math.random() * (params.randomMax - params.randomMin);
            return { ...item, percent: clampPercent(percent) };
          }),
        };
      }),
    );
  }

  async function saveFile() {
    if (!groups) return;
    setSaving(true);
    setSaveError(null);
    setSaveOk(null);
    setSaveConfirm(false);
    try {
      const backup =
        source === "server"
          ? await invoke<string | null>("write_mob_drop_file", { groups })
          : await invoke<string | null>("write_local_mob_drop_file", { path: localPath, groups });
      setSaveOk(backup ? `Gespeichert. Backup: ${backup}` : "Gespeichert.");
      logActivity(
        "mob-drop-editor",
        "save",
        source === "server"
          ? `mob_drop_item.txt auf dem Server gespeichert (${groups.length} Gruppe(n))`
          : `Lokale mob_drop_item.txt-Kopie gespeichert (${groups.length} Gruppe(n))`,
        "mob_drop_item.txt",
      );
    } catch (e) {
      setSaveError(String(e));
    } finally {
      setSaving(false);
    }
  }

  // Exportiert genau die Mobs, die die aktuelle Suche (Name/VNUM) gerade
  // zeigt - z.B. "Metin" eintippen und alle Metin-Mobs samt ihren Drops als
  // CSV rausschreiben, statt jeden einzeln manuell abzuschreiben.
  async function exportFiltered() {
    setExportError(null);
    setExporting(true);
    try {
      const rows = filteredGroups.flatMap(({ g }) =>
        g.items.length > 0
          ? g.items.map((item) => [
              g.name,
              g.mob_vnum,
              g.drop_type,
              item.item_vnum,
              item.count,
              item.percent,
              realDropChancePercent(item.percent).toFixed(4),
            ])
          : [[g.name, g.mob_vnum, g.drop_type, "", "", "", ""]],
      );
      await exportRowsAsCsv("mob_drop_export.csv", [
        "Gruppe",
        "Mob-VNUM",
        "Drop-Typ",
        "Item-VNUM",
        "Anzahl",
        "Prozent (Datei)",
        "Prozent (real)",
      ], rows);
    } catch (e) {
      setExportError(String(e));
    } finally {
      setExporting(false);
    }
  }

  function reload() {
    if (source === "server") loadFromServer();
    else if (localPath) loadLocalContent(localPath);
  }

  const filteredGroups = (groups ?? [])
    .map((g, index) => ({ g, index }))
    .filter(
      ({ g }) =>
        g.name.toLowerCase().includes(groupSearch.toLowerCase()) ||
        String(g.mob_vnum).includes(groupSearch),
    );

  const selected = selectedIndex !== null ? groups?.[selectedIndex] ?? null : null;

  if (loading && !groups && !rawRecovery) {
    return <p className="text-sm text-muted-foreground">Lade Datei…</p>;
  }

  if (loadError && !groups && !rawRecovery) {
    return (
      <div className="max-w-md space-y-3">
        <h1 className="text-2xl font-semibold">Mob Drop Editor</h1>
        <p className="text-sm text-destructive">{loadError}</p>
        <div className="flex gap-2">
          <Button onClick={reload}>Erneut versuchen</Button>
          {source === "local" && (
            <Button variant="outline" onClick={pickLocalFile}>
              Andere Datei wählen
            </Button>
          )}
        </div>
      </div>
    );
  }

  return (
    <div className="flex h-full flex-col gap-3">
      <div className="flex items-center justify-between">
        <div className="flex items-center gap-2">
          <h1 className="text-2xl font-semibold">Mob Drop Editor</h1>
          <Button variant="ghost" size="icon-sm" title="Hilfe zu diesem Modul" onClick={() => openManual("mob-drop-editor")}>
            <HelpCircle className="size-4" />
          </Button>
          <ServerInfoPopover />
        </div>
        <div className="flex items-center gap-2">
          {saveOk && (
            <span className="flex items-center gap-1 text-sm text-green-600">
              <CheckCircle2 className="size-4" /> {saveOk}
            </span>
          )}
          <Button variant="outline" onClick={() => setReverseLookupOpen(true)} disabled={!groups}>
            <Crosshair className="size-4" />
            Wer droppt…?
          </Button>
          <Button
            variant="outline"
            onClick={exportFiltered}
            disabled={!groups || exporting || filteredGroups.length === 0}
            title="Exportiert die aktuell gefilterte Mob-Liste (siehe Suchfeld) als CSV"
          >
            <Download className="size-4" />
            {exporting ? "Exportiere…" : "Exportieren"}
          </Button>
          <Button
            variant="outline"
            onClick={reload}
            disabled={loading || (source === "local" && !localPath)}
          >
            <RefreshCw className={`size-4 ${loading ? "animate-spin" : ""}`} />
            Neu laden
          </Button>
          <Button onClick={() => setSaveConfirm(true)} disabled={saving || !groups}>
            {saving ? "Speichere…" : "Speichern"}
          </Button>
        </div>
      </div>

      <div className="flex overflow-hidden rounded-md border border-border text-sm w-fit">
        <button
          onClick={() => switchSource("server")}
          className={`px-3 py-1 ${source === "server" ? "bg-primary text-primary-foreground" : ""}`}
        >
          Server (SFTP)
        </button>
        <button
          onClick={() => switchSource("local")}
          className={`px-3 py-1 ${source === "local" ? "bg-primary text-primary-foreground" : ""}`}
        >
          Lokale Datei prüfen/reparieren
        </button>
      </div>

      {saveError && <p className="text-sm text-destructive">{saveError}</p>}
      {exportError && <p className="text-sm text-destructive">{exportError}</p>}

      {(duplicateItems.length > 0 || duplicateMobs.length > 0) && (
        <div className="flex items-center justify-between gap-2 rounded-md border border-amber-500/40 bg-amber-500/10 p-2 text-sm text-amber-700 dark:text-amber-400">
          <span className="flex items-center gap-2">
            <Copy className="size-4 shrink-0" />
            {duplicateItems.length > 0 && `${duplicateItems.length} Mob(s) mit doppelten Items`}
            {duplicateItems.length > 0 && duplicateMobs.length > 0 && ", "}
            {duplicateMobs.length > 0 && `${duplicateMobs.length} doppelte(r) Mob-Eintrag`}
          </span>
          <Button
            size="sm"
            variant="outline"
            onClick={() => {
              ensureIcons(duplicateItems.map((f) => f.itemVnum));
              setDuplicatesOpen(true);
            }}
          >
            Anzeigen
          </Button>
        </div>
      )}

      {source === "local" && localPath && (
        <p className="text-xs text-muted-foreground">
          Datei: <code>{localPath}</code>
        </p>
      )}

      {source === "local" && !groups && !rawRecovery && (
        <div className="flex flex-1 flex-col items-center justify-center gap-3 rounded-lg border border-dashed border-border">
          <p className="text-sm text-muted-foreground">
            Wähle eine lokale <code>mob_drop_item.txt</code>, um sie auf korrekte Syntax zu prüfen.
          </p>
          <Button onClick={pickLocalFile}>
            <FolderOpen className="size-4" />
            Datei wählen…
          </Button>
        </div>
      )}

      {source === "local" && rawRecovery && (
        <div className="flex flex-1 flex-col gap-2">
          <div className="flex items-start gap-2 rounded-md border border-destructive/50 bg-destructive/10 p-2 text-sm text-destructive">
            <AlertTriangle className="mt-0.5 size-4 shrink-0" />
            <span className="whitespace-pre-wrap">{rawRecovery.error}</span>
          </div>
          <p className="text-xs text-muted-foreground">
            Die Syntax stimmt nicht - korrigiere die betroffene Stelle unten (Zeilennummer siehe
            Fehler oben) und prüfe erneut. Erst wenn es fehlerfrei einliest, kannst du im
            strukturierten Editor weiterbearbeiten und speichern.
          </p>
          <textarea
            value={rawRecovery.content}
            onChange={(e) => setRawRecovery({ content: e.target.value, error: rawRecovery.error })}
            spellCheck={false}
            className="min-h-0 flex-1 rounded-md border border-border bg-background p-2 font-mono text-xs"
          />
          <div className="flex gap-2">
            <Button onClick={retryParseRaw} disabled={rawChecking}>
              {rawChecking ? "Prüfe…" : "Erneut prüfen"}
            </Button>
            <Button variant="outline" onClick={pickLocalFile}>
              Andere Datei wählen
            </Button>
          </div>
        </div>
      )}

      {groups && (
      <>
      <div className="flex min-h-0 flex-1 gap-4">
        <GroupSidebar
          filteredGroups={filteredGroups}
          groupSearch={groupSearch}
          onSearchChange={setGroupSearch}
          selectedIndex={selectedIndex}
          onSelect={setSelectedIndex}
          onCreateClick={() => setCreating(true)}
        />

        <div className="min-w-0 flex-1 space-y-4 overflow-y-auto">
          {!selected && (
            <p className="text-sm text-muted-foreground">Wähle links einen Mob aus.</p>
          )}
          {selected && selectedIndex !== null && (
            <MobDetailPanel
              selected={selected}
              selectedIndex={selectedIndex}
              icons={icons}
              realChance={realChance}
              onUpdateGroup={updateGroup}
              onUpdateItem={updateItem}
              onRemoveItem={removeItem}
              onDeleteClick={() => setDeleteConfirm(selectedIndex)}
              onAddItemClick={() => setItemPicker(true)}
            />
          )}

          <BulkEditPanel
            hasGroups={!!groups && groups.length > 0}
            hasSelection={!!selected}
            selectedName={selected?.name}
            realChance={realChance}
            onApply={applyBulkEdit}
          />
        </div>
      </div>

      {itemPicker && (
        <ItemPickerModal
          onClose={() => setItemPicker(false)}
          onPick={addItem}
          icons={icons}
          onRowsChange={(rows) => ensureIcons(rows.map((r) => r.vnum))}
        />
      )}

      {reverseLookupOpen && (
        <ReverseLookupModal
          groups={groups}
          realChance={realChance}
          onSelectMob={(index) => {
            setSelectedIndex(index);
            setReverseLookupOpen(false);
          }}
          onClose={() => setReverseLookupOpen(false)}
        />
      )}

      {creating && <CreateMobModal onClose={() => setCreating(false)} onCreate={handleMobCreated} />}

      {duplicatesOpen && (
        <DuplicatesModal
          duplicateItems={duplicateItems}
          duplicateMobs={duplicateMobs}
          groups={groups}
          icons={icons}
          onNavigateToGroup={navigateToDuplicateGroup}
          onDeleteItemOccurrence={removeItem}
          onDeleteGroup={deleteDuplicateGroup}
          onClose={() => setDuplicatesOpen(false)}
        />
      )}

      {deleteConfirm !== null && groups && (
        <div className="fixed inset-0 flex items-center justify-center bg-black/50">
          <div className="w-80 space-y-3 rounded-lg border border-border bg-card p-4">
            <p className="text-sm">
              Mob <strong>{groups[deleteConfirm]?.name}</strong> wirklich aus der Liste entfernen?
              Wird erst beim Speichern in die Datei übernommen.
            </p>
            <div className="flex justify-end gap-2">
              <Button variant="outline" onClick={() => setDeleteConfirm(null)}>
                Abbrechen
              </Button>
              <Button variant="destructive" onClick={confirmDeleteGroup}>
                Entfernen
              </Button>
            </div>
          </div>
        </div>
      )}

      {saveConfirm && (
        <div className="fixed inset-0 flex items-center justify-center bg-black/50">
          <div className="w-96 space-y-3 rounded-lg border border-border bg-card p-4">
            <p className="flex items-start gap-2 text-sm">
              <AlertTriangle className="mt-0.5 size-4 shrink-0 text-destructive" />
              <span>
                {source === "server"
                  ? "Die Datei auf dem Server wird gesichert und dann komplett mit dem aktuellen Stand überschrieben."
                  : "Die lokale Datei wird gesichert und dann komplett mit dem aktuellen Stand überschrieben."}
              </span>
            </p>
            <div className="flex justify-end gap-2">
              <Button variant="outline" onClick={() => setSaveConfirm(false)}>
                Abbrechen
              </Button>
              <Button onClick={saveFile}>Speichern</Button>
            </div>
          </div>
        </div>
      )}
      </>
      )}
    </div>
  );
}
