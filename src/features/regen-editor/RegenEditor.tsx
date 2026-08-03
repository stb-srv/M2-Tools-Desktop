import { useEffect, useMemo, useState } from "react";
import { invoke } from "@tauri-apps/api/core";
import { runAsyncAction } from "@/lib/asyncAction";
import { reportSectionDirty } from "@/store/navigation";
import { Button } from "@/components/ui/button";
import { RegenMapView, type MapFolderInfo, type RegenMapImage } from "./RegenMapView";
import {
  Folder,
  FileText,
  ChevronRight,
  Home,
  Plus,
  Trash2,
  RefreshCw,
  AlertTriangle,
  CheckCircle2,
  Table2,
  Map as MapIcon,
} from "lucide-react";

interface RemoteEntry {
  name: string;
  is_dir: boolean;
}

export interface RegenSpawn {
  kind: "Spawn";
  spawn_type: string;
  center_x: number;
  center_y: number;
  radius_x: number;
  radius_y: number;
  z_section: number;
  direction: number;
  regen_time: string;
  regen_percent: number;
  max_count: number;
  vnum: number;
}

export interface RegenException {
  kind: "Exception";
  center_x: number;
  center_y: number;
  radius_x: number;
  radius_y: number;
  z_section: number;
}

export interface RegenRaw {
  kind: "Raw";
  text: string;
}

export type RegenLine = RegenSpawn | RegenException | RegenRaw;

export const SPAWN_TYPES = [
  { value: "m", label: "m - Einzelner Mob" },
  { value: "g", label: "g - Gruppe" },
  { value: "ga", label: "ga - Gruppe (aggressiv)" },
  { value: "r", label: "r - Gruppe von Gruppen" },
  { value: "s", label: "s - Irgendwo auf der Map" },
];

export function newSpawn(): RegenSpawn {
  return {
    kind: "Spawn",
    spawn_type: "m",
    center_x: 0,
    center_y: 0,
    radius_x: 10,
    radius_y: 10,
    z_section: 0,
    direction: 0,
    regen_time: "0s",
    regen_percent: 100,
    max_count: 1,
    vnum: 0,
  };
}

export function newException(): RegenException {
  return { kind: "Exception", center_x: 0, center_y: 0, radius_x: 10, radius_y: 10, z_section: 0 };
}

const DEFAULT_BASE_DIR = "/usr/home/game/share";

export function RegenEditor() {
  const [baseDir, setBaseDir] = useState<string | null>(null);
  // Absolute path currently being browsed - `list_remote_dir` takes a plain
  // path (it's shared with the future Backup-Browser and doesn't know about
  // `regen_base_dir`), so navigation works in absolute terms and only the
  // read/write calls convert back to a base-relative path.
  const [dir, setDir] = useState<string | null>(null);
  const [entries, setEntries] = useState<RemoteEntry[] | null>(null);
  const [dirLoading, setDirLoading] = useState(false);
  const [dirError, setDirError] = useState<string | null>(null);

  const [selectedPath, setSelectedPath] = useState<string | null>(null);
  const [lines, setLines] = useState<RegenLine[] | null>(null);
  const [contentLoading, setContentLoading] = useState(false);
  const [contentError, setContentError] = useState<string | null>(null);
  const [dirty, setDirty] = useState(false);
  useEffect(() => {
    reportSectionDirty("regen-editor", dirty);
    return () => reportSectionDirty("regen-editor", false);
  }, [dirty]);

  const [saving, setSaving] = useState(false);
  const [saveOk, setSaveOk] = useState<string | null>(null);
  const [saveError, setSaveError] = useState<string | null>(null);
  const [saveConfirm, setSaveConfirm] = useState(false);

  // Karten-Ansicht: alternative Darstellung derselben `lines` - keine eigene
  // Datenquelle, siehe RegenMapView.
  const [editorMode, setEditorMode] = useState<"table" | "map">("table");
  const [mapImage, setMapImage] = useState<RegenMapImage | null>(null);
  const [mapLoading, setMapLoading] = useState(false);
  const [mapError, setMapError] = useState<string | null>(null);
  const [mapFolders, setMapFolders] = useState<MapFolderInfo[] | null>(null);
  const [mapFoldersError, setMapFoldersError] = useState<string | null>(null);
  const [showMapPicker, setShowMapPicker] = useState(false);
  const [mapSelection, setMapSelection] = useState<{ category: string; folderName: string } | null>(
    null,
  );

  useEffect(() => {
    setMapImage(null);
    setMapError(null);
    setShowMapPicker(false);
    setMapSelection(null);
  }, [selectedPath]);

  useEffect(() => {
    if (editorMode !== "map" || !selectedPath || mapImage || mapLoading) return;
    (async () => {
      const mapping = await invoke<string | null>("get_setting", {
        key: `regen_map_mapping:${selectedPath}`,
      }).catch(() => null);
      if (mapping) {
        const [category, folderName] = mapping.split("/");
        if (category && folderName) {
          await loadMapImage(category, folderName);
          return;
        }
      }
      setShowMapPicker(true);
      if (!mapFolders) await loadMapFolders();
    })();
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, [editorMode, selectedPath]);

  async function loadMapFolders() {
    await runAsyncAction(() => invoke<MapFolderInfo[]>("list_client_map_folders"), {
      onSuccess: setMapFolders,
      onError: setMapFoldersError,
    });
  }

  async function loadMapImage(category: string, folderName: string, forceRebuild = false) {
    setMapSelection({ category, folderName });
    await runAsyncAction(
      () =>
        invoke<RegenMapImage>("get_regen_map_image", { category, folderName, forceRebuild }),
      {
        onStart: () => {
          setMapLoading(true);
          setMapError(null);
        },
        onSuccess: setMapImage,
        onError: setMapError,
        onFinally: () => setMapLoading(false),
      },
    );
  }

  async function chooseMapFolder(category: string, folderName: string) {
    if (!selectedPath) return;
    setShowMapPicker(false);
    await invoke("set_setting", {
      key: `regen_map_mapping:${selectedPath}`,
      value: `${category}/${folderName}`,
    }).catch(() => {});
    await loadMapImage(category, folderName);
  }

  useEffect(() => {
    (async () => {
      const configured =
        (await invoke<string | null>("get_setting", { key: "regen_base_dir" }).catch(
          () => null,
        )) || DEFAULT_BASE_DIR;
      setBaseDir(configured);
      setDir(configured);
    })();
  }, []);

  useEffect(() => {
    if (dir !== null) loadDir(dir);
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, [dir]);

  async function loadDir(path: string) {
    await runAsyncAction(() => invoke<RemoteEntry[]>("list_remote_dir", { path }), {
      onStart: () => {
        setDirLoading(true);
        setDirError(null);
      },
      onSuccess: setEntries,
      onError: setDirError,
      onFinally: () => setDirLoading(false),
    });
  }

  function toRelative(absolute: string) {
    if (!baseDir) return absolute;
    return absolute.slice(baseDir.length).replace(/^\//, "");
  }

  function openFolder(name: string) {
    setDir((prev) => `${prev}/${name}`);
  }

  function goUp() {
    setDir((prev) => {
      if (!prev || !baseDir || prev.length <= baseDir.length) return prev;
      const idx = prev.lastIndexOf("/");
      return idx === -1 ? baseDir : prev.slice(0, idx);
    });
  }

  async function openFile(name: string) {
    if (!dir || !baseDir) return;
    const relative = toRelative(`${dir}/${name}`);
    setSelectedPath(relative);
    setLines(null);
    setContentError(null);
    setDirty(false);
    setSaveOk(null);
    setSaveError(null);
    await runAsyncAction(() => invoke<RegenLine[]>("read_regen_file", { relativePath: relative }), {
      onStart: () => setContentLoading(true),
      onSuccess: setLines,
      onError: setContentError,
      onFinally: () => setContentLoading(false),
    });
  }

  function updateLine(index: number, patch: Partial<RegenLine>) {
    setLines((prev) =>
      prev!.map((l, i) => (i === index ? ({ ...l, ...patch } as RegenLine) : l)),
    );
    setDirty(true);
    setSaveOk(null);
  }

  function removeLine(index: number) {
    setLines((prev) => prev!.filter((_, i) => i !== index));
    setDirty(true);
    setSaveOk(null);
  }

  function addLine(line: RegenLine) {
    setLines((prev) => [...(prev ?? []), line]);
    setDirty(true);
    setSaveOk(null);
  }

  async function saveFile() {
    if (!selectedPath || !lines) return;
    setSaveConfirm(false);
    await runAsyncAction(
      () => invoke<string | null>("write_regen_file", { relativePath: selectedPath, lines }),
      {
        onStart: () => {
          setSaving(true);
          setSaveError(null);
        },
        onSuccess: (backup) => {
          setSaveOk(backup ? `Gespeichert. Backup: ${backup}` : "Gespeichert.");
          setDirty(false);
        },
        onError: setSaveError,
        onFinally: () => setSaving(false),
      },
    );
  }

  const breadcrumbs = useMemo(() => {
    if (!dir || !baseDir || dir === baseDir) return [];
    return dir.slice(baseDir.length).replace(/^\//, "").split("/");
  }, [dir, baseDir]);

  return (
    <div className="flex h-full flex-col gap-3">
      <div>
        <h1 className="text-2xl font-semibold">Regen-Datei-Editor</h1>
        <p className="text-sm text-muted-foreground">
          Bearbeitet die Monster-Spawn-Dateien, auf die Dungeon-Etagen im Quest Builder verweisen
          (z.B. <code>data/dungeon/dt_short/deviltower3_regen.txt</code>). Format 1:1 aus dem
          Server-Quellcode verifiziert (<code>regen.cpp</code>).
        </p>
      </div>

      <div className="flex min-h-0 flex-1 gap-4">
        <div className="flex w-80 shrink-0 flex-col gap-2">
          <div className="flex items-center gap-1 text-sm">
            <Button variant="ghost" size="icon-sm" onClick={() => setDir(baseDir)}>
              <Home className="size-3.5" />
            </Button>
            {breadcrumbs.map((part, i) => (
              <span key={i} className="flex items-center gap-1 text-muted-foreground">
                <ChevronRight className="size-3.5" />
                <button
                  className="hover:text-foreground"
                  onClick={() => setDir(`${baseDir}/${breadcrumbs.slice(0, i + 1).join("/")}`)}
                >
                  {part}
                </button>
              </span>
            ))}
            <Button
              variant="ghost"
              size="icon-sm"
              className="ml-auto"
              onClick={() => dir && loadDir(dir)}
            >
              <RefreshCw className={`size-3.5 ${dirLoading ? "animate-spin" : ""}`} />
            </Button>
          </div>

          {dirError && <p className="text-sm text-destructive">{dirError}</p>}

          <div className="flex-1 space-y-1 overflow-y-auto rounded-md border border-border p-1">
            {dir && baseDir && dir !== baseDir && (
              <button
                onClick={goUp}
                className="flex w-full items-center gap-2 rounded-md px-2 py-1 text-left text-sm hover:bg-muted"
              >
                <Folder className="size-4 text-muted-foreground" />
                ..
              </button>
            )}
            {(entries ?? []).map((entry) => (
              <button
                key={entry.name}
                onClick={() => (entry.is_dir ? openFolder(entry.name) : openFile(entry.name))}
                className={`flex w-full items-center gap-2 rounded-md px-2 py-1 text-left text-sm hover:bg-muted ${
                  !entry.is_dir && dir && selectedPath === toRelative(`${dir}/${entry.name}`)
                    ? "bg-muted"
                    : ""
                }`}
              >
                {entry.is_dir ? (
                  <Folder className="size-4 text-muted-foreground" />
                ) : (
                  <FileText className="size-4 text-muted-foreground" />
                )}
                {entry.name}
              </button>
            ))}
            {entries && entries.length === 0 && (
              <p className="p-2 text-sm text-muted-foreground">Leerer Ordner.</p>
            )}
          </div>
        </div>

        <div className="flex min-w-0 flex-1 flex-col gap-2">
          {!selectedPath && (
            <div className="flex flex-1 flex-col items-center justify-center rounded-lg border border-dashed border-border">
              <p className="text-sm text-muted-foreground">
                Wähle links eine Regen-Datei (.txt) aus.
              </p>
            </div>
          )}

          {selectedPath && (
            <>
              <div className="flex items-center justify-between">
                <code className="text-sm">{selectedPath}</code>
                <div className="flex items-center gap-2">
                  {saveOk && (
                    <span className="flex items-center gap-1 text-sm text-green-600">
                      <CheckCircle2 className="size-4" /> {saveOk}
                    </span>
                  )}
                  <div className="flex items-center rounded-md border border-border p-0.5">
                    <Button
                      variant={editorMode === "table" ? "secondary" : "ghost"}
                      size="sm"
                      onClick={() => setEditorMode("table")}
                    >
                      <Table2 className="size-3.5" />
                      Tabelle
                    </Button>
                    <Button
                      variant={editorMode === "map" ? "secondary" : "ghost"}
                      size="sm"
                      onClick={() => setEditorMode("map")}
                    >
                      <MapIcon className="size-3.5" />
                      Karte
                    </Button>
                  </div>
                  <Button variant="outline" size="sm" onClick={() => addLine(newSpawn())}>
                    <Plus className="size-3.5" />
                    Spawn
                  </Button>
                  <Button variant="outline" size="sm" onClick={() => addLine(newException())}>
                    <Plus className="size-3.5" />
                    Ausnahmezone
                  </Button>
                  <Button size="sm" onClick={() => setSaveConfirm(true)} disabled={saving || !dirty}>
                    {saving ? "Speichere…" : "Speichern"}
                  </Button>
                </div>
              </div>
              {saveError && <p className="text-sm text-destructive">{saveError}</p>}
              {contentError && <p className="text-sm text-destructive">{contentError}</p>}
              {contentLoading && <p className="text-sm text-muted-foreground">Lade…</p>}

              {lines && editorMode === "table" && (
                <div className="flex-1 space-y-1 overflow-y-auto">
                  {lines.map((line, index) => (
                    <RegenLineRow
                      key={index}
                      line={line}
                      onChange={(patch) => updateLine(index, patch)}
                      onRemove={() => removeLine(index)}
                    />
                  ))}
                  {lines.length === 0 && (
                    <p className="p-2 text-sm text-muted-foreground">Leere Datei.</p>
                  )}
                </div>
              )}

              {lines && editorMode === "map" && (
                <div className="min-h-0 flex-1">
                  {mapLoading && (
                    <p className="p-2 text-sm text-muted-foreground">Lade Karte…</p>
                  )}
                  {mapError && <p className="p-2 text-sm text-destructive">{mapError}</p>}
                  {mapImage && (
                    <RegenMapView
                      lines={lines}
                      mapImage={mapImage}
                      onUpdateLine={updateLine}
                      onAddLine={addLine}
                      onRemoveLine={removeLine}
                      onReloadMap={
                        mapSelection
                          ? () => loadMapImage(mapSelection.category, mapSelection.folderName, true)
                          : undefined
                      }
                      onChangeMapFolder={() => {
                        setShowMapPicker(true);
                        if (!mapFolders) void loadMapFolders();
                      }}
                    />
                  )}
                  {!mapLoading && !mapImage && !mapError && !showMapPicker && (
                    <p className="p-2 text-sm text-muted-foreground">
                      Keine Karte zugeordnet.
                    </p>
                  )}
                </div>
              )}
            </>
          )}
        </div>
      </div>

      {showMapPicker && (
        <div className="fixed inset-0 flex items-center justify-center bg-black/50">
          <div className="max-h-[70vh] w-[32rem] space-y-3 overflow-y-auto rounded-lg border border-border bg-card p-4">
            <h2 className="text-sm font-medium">Kartenordner zuordnen</h2>
            <p className="text-xs text-muted-foreground">
              Wähle den Client-Kartenordner, der zu <code>{selectedPath}</code> gehört. Die
              Zuordnung wird für diese Datei gemerkt.
            </p>
            {mapFoldersError && (
              <p className="text-sm text-destructive">{mapFoldersError}</p>
            )}
            {!mapFolders && !mapFoldersError && (
              <p className="text-sm text-muted-foreground">Lade Kartenordner…</p>
            )}
            {mapFolders && mapFolders.length === 0 && (
              <p className="text-sm text-muted-foreground">
                Keine entpackten Kartenordner unter dem konfigurierten Client-Pfad gefunden.
              </p>
            )}
            <div className="space-y-2">
              {mapFolders &&
                Object.entries(
                  mapFolders.reduce<Record<string, MapFolderInfo[]>>((acc, f) => {
                    (acc[f.category] ??= []).push(f);
                    return acc;
                  }, {}),
                ).map(([category, folders]) => (
                  <div key={category}>
                    <p className="text-xs font-medium text-muted-foreground">{category}</p>
                    <div className="flex flex-wrap gap-1 pt-1">
                      {folders.map((f) => (
                        <button
                          key={f.folder_name}
                          onClick={() => chooseMapFolder(f.category, f.folder_name)}
                          className="rounded-md border border-border px-2 py-1 text-xs hover:bg-muted"
                        >
                          {f.folder_name}
                        </button>
                      ))}
                    </div>
                  </div>
                ))}
            </div>
            <div className="flex justify-end">
              <Button variant="outline" size="sm" onClick={() => setShowMapPicker(false)}>
                Abbrechen
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
              <span>Die Datei auf dem Server wird gesichert und dann überschrieben.</span>
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
    </div>
  );
}

function NumField({
  label,
  value,
  onChange,
  width = "w-16",
}: {
  label: string;
  value: number;
  onChange: (value: number) => void;
  width?: string;
}) {
  return (
    <label className="flex flex-col gap-0.5 text-[10px] text-muted-foreground">
      {label}
      <input
        type="number"
        value={value}
        onChange={(e) => onChange(Number(e.target.value) || 0)}
        className={`${width} rounded-md border border-border bg-background px-1.5 py-1 text-sm`}
      />
    </label>
  );
}

export function RegenLineRow({
  line,
  onChange,
  onRemove,
}: {
  line: RegenLine;
  onChange: (patch: Partial<RegenLine>) => void;
  onRemove: () => void;
}) {
  if (line.kind === "Raw") {
    return (
      <div className="flex items-center gap-2 rounded-md border border-dashed border-border px-2 py-1 text-xs text-muted-foreground">
        <span className="flex-1 truncate font-mono">{line.text || "(leere Zeile)"}</span>
        <Button variant="ghost" size="icon-sm" onClick={onRemove}>
          <Trash2 className="size-3.5" />
        </Button>
      </div>
    );
  }

  if (line.kind === "Exception") {
    return (
      <div className="flex flex-wrap items-end gap-2 rounded-md border border-border p-2">
        <span className="mb-1 text-xs font-medium">Ausnahmezone</span>
        <NumField label="X" value={line.center_x} onChange={(v) => onChange({ center_x: v })} />
        <NumField label="Y" value={line.center_y} onChange={(v) => onChange({ center_y: v })} />
        <NumField
          label="Radius X"
          value={line.radius_x}
          onChange={(v) => onChange({ radius_x: v })}
        />
        <NumField
          label="Radius Y"
          value={line.radius_y}
          onChange={(v) => onChange({ radius_y: v })}
        />
        <NumField
          label="Z-Sektion"
          value={line.z_section}
          onChange={(v) => onChange({ z_section: v })}
        />
        <Button variant="ghost" size="icon-sm" className="ml-auto" onClick={onRemove}>
          <Trash2 className="size-3.5" />
        </Button>
      </div>
    );
  }

  return (
    <div className="flex flex-wrap items-end gap-2 rounded-md border border-border p-2">
      <label className="flex flex-col gap-0.5 text-[10px] text-muted-foreground">
        Typ
        <select
          value={line.spawn_type}
          onChange={(e) => onChange({ spawn_type: e.target.value })}
          className="w-36 rounded-md border border-border bg-background px-1.5 py-1 text-sm"
        >
          {SPAWN_TYPES.map((t) => (
            <option key={t.value} value={t.value}>
              {t.label}
            </option>
          ))}
        </select>
      </label>
      <NumField label="X" value={line.center_x} onChange={(v) => onChange({ center_x: v })} />
      <NumField label="Y" value={line.center_y} onChange={(v) => onChange({ center_y: v })} />
      <NumField
        label="Radius X"
        value={line.radius_x}
        onChange={(v) => onChange({ radius_x: v })}
      />
      <NumField
        label="Radius Y"
        value={line.radius_y}
        onChange={(v) => onChange({ radius_y: v })}
      />
      <NumField
        label="Z-Sektion"
        value={line.z_section}
        onChange={(v) => onChange({ z_section: v })}
      />
      <NumField
        label="Richtung"
        value={line.direction}
        onChange={(v) => onChange({ direction: v })}
      />
      <label className="flex flex-col gap-0.5 text-[10px] text-muted-foreground">
        Regen-Zeit
        <input
          value={line.regen_time}
          onChange={(e) => onChange({ regen_time: e.target.value })}
          placeholder="z.B. 1h30m"
          className="w-20 rounded-md border border-border bg-background px-1.5 py-1 text-sm"
        />
      </label>
      <NumField
        label="Max. Anzahl"
        value={line.max_count}
        onChange={(v) => onChange({ max_count: v })}
      />
      <NumField label="Mob-VNUM" value={line.vnum} onChange={(v) => onChange({ vnum: v })} width="w-20" />
      <Button variant="ghost" size="icon-sm" className="ml-auto" onClick={onRemove}>
        <Trash2 className="size-3.5" />
      </Button>
    </div>
  );
}
