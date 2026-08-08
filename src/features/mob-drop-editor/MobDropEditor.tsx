import { useEffect, useMemo, useState } from "react";
import { invoke } from "@tauri-apps/api/core";
import { open } from "@tauri-apps/plugin-dialog";
import { runAsyncAction } from "@/lib/asyncAction";
import { Button } from "@/components/ui/button";
import {
  Search,
  Plus,
  Trash2,
  X,
  RefreshCw,
  AlertTriangle,
  CheckCircle2,
  Info,
  FolderOpen,
  Crosshair,
  HelpCircle,
} from "lucide-react";
import { SERVER_NOTES } from "./serverNotes";
import { formatRealDropChance } from "./dropChance";
import { openManual } from "@/lib/manual";

interface MobDropItem {
  item_vnum: number;
  count: number;
  percent: number;
}

interface MobDropGroup {
  name: string;
  mob_vnum: number;
  drop_type: string;
  items: MobDropItem[];
}

interface ItemSearchResult {
  vnum: number;
  name: string;
}

type BulkMode = "delta" | "fixed" | "random" | "specific-item";
type BulkScope = "global" | "current";
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
  const [itemQuery, setItemQuery] = useState("");
  const [itemResults, setItemResults] = useState<ItemSearchResult[]>([]);
  const [itemSearching, setItemSearching] = useState(false);
  const [itemSearchError, setItemSearchError] = useState<string | null>(null);

  const [creating, setCreating] = useState(false);
  const [newName, setNewName] = useState("");
  const [newNamePreview, setNewNamePreview] = useState("");
  const [newMobVnum, setNewMobVnum] = useState("");
  const [newType, setNewType] = useState("drop");

  const [deleteConfirm, setDeleteConfirm] = useState<number | null>(null);

  const [bulkScope, setBulkScope] = useState<BulkScope>("current");
  const [bulkMode, setBulkMode] = useState<BulkMode>("delta");
  const [deltaValue, setDeltaValue] = useState(0);
  const [fixedValue, setFixedValue] = useState(0);
  const [randomMin, setRandomMin] = useState(0);
  const [randomMax, setRandomMax] = useState(30);
  const [specificItemVnum, setSpecificItemVnum] = useState("");
  const [specificItemPercent, setSpecificItemPercent] = useState(0);
  const [bulkConfirm, setBulkConfirm] = useState(false);

  const [saving, setSaving] = useState(false);
  const [saveConfirm, setSaveConfirm] = useState(false);
  const [saveError, setSaveError] = useState<string | null>(null);
  const [saveOk, setSaveOk] = useState<string | null>(null);

  const [serverInfoOpen, setServerInfoOpen] = useState(false);

  // Reverse-Suche "wer droppt Item X" - rein clientseitig, da `groups` beim
  // Öffnen bereits die komplette Datei enthält (ein einzelnes File, kein
  // Mob-für-Mob-Nachladen nötig).
  const [reverseLookupOpen, setReverseLookupOpen] = useState(false);
  const [reverseQuery, setReverseQuery] = useState("");
  const [reverseResults, setReverseResults] = useState<ItemSearchResult[]>([]);
  const [reverseSearching, setReverseSearching] = useState(false);
  const [reverseSearchError, setReverseSearchError] = useState<string | null>(null);
  const [reverseSelectedItem, setReverseSelectedItem] = useState<ItemSearchResult | null>(null);

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

  useEffect(() => {
    if (!creating) return;
    if (!newName.trim()) {
      setNewNamePreview("");
      return;
    }
    invoke<string>("sanitize_mob_drop_group_name", { name: newName })
      .then(setNewNamePreview)
      .catch(() => setNewNamePreview(""));
  }, [newName, creating]);

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

  useEffect(() => {
    ensureIcons(itemResults.map((r) => r.vnum));
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, [itemResults]);

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

  async function runReverseSearch() {
    if (!reverseQuery.trim()) return;
    await runAsyncAction(
      () => invoke<ItemSearchResult[]>("search_items", { query: reverseQuery.trim() }),
      {
        onStart: () => {
          setReverseSearching(true);
          setReverseSearchError(null);
        },
        onSuccess: setReverseResults,
        onError: setReverseSearchError,
        onFinally: () => setReverseSearching(false),
      },
    );
  }

  function closeReverseLookup() {
    setReverseLookupOpen(false);
    setReverseQuery("");
    setReverseResults([]);
    setReverseSelectedItem(null);
    setReverseSearchError(null);
  }

  const reverseMatches = useMemo(() => {
    if (!groups || !reverseSelectedItem) return [];
    return groups
      .map((g, index) => ({ g, index, drop: g.items.find((it) => it.item_vnum === reverseSelectedItem.vnum) }))
      .filter((m): m is { g: MobDropGroup; index: number; drop: MobDropItem } => !!m.drop);
  }, [groups, reverseSelectedItem]);

  async function runItemSearch() {
    if (!itemQuery.trim()) return;
    await runAsyncAction(
      () => invoke<ItemSearchResult[]>("search_items", { query: itemQuery.trim() }),
      {
        onStart: () => {
          setItemSearching(true);
          setItemSearchError(null);
        },
        onSuccess: setItemResults,
        onError: setItemSearchError,
        onFinally: () => setItemSearching(false),
      },
    );
  }

  function addItem(item: ItemSearchResult) {
    if (selectedIndex === null) return;
    setGroups((prev) =>
      prev!.map((g, gi) =>
        gi !== selectedIndex
          ? g
          : { ...g, items: [...g.items, { item_vnum: item.vnum, count: 1, percent: 0 }] },
      ),
    );
    setItemPicker(false);
    setItemQuery("");
    setItemResults([]);
  }

  function submitNewGroup() {
    const mobVnum = Number(newMobVnum);
    if (!newNamePreview || !Number.isFinite(mobVnum) || mobVnum <= 0) return;
    setGroups((prev) => [
      ...(prev ?? []),
      { name: newNamePreview, mob_vnum: mobVnum, drop_type: newType.trim() || "drop", items: [] },
    ]);
    setSelectedIndex(groups ? groups.length : 0);
    setCreating(false);
    setNewName("");
    setNewNamePreview("");
    setNewMobVnum("");
    setNewType("drop");
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

  function clampPercent(value: number) {
    return Math.max(0, Math.min(100, Math.round(value * 10000) / 10000));
  }

  // See dropChance.ts for the full derivation against the server source and
  // the regression this fixes (previously off by a factor of 100).
  const realChance = formatRealDropChance;

  function applyBulkEdit() {
    const specificVnum = Number(specificItemVnum);
    setGroups((prev) =>
      prev!.map((g, gi) => {
        if (bulkScope === "current" && gi !== selectedIndex) return g;
        return {
          ...g,
          items: g.items.map((item) => {
            if (bulkMode === "specific-item") {
              if (!Number.isFinite(specificVnum) || item.item_vnum !== specificVnum) return item;
              return { ...item, percent: clampPercent(specificItemPercent) };
            }
            let percent = item.percent;
            if (bulkMode === "delta") percent = item.percent + deltaValue;
            else if (bulkMode === "fixed") percent = fixedValue;
            else if (bulkMode === "random") percent = randomMin + Math.random() * (randomMax - randomMin);
            return { ...item, percent: clampPercent(percent) };
          }),
        };
      }),
    );
    setBulkConfirm(false);
  }

  async function saveFile() {
    if (!groups) return;
    setSaving(true);
    setSaveError(null);
    setSaveConfirm(false);
    try {
      const backup =
        source === "server"
          ? await invoke<string | null>("write_mob_drop_file", { groups })
          : await invoke<string | null>("write_local_mob_drop_file", { path: localPath, groups });
      setSaveOk(backup ? `Gespeichert. Backup: ${backup}` : "Gespeichert.");
    } catch (e) {
      setSaveError(String(e));
    } finally {
      setSaving(false);
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
          <div className="relative">
            <Button
              variant="ghost"
              size="icon-sm"
              onClick={() => setServerInfoOpen((v) => !v)}
              title="Wie Prozentwerte auf diesem Server wirken"
            >
              <Info className="size-4" />
            </Button>
            {serverInfoOpen && (
              <div className="absolute left-0 top-full z-10 mt-1 w-96 space-y-3 rounded-lg border border-border bg-card p-3 shadow-lg">
                {SERVER_NOTES.map((note, i) => (
                  <div key={i} className="space-y-1.5">
                    <p className="text-sm font-semibold">{note.title}</p>
                    {note.intro && (
                      <p className="text-xs text-muted-foreground">{note.intro}</p>
                    )}
                    {note.formula && (
                      <p className="rounded-md bg-muted/60 px-2 py-1 text-xs font-medium">
                        {note.formula}
                      </p>
                    )}
                    {note.bullets && (
                      <ul className="list-disc space-y-1 pl-4 text-xs text-muted-foreground">
                        {note.bullets.map((b, bi) => (
                          <li key={bi}>{b}</li>
                        ))}
                      </ul>
                    )}
                  </div>
                ))}
                <Button variant="outline" size="sm" onClick={() => setServerInfoOpen(false)}>
                  Schließen
                </Button>
              </div>
            )}
          </div>
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
        <div className="flex w-72 shrink-0 flex-col gap-2">
          <div className="flex gap-2">
            <div className="relative flex-1">
              <Search className="absolute left-2 top-1/2 size-3.5 -translate-y-1/2 text-muted-foreground" />
              <input
                value={groupSearch}
                onChange={(e) => setGroupSearch(e.target.value)}
                placeholder="Mob suchen (Name/VNUM)…"
                className="w-full rounded-md border border-border bg-background py-1 pl-7 pr-2 text-sm"
              />
            </div>
            <Button variant="outline" size="icon" onClick={() => setCreating(true)}>
              <Plus className="size-4" />
            </Button>
          </div>
          <div className="flex-1 space-y-1 overflow-y-auto">
            {filteredGroups.map(({ g, index }) => (
              <div
                key={index}
                onClick={() => setSelectedIndex(index)}
                className={`cursor-pointer rounded-md border border-border p-2 text-sm hover:bg-muted ${
                  selectedIndex === index ? "bg-muted" : ""
                }`}
              >
                <div className="font-medium">{g.name}</div>
                <div className="text-xs text-muted-foreground">
                  Mob: {g.mob_vnum} · {g.items.length} Drops
                </div>
              </div>
            ))}
            {filteredGroups.length === 0 && (
              <p className="p-2 text-sm text-muted-foreground">Keine Mobs in der Datei.</p>
            )}
          </div>
        </div>

        <div className="min-w-0 flex-1 space-y-4 overflow-y-auto">
          {!selected && (
            <p className="text-sm text-muted-foreground">Wähle links einen Mob aus.</p>
          )}
          {selected && selectedIndex !== null && (
            <>
              <div className="flex items-start justify-between rounded-lg border border-border p-3">
                <div className="flex flex-wrap items-end gap-3">
                  <Field label="Gruppenname">
                    <input
                      value={selected.name}
                      onChange={(e) => updateGroup(selectedIndex, { name: e.target.value })}
                      className="w-48 rounded-md border border-border bg-background px-2 py-1 text-sm"
                    />
                  </Field>
                  <Field label="Mob-VNUM">
                    <input
                      type="number"
                      value={selected.mob_vnum}
                      onChange={(e) =>
                        updateGroup(selectedIndex, { mob_vnum: Number(e.target.value) || 0 })
                      }
                      className="w-24 rounded-md border border-border bg-background px-2 py-1 text-sm"
                    />
                  </Field>
                  <Field label="Type">
                    <input
                      value={selected.drop_type}
                      onChange={(e) => updateGroup(selectedIndex, { drop_type: e.target.value })}
                      className="w-24 rounded-md border border-border bg-background px-2 py-1 text-sm"
                    />
                  </Field>
                </div>
                <Button variant="destructive" size="sm" onClick={() => setDeleteConfirm(selectedIndex)}>
                  <Trash2 className="size-4" />
                  Mob löschen
                </Button>
              </div>

              <div className="space-y-2 rounded-lg border border-border p-3">
                <div className="flex items-center justify-between">
                  <h2 className="text-sm font-medium text-muted-foreground">
                    Drops ({selected.items.length})
                  </h2>
                  <Button
                    size="sm"
                    variant="outline"
                    onClick={() => {
                      setItemPicker(true);
                      setItemSearchError(null);
                    }}
                  >
                    <Plus className="size-3.5" />
                    Item hinzufügen
                  </Button>
                </div>
                <div className="space-y-1">
                  {selected.items.map((item, itemIndex) => (
                    <div
                      key={itemIndex}
                      className="flex items-center gap-3 rounded-md border border-border p-2"
                    >
                      <div className="flex size-9 shrink-0 items-center justify-center rounded-md border border-border bg-muted/40">
                        {icons[item.item_vnum] ? (
                          <img
                            src={icons[item.item_vnum]!}
                            alt=""
                            className="max-h-full w-7 object-contain [image-rendering:pixelated]"
                          />
                        ) : (
                          <span className="text-[10px] text-muted-foreground">{item.item_vnum}</span>
                        )}
                      </div>
                      <span className="w-20 shrink-0 text-xs text-muted-foreground">
                        #{item.item_vnum}
                      </span>
                      <Field label="Anzahl">
                        <input
                          type="number"
                          min={1}
                          value={item.count}
                          onChange={(e) =>
                            updateItem(selectedIndex, itemIndex, {
                              count: Math.max(1, Number(e.target.value) || 1),
                            })
                          }
                          className="w-16 rounded-md border border-border bg-background px-2 py-1 text-sm"
                        />
                      </Field>
                      <Field label="Prozent">
                        <input
                          type="number"
                          min={0}
                          max={100}
                          step={0.01}
                          value={item.percent}
                          onChange={(e) =>
                            updateItem(selectedIndex, itemIndex, {
                              percent: clampPercent(Number(e.target.value) || 0),
                            })
                          }
                          className="w-20 rounded-md border border-border bg-background px-2 py-1 text-sm"
                        />
                      </Field>
                      <span className="text-xs text-muted-foreground" title="Reale Drop-Chance auf diesem Server (siehe i-Info oben)">
                        ≈{realChance(item.percent)}
                      </span>
                      <Button
                        variant="ghost"
                        size="icon-sm"
                        className="ml-auto"
                        onClick={() => removeItem(selectedIndex, itemIndex)}
                      >
                        <X className="size-4" />
                      </Button>
                    </div>
                  ))}
                  {selected.items.length === 0 && (
                    <p className="text-sm text-muted-foreground">Noch keine Drops.</p>
                  )}
                </div>
              </div>
            </>
          )}

          {/* Bulk-Änderung */}
          <div className="space-y-3 rounded-lg border border-border p-3">
            <h2 className="text-sm font-medium text-muted-foreground">
              Prozentwerte in einem Rutsch ändern
            </h2>

            <div className="flex overflow-hidden rounded-md border border-border text-sm w-fit">
              <button
                onClick={() => setBulkScope("global")}
                className={`px-3 py-1 ${bulkScope === "global" ? "bg-primary text-primary-foreground" : ""}`}
              >
                Global (alle Mobs)
              </button>
              <button
                onClick={() => setBulkScope("current")}
                disabled={!selected}
                className={`px-3 py-1 disabled:opacity-40 ${bulkScope === "current" ? "bg-primary text-primary-foreground" : ""}`}
              >
                Nur dieser Mob
              </button>
            </div>

            <div className="flex flex-wrap gap-2 text-sm">
              {(
                [
                  ["delta", "Addieren/Subtrahieren"],
                  ["fixed", "Fester Wert"],
                  ["random", "Zufall in Bereich"],
                  ["specific-item", "Bestimmtes Item überall"],
                ] as const
              ).map(([mode, label]) => (
                <button
                  key={mode}
                  onClick={() => setBulkMode(mode)}
                  className={`rounded-md border border-border px-3 py-1 ${
                    bulkMode === mode ? "bg-primary text-primary-foreground" : ""
                  }`}
                >
                  {label}
                </button>
              ))}
            </div>

            {bulkMode === "delta" && (
              <Field label="Betrag (z.B. -5 oder 5)">
                <input
                  type="number"
                  value={deltaValue}
                  onChange={(e) => setDeltaValue(Number(e.target.value) || 0)}
                  className="w-24 rounded-md border border-border bg-background px-2 py-1 text-sm"
                />
              </Field>
            )}
            {bulkMode === "fixed" && (
              <div className="flex items-end gap-2">
                <Field label="Neuer Wert für alle">
                  <input
                    type="number"
                    min={0}
                    max={100}
                    value={fixedValue}
                    onChange={(e) => setFixedValue(Number(e.target.value) || 0)}
                    className="w-24 rounded-md border border-border bg-background px-2 py-1 text-sm"
                  />
                </Field>
                <span className="pb-1.5 text-xs text-muted-foreground">≈{realChance(fixedValue)} real</span>
              </div>
            )}
            {bulkMode === "random" && (
              <div className="flex items-end gap-3">
                <Field label="Min">
                  <input
                    type="number"
                    min={0}
                    max={100}
                    value={randomMin}
                    onChange={(e) => setRandomMin(Number(e.target.value) || 0)}
                    className="w-20 rounded-md border border-border bg-background px-2 py-1 text-sm"
                  />
                </Field>
                <Field label="Max">
                  <input
                    type="number"
                    min={0}
                    max={100}
                    value={randomMax}
                    onChange={(e) => setRandomMax(Number(e.target.value) || 0)}
                    className="w-20 rounded-md border border-border bg-background px-2 py-1 text-sm"
                  />
                </Field>
                <span className="pb-1.5 text-xs text-muted-foreground">
                  ≈{realChance(randomMin)}–{realChance(randomMax)} real
                </span>
              </div>
            )}
            {bulkMode === "specific-item" && (
              <div className="flex items-end gap-2">
                <Field label="Item-VNUM">
                  <input
                    type="number"
                    value={specificItemVnum}
                    onChange={(e) => setSpecificItemVnum(e.target.value)}
                    className="w-24 rounded-md border border-border bg-background px-2 py-1 text-sm"
                  />
                </Field>
                <Field label="Prozent">
                  <input
                    type="number"
                    min={0}
                    max={100}
                    value={specificItemPercent}
                    onChange={(e) => setSpecificItemPercent(Number(e.target.value) || 0)}
                    className="w-20 rounded-md border border-border bg-background px-2 py-1 text-sm"
                  />
                </Field>
                <span className="pb-1.5 text-xs text-muted-foreground">
                  ≈{realChance(specificItemPercent)} real
                </span>
              </div>
            )}

            <Button
              variant="outline"
              onClick={() => setBulkConfirm(true)}
              disabled={!groups || groups.length === 0 || (bulkScope === "current" && !selected)}
            >
              Anwenden
            </Button>
            <p className="text-xs text-muted-foreground">
              Wirkt zunächst nur auf die geladenen Daten - erst "Speichern" oben schreibt sie in die
              Datei.
            </p>
          </div>
        </div>
      </div>

      {/* Item-Picker */}
      {itemPicker && (
        <Modal onClose={() => setItemPicker(false)}>
          <div className="space-y-2">
            <div className="flex gap-2">
              <input
                autoFocus
                value={itemQuery}
                onChange={(e) => setItemQuery(e.target.value)}
                onKeyDown={(e) => e.key === "Enter" && runItemSearch()}
                placeholder="Item nach Name oder VNUM suchen…"
                className="flex-1 rounded-md border border-border bg-background px-2 py-1 text-sm"
              />
              <Button variant="outline" onClick={runItemSearch} disabled={itemSearching}>
                <Search className="size-4" />
              </Button>
            </div>
            {itemSearchError && (
              <p className="flex items-start gap-2 text-sm text-destructive">
                <AlertTriangle className="mt-0.5 size-4 shrink-0" />
                <span>{itemSearchError}</span>
              </p>
            )}
            <div className="max-h-64 space-y-1 overflow-y-auto">
              {itemResults.map((item) => (
                <div
                  key={item.vnum}
                  className="flex items-center justify-between rounded-md px-2 py-1 text-sm hover:bg-muted"
                >
                  <span className="flex items-center gap-2">
                    {icons[item.vnum] && (
                      <img
                        src={icons[item.vnum]!}
                        alt=""
                        className="size-6 object-contain [image-rendering:pixelated]"
                      />
                    )}
                    {item.name} <span className="text-muted-foreground">#{item.vnum}</span>
                  </span>
                  <Button size="sm" onClick={() => addItem(item)}>
                    Hinzufügen
                  </Button>
                </div>
              ))}
            </div>
          </div>
        </Modal>
      )}

      {/* Reverse-Suche: wer droppt Item X */}
      {reverseLookupOpen && (
        <Modal onClose={closeReverseLookup}>
          <div className="space-y-2">
            <p className="text-sm font-medium">Wer droppt dieses Item?</p>
            <div className="flex gap-2">
              <input
                autoFocus
                value={reverseQuery}
                onChange={(e) => setReverseQuery(e.target.value)}
                onKeyDown={(e) => e.key === "Enter" && runReverseSearch()}
                placeholder="Item nach Name oder VNUM suchen…"
                className="flex-1 rounded-md border border-border bg-background px-2 py-1 text-sm"
              />
              <Button variant="outline" onClick={runReverseSearch} disabled={reverseSearching}>
                <Search className="size-4" />
              </Button>
            </div>
            {reverseSearchError && (
              <p className="flex items-start gap-2 text-sm text-destructive">
                <AlertTriangle className="mt-0.5 size-4 shrink-0" />
                <span>{reverseSearchError}</span>
              </p>
            )}

            {!reverseSelectedItem && (
              <div className="max-h-64 space-y-1 overflow-y-auto">
                {reverseResults.map((item) => (
                  <div
                    key={item.vnum}
                    className="flex items-center justify-between rounded-md px-2 py-1 text-sm hover:bg-muted"
                  >
                    <span>
                      {item.name} <span className="text-muted-foreground">#{item.vnum}</span>
                    </span>
                    <Button size="sm" onClick={() => setReverseSelectedItem(item)}>
                      Auswählen
                    </Button>
                  </div>
                ))}
                {reverseResults.length === 0 && !reverseSearching && (
                  <p className="p-2 text-sm text-muted-foreground">Noch keine Suche.</p>
                )}
              </div>
            )}

            {reverseSelectedItem && (
              <div className="space-y-2">
                <div className="flex items-center justify-between">
                  <p className="text-sm">
                    <strong>{reverseSelectedItem.name}</strong>{" "}
                    <span className="text-muted-foreground">#{reverseSelectedItem.vnum}</span> wird
                    von {reverseMatches.length} Mob(s) gedroppt:
                  </p>
                  <button
                    className="text-xs text-muted-foreground underline hover:text-foreground"
                    onClick={() => {
                      setReverseSelectedItem(null);
                      setReverseResults([]);
                      setReverseQuery("");
                    }}
                  >
                    Anderes Item
                  </button>
                </div>
                <div className="max-h-72 space-y-1 overflow-y-auto">
                  {reverseMatches.map(({ g, index, drop }) => (
                    <button
                      key={index}
                      onClick={() => {
                        setSelectedIndex(index);
                        closeReverseLookup();
                      }}
                      className="flex w-full items-center justify-between rounded-md border border-border px-2 py-1 text-left text-sm hover:bg-muted"
                    >
                      <span>
                        {g.name} <span className="text-muted-foreground">(Mob #{g.mob_vnum})</span>
                      </span>
                      <span className="text-xs text-muted-foreground">
                        Anzahl {drop.count} · {drop.percent}% (≈{realChance(drop.percent)} real)
                      </span>
                    </button>
                  ))}
                  {reverseMatches.length === 0 && (
                    <p className="p-2 text-sm text-muted-foreground">
                      Kein geladener Mob droppt dieses Item.
                    </p>
                  )}
                </div>
              </div>
            )}
          </div>
        </Modal>
      )}

      {/* Neuer Mob */}
      {creating && (
        <Modal onClose={() => setCreating(false)}>
          <div className="space-y-3">
            <p className="text-sm font-medium">Neuen Mob-Eintrag anlegen</p>
            <Field label="Name">
              <input
                autoFocus
                value={newName}
                onChange={(e) => setNewName(e.target.value)}
                placeholder="z.B. Wildhund"
                className="w-full rounded-md border border-border bg-background px-2 py-1 text-sm"
              />
            </Field>
            {newName.trim() && (
              <p className="text-xs text-muted-foreground">
                Wird gespeichert als: <code>{newNamePreview || "…"}</code>
              </p>
            )}
            <Field label="Mob-VNUM">
              <input
                type="number"
                value={newMobVnum}
                onChange={(e) => setNewMobVnum(e.target.value)}
                className="w-full rounded-md border border-border bg-background px-2 py-1 text-sm"
              />
            </Field>
            <Field label="Type">
              <input
                value={newType}
                onChange={(e) => setNewType(e.target.value)}
                className="w-full rounded-md border border-border bg-background px-2 py-1 text-sm"
              />
            </Field>
            <Button
              onClick={submitNewGroup}
              disabled={!newNamePreview || !newMobVnum}
            >
              Anlegen
            </Button>
          </div>
        </Modal>
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

      {bulkConfirm && (
        <div className="fixed inset-0 flex items-center justify-center bg-black/50">
          <div className="w-96 space-y-3 rounded-lg border border-border bg-card p-4">
            <p className="text-sm">
              Prozentwerte {bulkScope === "global" ? "aller Mobs" : `von "${selected?.name}"`} jetzt
              ändern? Das betrifft nur die geladenen Daten, bis du speicherst.
            </p>
            <div className="flex justify-end gap-2">
              <Button variant="outline" onClick={() => setBulkConfirm(false)}>
                Abbrechen
              </Button>
              <Button onClick={applyBulkEdit}>Anwenden</Button>
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

function Field({ label, children }: { label: string; children: React.ReactNode }) {
  return (
    <label className="flex flex-col gap-1 text-xs text-muted-foreground">
      {label}
      {children}
    </label>
  );
}

function Modal({ children, onClose }: { children: React.ReactNode; onClose: () => void }) {
  return (
    <div className="fixed inset-0 flex items-center justify-center bg-black/50">
      <div className="w-96 rounded-lg border border-border bg-card p-4">
        <div className="mb-2 flex justify-end">
          <button onClick={onClose} className="text-muted-foreground hover:text-foreground">
            <X className="size-4" />
          </button>
        </div>
        {children}
      </div>
    </div>
  );
}
