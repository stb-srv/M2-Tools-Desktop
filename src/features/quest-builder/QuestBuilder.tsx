import { useEffect, useMemo, useRef, useState } from "react";
import { invoke } from "@tauri-apps/api/core";
import { listen } from "@tauri-apps/api/event";
import { WebviewWindow } from "@tauri-apps/api/webviewWindow";
import CodeMirror, { type ReactCodeMirrorRef } from "@uiw/react-codemirror";
import { StreamLanguage } from "@codemirror/language";
import { lua } from "@codemirror/legacy-modes/mode/lua";
import { runAsyncAction } from "@/lib/asyncAction";
import { logActivity } from "@/lib/logActivity";
import { reportSectionDirty, useNavigationStore } from "@/store/navigation";
import { Button } from "@/components/ui/button";
import {
  Search,
  FileSearch,
  Plus,
  Trash2,
  X,
  RefreshCw,
  AlertTriangle,
  CheckCircle2,
  PlayCircle,
  Code2,
  BookOpen,
  HelpCircle,
} from "lucide-react";
import { openManual } from "@/lib/manual";
import QUEST_FUNCTIONS from "./questFunctions.json";
import { CreateQuestModal } from "./components/CreateQuestModal";

interface QuestFile {
  category: string;
  name: string;
  extension: string;
  relative_path: string;
}

interface QuestSearchLine {
  line_number: number;
  text: string;
}

interface QuestSearchMatch {
  relative_path: string;
  category: string;
  name: string;
  lines: QuestSearchLine[];
}

const luaLanguage = StreamLanguage.define(lua);

// The app's dark mode is a `dark` class toggled on <html> (see
// src/store/theme.ts), not a React-visible value the editor can subscribe
// to directly - watch it via MutationObserver so the code editor follows
// theme changes (including "system" switching at the OS level) instead of
// only picking up the theme active on first mount.
function useIsDark(): boolean {
  const [isDark, setIsDark] = useState(() =>
    document.documentElement.classList.contains("dark"),
  );
  useEffect(() => {
    const observer = new MutationObserver(() =>
      setIsDark(document.documentElement.classList.contains("dark")),
    );
    observer.observe(document.documentElement, { attributes: true, attributeFilter: ["class"] });
    return () => observer.disconnect();
  }, []);
  return isDark;
}

export function QuestBuilder() {
  const [files, setFiles] = useState<QuestFile[] | null>(null);
  const [loading, setLoading] = useState(false);
  const [loadError, setLoadError] = useState<string | null>(null);
  const [search, setSearch] = useState("");
  const [contentResults, setContentResults] = useState<QuestSearchMatch[] | null>(null);
  const [contentSearching, setContentSearching] = useState(false);
  const [contentSearchError, setContentSearchError] = useState<string | null>(null);
  const [pendingJumpLine, setPendingJumpLine] = useState<number | null>(null);
  const isDark = useIsDark();

  const [selectedPath, setSelectedPath] = useState<string | null>(null);
  const [content, setContent] = useState("");
  const [contentLoading, setContentLoading] = useState(false);
  const [contentError, setContentError] = useState<string | null>(null);
  const [dirty, setDirty] = useState(false);
  useEffect(() => {
    reportSectionDirty("quest-builder", dirty);
    return () => reportSectionDirty("quest-builder", false);
  }, [dirty]);

  const [saving, setSaving] = useState(false);
  const [saveOk, setSaveOk] = useState<string | null>(null);
  const [saveError, setSaveError] = useState<string | null>(null);
  const [deleteConfirm, setDeleteConfirm] = useState(false);

  const [creating, setCreating] = useState(false);

  const [deploying, setDeploying] = useState(false);
  const [deployLog, setDeployLog] = useState<string[]>([]);
  const [deployOpen, setDeployOpen] = useState(false);

  const [functionRefOpen, setFunctionRefOpen] = useState(false);
  const [functionQuery, setFunctionQuery] = useState("");
  const codeRef = useRef<ReactCodeMirrorRef>(null);

  useEffect(() => {
    loadFiles();
  }, []);

  // Globale Suche (Strg+Umschalt+F) springt hierher mit dem relative_path
  // eines Quest-Treffers.
  useEffect(() => {
    const targetRef = useNavigationStore.getState().consumePendingSelection("quest-builder");
    if (targetRef) openFile(targetRef);
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, []);

  useEffect(() => {
    const unlisten = listen<string>("server-output", (event) => {
      setDeployLog((prev) => [...prev, event.payload]);
    });
    return () => {
      unlisten.then((fn) => fn());
    };
  }, []);

  async function loadFiles() {
    setLoading(true);
    setLoadError(null);
    try {
      const result = await invoke<QuestFile[]>("list_quest_files");
      setFiles(result);
    } catch (e) {
      setLoadError(String(e));
    } finally {
      setLoading(false);
    }
  }

  async function runContentSearch() {
    if (!search.trim()) return;
    await runAsyncAction(
      () => invoke<QuestSearchMatch[]>("search_quest_files", { query: search.trim() }),
      {
        onStart: () => {
          setContentSearching(true);
          setContentSearchError(null);
        },
        onSuccess: setContentResults,
        onError: setContentSearchError,
        onFinally: () => setContentSearching(false),
      },
    );
  }

  async function openFileAtLine(path: string, lineNumber: number) {
    setPendingJumpLine(lineNumber);
    await openFile(path);
  }

  async function openFile(path: string) {
    setSelectedPath(path);
    setContent("");
    setContentError(null);
    setDirty(false);
    setSaveOk(null);
    setSaveError(null);
    setContentLoading(true);
    try {
      const text = await invoke<string>("read_quest_file", { relativePath: path });
      setContent(text);
    } catch (e) {
      setContentError(String(e));
    } finally {
      setContentLoading(false);
    }
  }

  async function saveFile() {
    if (!selectedPath) return;
    setSaving(true);
    setSaveError(null);
    try {
      const backup = await invoke<string | null>("write_quest_file", {
        relativePath: selectedPath,
        content,
      });
      setSaveOk(backup ? `Gespeichert. Backup: ${backup}` : "Gespeichert.");
      setDirty(false);
      logActivity("quest-builder", "update", `Quest '${selectedPath}' gespeichert`, "quest", selectedPath);
    } catch (e) {
      setSaveError(String(e));
    } finally {
      setSaving(false);
    }
  }

  async function confirmDelete() {
    if (!selectedPath) return;
    setDeleteConfirm(false);
    try {
      await invoke("delete_quest_file", { relativePath: selectedPath });
      logActivity("quest-builder", "delete", `Quest '${selectedPath}' gelöscht`, "quest", selectedPath);
      setSelectedPath(null);
      setContent("");
      await loadFiles();
    } catch (e) {
      setSaveError(String(e));
    }
  }

  function insertFunctionAtCursor(name: string) {
    const view = codeRef.current?.view;
    if (!view) return;
    const { from, to } = view.state.selection.main;
    view.dispatch({
      changes: { from, to, insert: name },
      selection: { anchor: from + name.length },
    });
    view.focus();
    setDirty(true);
    setSaveOk(null);
  }

  // Jumps to a line found via the content search once its file has finished
  // loading into the editor - can't do this at click time, since the file
  // usually still needs to be fetched over SFTP first.
  useEffect(() => {
    if (pendingJumpLine == null) return;
    const view = codeRef.current?.view;
    if (!view) return;
    const lineNumber = Math.min(pendingJumpLine, view.state.doc.lines);
    const line = view.state.doc.line(lineNumber);
    view.dispatch({ selection: { anchor: line.from, head: line.to }, scrollIntoView: true });
    view.focus();
    setPendingJumpLine(null);
  }, [content, pendingJumpLine]);

  const filteredFunctions = useMemo(() => {
    const q = functionQuery.trim().toLowerCase();
    const list = q ? QUEST_FUNCTIONS.filter((f) => f.toLowerCase().includes(q)) : QUEST_FUNCTIONS;
    return list.slice(0, 200);
  }, [functionQuery]);

  // Eigenes Fenster statt eines Bereichs im Hauptfenster, damit die Wiki
  // parallel zum Skript-Editor offen bleiben kann - `getByLabel` fokussiert
  // ein bereits offenes Fenster erneut statt ein zweites zu öffnen.
  async function openWiki() {
    const existing = await WebviewWindow.getByLabel("wiki");
    if (existing) {
      await existing.setFocus();
      return;
    }
    new WebviewWindow("wiki", {
      url: "wiki.html",
      title: "Quest-Wiki - M2Manager",
      width: 1100,
      height: 800,
    });
  }

  async function runDeploy() {
    setDeployOpen(true);
    setDeploying(true);
    setDeployLog((prev) => [...prev, "\n[Kompiliere Quests auf dem Server…]\n"]);
    try {
      const workdir =
        (await invoke<string | null>("get_setting", { key: "server_workdir" }).catch(
          () => null,
        )) || "/usr/home/game";
      const command =
        (await invoke<string | null>("get_setting", {
          key: "server_cmd_reload_quests",
        }).catch(() => null)) || `cd ${workdir} && echo 4 | sh index.sh`;
      const result = await invoke<{ output: string; exit_status: number | null }>(
        "run_server_command",
        { command },
      );
      if (result.exit_status !== null && result.exit_status !== 0) {
        setDeployLog((prev) => [...prev, `\n[Beendet mit Code ${result.exit_status}]\n`]);
      }
    } catch (e) {
      setDeployLog((prev) => [...prev, `\nFehler: ${String(e)}\n`]);
    } finally {
      setDeploying(false);
    }
  }

  const categories = useMemo(() => {
    const map = new Map<string, QuestFile[]>();
    for (const f of files ?? []) {
      if (
        search &&
        !f.name.toLowerCase().includes(search.toLowerCase()) &&
        !f.category.toLowerCase().includes(search.toLowerCase())
      ) {
        continue;
      }
      const list = map.get(f.category) ?? [];
      list.push(f);
      map.set(f.category, list);
    }
    return [...map.entries()].sort(([a], [b]) => a.localeCompare(b));
  }, [files, search]);

  if (loading && !files) {
    return <p className="text-sm text-muted-foreground">Lade Quest-Liste…</p>;
  }

  if (loadError && !files) {
    return (
      <div className="max-w-md space-y-3">
        <h1 className="text-2xl font-semibold">Quest Builder</h1>
        <p className="text-sm text-destructive">{loadError}</p>
        <Button onClick={loadFiles}>Erneut versuchen</Button>
      </div>
    );
  }

  return (
    <div className="flex h-full flex-col gap-3">
      <div className="flex items-center justify-between">
        <div className="flex items-center gap-2">
          <h1 className="text-2xl font-semibold">Quest Builder</h1>
          <Button variant="ghost" size="icon-sm" title="Hilfe zu diesem Modul" onClick={() => openManual("quest-builder")}>
            <HelpCircle className="size-4" />
          </Button>
        </div>
        <div className="flex items-center gap-2">
          <Button variant="outline" onClick={openWiki}>
            <BookOpen className="size-4" />
            Wiki öffnen
          </Button>
          <Button variant="outline" onClick={loadFiles} disabled={loading}>
            <RefreshCw className={`size-4 ${loading ? "animate-spin" : ""}`} />
            Neu laden
          </Button>
          <Button onClick={runDeploy} disabled={deploying}>
            <PlayCircle className="size-4" />
            {deploying ? "Kompiliere…" : "Kompilieren & Neuladen"}
          </Button>
        </div>
      </div>
      <p className="text-xs text-muted-foreground">
        Führt denselben Befehl aus wie „Quests reloaden" in der Server-Steuerung
        (<code>make.py</code> kompiliert alle in <code>quest_list</code> eingetragenen Dateien neu).
        Fehler beim Kompilieren erscheinen unten in der Ausgabe.
      </p>

      <div className="flex min-h-0 flex-1 gap-4">
        <div className="flex w-72 shrink-0 flex-col gap-2">
          <div className="flex gap-2">
            <div className="relative flex-1">
              <Search className="absolute left-2 top-1/2 size-3.5 -translate-y-1/2 text-muted-foreground" />
              <input
                value={search}
                onChange={(e) => {
                  setSearch(e.target.value);
                  setContentResults(null);
                }}
                onKeyDown={(e) => e.key === "Enter" && runContentSearch()}
                placeholder="Quest suchen (Name) oder Enter für Volltextsuche…"
                className="w-full rounded-md border border-border bg-background py-1 pl-7 pr-2 text-sm"
              />
            </div>
            <Button
              variant="outline"
              size="icon"
              onClick={runContentSearch}
              disabled={contentSearching || !search.trim()}
              title="Volltextsuche über alle Quest-Dateien"
            >
              <FileSearch className="size-4" />
            </Button>
            <Button variant="outline" size="icon" onClick={() => setCreating(true)}>
              <Plus className="size-4" />
            </Button>
          </div>

          {contentSearching && (
            <p className="text-xs text-muted-foreground">Durchsuche Dateiinhalte…</p>
          )}
          {contentSearchError && <p className="text-xs text-destructive">{contentSearchError}</p>}

          {contentResults && (
            <div className="space-y-1 rounded-md border border-border p-2">
              <div className="flex items-center justify-between">
                <span className="text-xs font-medium text-muted-foreground">
                  {contentResults.length} Datei(en) mit Treffer für „{search}"
                </span>
                <button
                  onClick={() => setContentResults(null)}
                  className="text-muted-foreground hover:text-foreground"
                >
                  <X className="size-3.5" />
                </button>
              </div>
              <div className="max-h-56 space-y-2 overflow-y-auto">
                {contentResults.map((m) => (
                  <div key={m.relative_path}>
                    <div className="text-xs font-medium">
                      {m.category}/{m.name}
                    </div>
                    {m.lines.slice(0, 5).map((l) => (
                      <button
                        key={l.line_number}
                        onClick={() => openFileAtLine(m.relative_path, l.line_number)}
                        className="block w-full truncate rounded px-1.5 py-0.5 text-left font-mono text-[11px] hover:bg-muted"
                        title={l.text}
                      >
                        <span className="text-muted-foreground">{l.line_number}:</span> {l.text.trim()}
                      </button>
                    ))}
                    {m.lines.length > 5 && (
                      <p className="pl-1.5 text-[10px] text-muted-foreground">
                        +{m.lines.length - 5} weitere Treffer
                      </p>
                    )}
                  </div>
                ))}
                {contentResults.length === 0 && (
                  <p className="text-xs text-muted-foreground">Keine Treffer im Dateiinhalt.</p>
                )}
              </div>
            </div>
          )}

          <div className="flex-1 space-y-3 overflow-y-auto">
            {categories.map(([category, list]) => (
              <div key={category} className="space-y-1">
                <div className="px-1 text-xs font-medium text-muted-foreground">
                  {category || "(ohne Kategorie)"}
                </div>
                {list.map((f) => (
                  <div
                    key={f.relative_path}
                    onClick={() => openFile(f.relative_path)}
                    className={`cursor-pointer rounded-md border border-border p-2 text-sm hover:bg-muted ${
                      selectedPath === f.relative_path ? "bg-muted" : ""
                    }`}
                  >
                    <div className="font-medium">{f.name}</div>
                    <div className="text-xs text-muted-foreground">{f.relative_path}</div>
                  </div>
                ))}
              </div>
            ))}
            {categories.length === 0 && (
              <p className="p-2 text-sm text-muted-foreground">Keine Quests gefunden.</p>
            )}
          </div>
        </div>

        <div className="flex min-w-0 flex-1 flex-col gap-2">
          {!selectedPath && (
            <div className="flex flex-1 flex-col items-center justify-center gap-3 rounded-lg border border-dashed border-border text-center">
              <p className="text-sm text-muted-foreground">
                Wähle links eine Quest aus oder lege über „+" eine neue an.
              </p>
            </div>
          )}

          {selectedPath && (
            <>
              <div className="flex items-center justify-between">
                <div className="flex items-center gap-2">
                  <Code2 className="size-4 text-muted-foreground" />
                  <code className="text-sm">{selectedPath}</code>
                </div>
                <div className="flex items-center gap-2">
                  {saveOk && (
                    <span className="flex items-center gap-1 text-sm text-green-600">
                      <CheckCircle2 className="size-4" /> {saveOk}
                    </span>
                  )}
                  <Button
                    variant={functionRefOpen ? "default" : "outline"}
                    size="sm"
                    onClick={() => setFunctionRefOpen((v) => !v)}
                  >
                    <BookOpen className="size-4" />
                    Funktionen
                  </Button>
                  <Button variant="destructive" size="sm" onClick={() => setDeleteConfirm(true)}>
                    <Trash2 className="size-4" />
                    Löschen
                  </Button>
                  <Button size="sm" onClick={saveFile} disabled={saving || !dirty}>
                    {saving ? "Speichere…" : "Speichern"}
                  </Button>
                </div>
              </div>
              {saveError && <p className="text-sm text-destructive">{saveError}</p>}
              {contentError && <p className="text-sm text-destructive">{contentError}</p>}
              {contentLoading ? (
                <p className="text-sm text-muted-foreground">Lade…</p>
              ) : (
                <div className="flex min-h-0 flex-1 gap-2">
                  <div className="min-h-0 min-w-0 flex-1 overflow-hidden rounded-md border border-border">
                    <CodeMirror
                      ref={codeRef}
                      value={content}
                      height="100%"
                      theme={isDark ? "dark" : "light"}
                      extensions={[luaLanguage]}
                      basicSetup={{ foldGutter: true, highlightActiveLine: true }}
                      className="h-full text-xs"
                      onChange={(value) => {
                        setContent(value);
                        setDirty(true);
                        setSaveOk(null);
                      }}
                    />
                  </div>
                  {functionRefOpen && (
                    <div className="flex w-64 shrink-0 flex-col gap-2 rounded-md border border-border p-2">
                      <div className="relative">
                        <Search className="absolute left-2 top-1/2 size-3.5 -translate-y-1/2 text-muted-foreground" />
                        <input
                          autoFocus
                          value={functionQuery}
                          onChange={(e) => setFunctionQuery(e.target.value)}
                          placeholder="Funktion suchen…"
                          className="w-full rounded-md border border-border bg-background py-1 pl-7 pr-2 text-sm"
                        />
                      </div>
                      <p className="text-[10px] text-muted-foreground">
                        {QUEST_FUNCTIONS.length} verifizierte Funktionen vom Server. Klick fügt sie
                        an der Cursor-Position ein.
                      </p>
                      <div className="flex-1 space-y-0.5 overflow-y-auto">
                        {filteredFunctions.map((fn) => (
                          <button
                            key={fn}
                            onClick={() => insertFunctionAtCursor(fn)}
                            className="block w-full truncate rounded px-1.5 py-0.5 text-left font-mono text-xs hover:bg-muted"
                            title={fn}
                          >
                            {fn}
                          </button>
                        ))}
                        {filteredFunctions.length === 0 && (
                          <p className="p-1 text-xs text-muted-foreground">Keine Treffer.</p>
                        )}
                      </div>
                    </div>
                  )}
                </div>
              )}
            </>
          )}
        </div>
      </div>

      {/* Neue Quest */}
      {creating && (
        <CreateQuestModal
          onClose={() => setCreating(false)}
          onCreated={async (path) => {
            await loadFiles();
            await openFile(path);
          }}
        />
      )}

      {deleteConfirm && (
        <div className="fixed inset-0 flex items-center justify-center bg-black/50">
          <div className="w-96 space-y-3 rounded-lg border border-border bg-card p-4">
            <p className="flex items-start gap-2 text-sm">
              <AlertTriangle className="mt-0.5 size-4 shrink-0 text-destructive" />
              <span>
                <code>{selectedPath}</code> wird aus <code>quest_list</code> entfernt und die
                Quelldatei ins Backup-Verzeichnis verschoben (kein endgültiges Löschen).
              </span>
            </p>
            <div className="flex justify-end gap-2">
              <Button variant="outline" onClick={() => setDeleteConfirm(false)}>
                Abbrechen
              </Button>
              <Button variant="destructive" onClick={confirmDelete}>
                Entfernen
              </Button>
            </div>
          </div>
        </div>
      )}

      {deployOpen && (
        <div className="fixed inset-0 flex items-center justify-center bg-black/50">
          <div className="flex max-h-[70vh] w-[36rem] flex-col gap-3 rounded-lg border border-border bg-card p-4">
            <div className="flex items-center justify-between">
              <p className="text-sm font-medium">Quests kompilieren & neu laden</p>
              <button
                onClick={() => setDeployOpen(false)}
                className="text-muted-foreground hover:text-foreground"
              >
                <X className="size-4" />
              </button>
            </div>
            <pre className="min-h-0 flex-1 overflow-auto whitespace-pre-wrap rounded-md border border-border bg-muted/30 p-2 font-mono text-xs">
              {deployLog.length ? deployLog.join("") : "…"}
            </pre>
          </div>
        </div>
      )}
    </div>
  );
}
