import { useEffect, useMemo, useRef, useState } from "react";
import { invoke } from "@tauri-apps/api/core";
import { listen } from "@tauri-apps/api/event";
import CodeMirror, { type ReactCodeMirrorRef } from "@uiw/react-codemirror";
import { StreamLanguage } from "@codemirror/language";
import { lua } from "@codemirror/legacy-modes/mode/lua";
import { runAsyncAction } from "@/lib/asyncAction";
import { reportSectionDirty } from "@/store/navigation";
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
  Wand2,
  BookOpen,
} from "lucide-react";
import {
  DEFAULT_FORM,
  DEFAULT_DUNGEON_FORM,
  DEFAULT_FLOOR,
  generateQuestLua,
  generateDungeonQuest,
  TEMPLATE_HINTS,
  TEMPLATE_LABELS,
  type QuestFormState,
  type DungeonFormState,
  type TemplateType,
} from "./questTemplates";
import QUEST_FUNCTIONS from "./questFunctions.json";

interface QuestFile {
  category: string;
  name: string;
  extension: string;
  relative_path: string;
}

interface PickResult {
  vnum: number;
  name: string;
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

// NPCs and monsters are the same `mob_proto` table on this server (see
// db/shop.rs::list_shops, which joins it for shop NPC names) - "npc"/"mob"
// both search it, only "item" searches item_proto.
type PickerKind = "npc" | "mob" | "item";

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
  const [newCategory, setNewCategory] = useState("");
  const [newCategoryPreview, setNewCategoryPreview] = useState("");
  const [newName, setNewName] = useState("");
  const [newNamePreview, setNewNamePreview] = useState("");
  const [templateType, setTemplateType] = useState<TemplateType>("dialog");
  const [form, setForm] = useState<QuestFormState>(DEFAULT_FORM);
  const [dungeonForm, setDungeonForm] = useState<DungeonFormState>(DEFAULT_DUNGEON_FORM);
  const [createError, setCreateError] = useState<string | null>(null);
  const [creatingBusy, setCreatingBusy] = useState(false);

  const [picker, setPicker] = useState<{ kind: PickerKind; onPick: (vnum: number) => void } | null>(
    null,
  );
  const [pickerQuery, setPickerQuery] = useState("");
  const [pickerResults, setPickerResults] = useState<PickResult[]>([]);
  const [pickerSearching, setPickerSearching] = useState(false);
  const [pickerError, setPickerError] = useState<string | null>(null);

  const [deploying, setDeploying] = useState(false);
  const [deployLog, setDeployLog] = useState<string[]>([]);
  const [deployOpen, setDeployOpen] = useState(false);

  const [functionRefOpen, setFunctionRefOpen] = useState(false);
  const [functionQuery, setFunctionQuery] = useState("");
  const codeRef = useRef<ReactCodeMirrorRef>(null);

  useEffect(() => {
    loadFiles();
  }, []);

  useEffect(() => {
    const unlisten = listen<string>("server-output", (event) => {
      setDeployLog((prev) => [...prev, event.payload]);
    });
    return () => {
      unlisten.then((fn) => fn());
    };
  }, []);

  useEffect(() => {
    if (!creating) return;
    if (!newCategory.trim()) {
      setNewCategoryPreview("");
    } else {
      invoke<string>("sanitize_quest_identifier", { name: newCategory })
        .then(setNewCategoryPreview)
        .catch(() => setNewCategoryPreview(""));
    }
    if (!newName.trim()) {
      setNewNamePreview("");
    } else {
      invoke<string>("sanitize_quest_identifier", { name: newName })
        .then(setNewNamePreview)
        .catch(() => setNewNamePreview(""));
    }
  }, [newCategory, newName, creating]);

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
      setSelectedPath(null);
      setContent("");
      await loadFiles();
    } catch (e) {
      setSaveError(String(e));
    }
  }

  function updateForm(patch: Partial<QuestFormState>) {
    setForm((prev) => ({ ...prev, ...patch }));
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

  function updateDungeonForm(patch: Partial<DungeonFormState>) {
    setDungeonForm((prev) => ({ ...prev, ...patch }));
  }

  function updateFloor(index: number, patch: Partial<(typeof dungeonForm.floors)[number]>) {
    setDungeonForm((prev) => ({
      ...prev,
      floors: prev.floors.map((f, i) => (i === index ? { ...f, ...patch } : f)),
    }));
  }

  function addFloor() {
    setDungeonForm((prev) => ({ ...prev, floors: [...prev.floors, { ...DEFAULT_FLOOR }] }));
  }

  function removeFloor(index: number) {
    setDungeonForm((prev) => ({
      ...prev,
      floors: prev.floors.length > 1 ? prev.floors.filter((_, i) => i !== index) : prev.floors,
    }));
  }

  const preview = useMemo(() => {
    const name = newNamePreview || "neue_quest";
    try {
      return templateType === "dungeon"
        ? generateDungeonQuest(name, dungeonForm)
        : generateQuestLua(name, templateType, form);
    } catch {
      return "";
    }
  }, [newNamePreview, templateType, form, dungeonForm]);

  async function submitCreate() {
    if (!newCategoryPreview || !newNamePreview) return;
    setCreatingBusy(true);
    setCreateError(null);
    try {
      const path = await invoke<string>("create_quest_file", {
        category: newCategoryPreview,
        name: newNamePreview,
        content: preview,
        extension: templateType === "dungeon" ? "quest" : "lua",
      });
      setCreating(false);
      setNewCategory("");
      setNewName("");
      setForm(DEFAULT_FORM);
      setDungeonForm(DEFAULT_DUNGEON_FORM);
      await loadFiles();
      await openFile(path);
    } catch (e) {
      setCreateError(String(e));
    } finally {
      setCreatingBusy(false);
    }
  }

  function openPicker(kind: PickerKind, onPick: (vnum: number) => void) {
    setPicker({ kind, onPick });
    setPickerQuery("");
    setPickerResults([]);
    setPickerError(null);
  }

  async function runPickerSearch() {
    if (!picker || !pickerQuery.trim()) return;
    const command = picker.kind === "item" ? "search_items" : "search_mobs";
    await runAsyncAction(() => invoke<PickResult[]>(command, { query: pickerQuery.trim() }), {
      onStart: () => {
        setPickerSearching(true);
        setPickerError(null);
      },
      onSuccess: setPickerResults,
      onError: setPickerError,
      onFinally: () => setPickerSearching(false),
    });
  }

  function pickValue(vnum: number) {
    if (!picker) return;
    picker.onPick(vnum);
    setPicker(null);
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
        <h1 className="text-2xl font-semibold">Quest Builder</h1>
        <div className="flex items-center gap-2">
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
        <Modal onClose={() => setCreating(false)} wide>
          <div className="flex max-h-[80vh] gap-4">
            <div className="w-80 shrink-0 space-y-3 overflow-y-auto pr-2">
              <p className="text-sm font-medium">Neue Quest anlegen</p>
              <div className="flex gap-2">
                <Field label="Kategorie (Ordner)">
                  <input
                    value={newCategory}
                    onChange={(e) => setNewCategory(e.target.value)}
                    placeholder="z.B. Biologie"
                    className="w-36 rounded-md border border-border bg-background px-2 py-1 text-sm"
                  />
                </Field>
                <Field label="Name">
                  <input
                    value={newName}
                    onChange={(e) => setNewName(e.target.value)}
                    placeholder="z.B. Orkzahn"
                    className="w-36 rounded-md border border-border bg-background px-2 py-1 text-sm"
                  />
                </Field>
              </div>
              {(newCategory.trim() || newName.trim()) && (
                <p className="text-xs text-muted-foreground">
                  Wird gespeichert als:{" "}
                  <code>
                    {newCategoryPreview || "…"}/{newNamePreview || "…"}.
                    {templateType === "dungeon" ? "quest" : "lua"}
                  </code>
                </p>
              )}

              <Field label="Vorlage">
                <select
                  value={templateType}
                  onChange={(e) => setTemplateType(e.target.value as TemplateType)}
                  className="w-full rounded-md border border-border bg-background px-2 py-1 text-sm"
                >
                  {Object.entries(TEMPLATE_LABELS).map(([value, label]) => (
                    <option key={value} value={value}>
                      {label}
                    </option>
                  ))}
                </select>
              </Field>
              <p className="text-xs text-muted-foreground">{TEMPLATE_HINTS[templateType]}</p>

              {templateType !== "dungeon" && (
                <>
                  <Field label="NPC-VNUM">
                    <VnumInput
                      value={form.npcVnum}
                      onChange={(v) => updateForm({ npcVnum: v })}
                      onPick={() => openPicker("npc", (v) => updateForm({ npcVnum: String(v) }))}
                    />
                  </Field>
                  <Field label="Dialog-Auslöser (chat-Label)">
                    <input
                      value={form.chatLabel}
                      onChange={(e) => updateForm({ chatLabel: e.target.value })}
                      className="w-full rounded-md border border-border bg-background px-2 py-1 text-sm"
                    />
                  </Field>
                  <Field label="NPC-Titel (say_title)">
                    <input
                      value={form.npcTitle}
                      onChange={(e) => updateForm({ npcTitle: e.target.value })}
                      className="w-full rounded-md border border-border bg-background px-2 py-1 text-sm"
                    />
                  </Field>
                </>
              )}

              {templateType === "dialog" && (
                <Field label="Dialogtext">
                  <textarea
                    value={form.dialogText}
                    onChange={(e) => updateForm({ dialogText: e.target.value })}
                    className="h-20 w-full rounded-md border border-border bg-background px-2 py-1 text-sm"
                  />
                </Field>
              )}

              {templateType === "collect" && (
                <>
                  <div className="flex gap-2">
                    <Field label="Item-VNUM (abzugeben)">
                      <VnumInput
                        value={form.requiredItemVnum}
                        onChange={(v) => updateForm({ requiredItemVnum: v })}
                        onPick={() =>
                          openPicker("item", (v) => updateForm({ requiredItemVnum: String(v) }))
                        }
                      />
                    </Field>
                    <Field label="Anzahl">
                      <input
                        type="number"
                        min={1}
                        value={form.requiredItemCount}
                        onChange={(e) => updateForm({ requiredItemCount: e.target.value })}
                        className="w-20 rounded-md border border-border bg-background px-2 py-1 text-sm"
                      />
                    </Field>
                  </div>
                  <Field label="Text bei Erfolg">
                    <textarea
                      value={form.successText}
                      onChange={(e) => updateForm({ successText: e.target.value })}
                      className="h-16 w-full rounded-md border border-border bg-background px-2 py-1 text-sm"
                    />
                  </Field>
                  <Field label="Text wenn noch nicht genug Items">
                    <textarea
                      value={form.failText}
                      onChange={(e) => updateForm({ failText: e.target.value })}
                      className="h-16 w-full rounded-md border border-border bg-background px-2 py-1 text-sm"
                    />
                  </Field>
                </>
              )}

              {templateType === "chance_collect" && (
                <>
                  <div className="flex gap-2">
                    <Field label="Item-VNUM (abzugeben)">
                      <VnumInput
                        value={form.requiredItemVnum}
                        onChange={(v) => updateForm({ requiredItemVnum: v })}
                        onPick={() =>
                          openPicker("item", (v) => updateForm({ requiredItemVnum: String(v) }))
                        }
                      />
                    </Field>
                    <Field label="Anzahl pro Versuch">
                      <input
                        type="number"
                        min={1}
                        value={form.itemsPerAttempt}
                        onChange={(e) => updateForm({ itemsPerAttempt: e.target.value })}
                        className="w-20 rounded-md border border-border bg-background px-2 py-1 text-sm"
                      />
                    </Field>
                  </div>
                  <div className="flex gap-2">
                    <Field label="Chance der Annahme (%)">
                      <input
                        type="number"
                        min={0}
                        max={100}
                        value={form.acceptChancePercent}
                        onChange={(e) => updateForm({ acceptChancePercent: e.target.value })}
                        className="w-24 rounded-md border border-border bg-background px-2 py-1 text-sm"
                      />
                    </Field>
                    <Field label="Nötige Erfolge bis Belohnung">
                      <input
                        type="number"
                        min={1}
                        value={form.requiredSuccesses}
                        onChange={(e) => updateForm({ requiredSuccesses: e.target.value })}
                        className="w-24 rounded-md border border-border bg-background px-2 py-1 text-sm"
                      />
                    </Field>
                  </div>
                  <p className="text-xs text-muted-foreground">
                    Das Item wird bei jedem Versuch verbraucht - unabhängig davon, ob der Wurf
                    erfolgreich war (genau wie in Biologie/Orkzahn.lua auf dem Server).
                  </p>
                  <Field label="Text bei Erfolg (%d = aktueller Fortschritt, %d = Ziel)">
                    <input
                      value={form.progressText}
                      onChange={(e) => updateForm({ progressText: e.target.value })}
                      className="w-full rounded-md border border-border bg-background px-2 py-1 text-sm"
                    />
                  </Field>
                  <Field label="Text bei Fehlschlag">
                    <textarea
                      value={form.failText}
                      onChange={(e) => updateForm({ failText: e.target.value })}
                      className="h-16 w-full rounded-md border border-border bg-background px-2 py-1 text-sm"
                    />
                  </Field>
                  <Field label="Text wenn Item nicht vorhanden">
                    <textarea
                      value={form.noItemText}
                      onChange={(e) => updateForm({ noItemText: e.target.value })}
                      className="h-16 w-full rounded-md border border-border bg-background px-2 py-1 text-sm"
                    />
                  </Field>
                  <Field label="Text bei Abschluss (genug Erfolge erreicht)">
                    <textarea
                      value={form.completeText}
                      onChange={(e) => updateForm({ completeText: e.target.value })}
                      className="h-16 w-full rounded-md border border-border bg-background px-2 py-1 text-sm"
                    />
                  </Field>
                </>
              )}

              {templateType === "kill" && (
                <>
                  <div className="flex gap-2">
                    <Field label="Monster-VNUM">
                      <VnumInput
                        value={form.mobVnum}
                        onChange={(v) => updateForm({ mobVnum: v })}
                        onPick={() => openPicker("mob", (v) => updateForm({ mobVnum: String(v) }))}
                      />
                    </Field>
                    <Field label="Anzahl zu töten">
                      <input
                        type="number"
                        min={1}
                        value={form.requiredKills}
                        onChange={(e) => updateForm({ requiredKills: e.target.value })}
                        className="w-24 rounded-md border border-border bg-background px-2 py-1 text-sm"
                      />
                    </Field>
                  </div>
                  <Field label="Fortschrittstext (%d = aktuell, %d = Ziel)">
                    <input
                      value={form.progressText}
                      onChange={(e) => updateForm({ progressText: e.target.value })}
                      className="w-full rounded-md border border-border bg-background px-2 py-1 text-sm"
                    />
                  </Field>
                  <Field label="Text bei Erfolg">
                    <textarea
                      value={form.successText}
                      onChange={(e) => updateForm({ successText: e.target.value })}
                      className="h-16 w-full rounded-md border border-border bg-background px-2 py-1 text-sm"
                    />
                  </Field>
                </>
              )}

              {templateType === "use" && (
                <>
                  <Field label="Item-VNUM (zu benutzen)">
                    <VnumInput
                      value={form.useItemVnum}
                      onChange={(v) => updateForm({ useItemVnum: v })}
                      onPick={() =>
                        openPicker("item", (v) => updateForm({ useItemVnum: String(v) }))
                      }
                    />
                  </Field>
                  <Field label="Text beim Benutzen">
                    <textarea
                      value={form.useText}
                      onChange={(e) => updateForm({ useText: e.target.value })}
                      className="h-16 w-full rounded-md border border-border bg-background px-2 py-1 text-sm"
                    />
                  </Field>
                </>
              )}

              {templateType !== "dungeon" && (
                <>
                  <div className="flex gap-2">
                    <Field label="Belohnung Item-VNUM (optional)">
                      <VnumInput
                        value={form.rewardItemVnum}
                        onChange={(v) => updateForm({ rewardItemVnum: v })}
                        onPick={() =>
                          openPicker("item", (v) => updateForm({ rewardItemVnum: String(v) }))
                        }
                      />
                    </Field>
                    <Field label="Anzahl">
                      <input
                        type="number"
                        min={1}
                        value={form.rewardItemCount}
                        onChange={(e) => updateForm({ rewardItemCount: e.target.value })}
                        className="w-20 rounded-md border border-border bg-background px-2 py-1 text-sm"
                      />
                    </Field>
                  </div>
                  <Field label="Belohnung Yang (optional)">
                    <input
                      type="number"
                      value={form.rewardMoney}
                      onChange={(e) => updateForm({ rewardMoney: e.target.value })}
                      className="w-32 rounded-md border border-border bg-background px-2 py-1 text-sm"
                    />
                  </Field>
                </>
              )}

              {templateType === "dungeon" && (
                <>
                  <Field label="Einstiegs-NPC-VNUM">
                    <VnumInput
                      value={dungeonForm.entryNpcVnum}
                      onChange={(v) => updateDungeonForm({ entryNpcVnum: v })}
                      onPick={() =>
                        openPicker("npc", (v) => updateDungeonForm({ entryNpcVnum: String(v) }))
                      }
                    />
                  </Field>
                  <Field label="Dialog-Auslöser (chat-Label)">
                    <input
                      value={dungeonForm.entryChatLabel}
                      onChange={(e) => updateDungeonForm({ entryChatLabel: e.target.value })}
                      className="w-full rounded-md border border-border bg-background px-2 py-1 text-sm"
                    />
                  </Field>
                  <Field label="NPC-Titel (say_title)">
                    <input
                      value={dungeonForm.entryTitle}
                      onChange={(e) => updateDungeonForm({ entryTitle: e.target.value })}
                      className="w-full rounded-md border border-border bg-background px-2 py-1 text-sm"
                    />
                  </Field>
                  <Field label="Einstiegstext">
                    <textarea
                      value={dungeonForm.entryText}
                      onChange={(e) => updateDungeonForm({ entryText: e.target.value })}
                      className="h-16 w-full rounded-md border border-border bg-background px-2 py-1 text-sm"
                    />
                  </Field>
                  <div className="flex gap-2">
                    <Field label="Mindest-Level (0 = keins)">
                      <input
                        type="number"
                        min={0}
                        value={dungeonForm.requiredLevel}
                        onChange={(e) => updateDungeonForm({ requiredLevel: e.target.value })}
                        className="w-24 rounded-md border border-border bg-background px-2 py-1 text-sm"
                      />
                    </Field>
                    <Field label="Dungeon-Map-Index">
                      <input
                        type="number"
                        value={dungeonForm.dungeonMapIndex}
                        onChange={(e) => updateDungeonForm({ dungeonMapIndex: e.target.value })}
                        className="w-28 rounded-md border border-border bg-background px-2 py-1 text-sm"
                      />
                    </Field>
                  </div>
                  <div className="flex gap-2">
                    <Field label="Map-Index-Bereich Start">
                      <input
                        type="number"
                        value={dungeonForm.mapIndexRangeStart}
                        onChange={(e) =>
                          updateDungeonForm({ mapIndexRangeStart: e.target.value })
                        }
                        className="w-28 rounded-md border border-border bg-background px-2 py-1 text-sm"
                      />
                    </Field>
                    <Field label="…bis (exklusiv)">
                      <input
                        type="number"
                        value={dungeonForm.mapIndexRangeEnd}
                        onChange={(e) => updateDungeonForm({ mapIndexRangeEnd: e.target.value })}
                        className="w-28 rounded-md border border-border bg-background px-2 py-1 text-sm"
                      />
                    </Field>
                  </div>
                  <p className="text-xs text-muted-foreground">
                    Der Map-Index-Bereich ist die vom Server für diesen Dungeon-Typ vergebene
                    Instanz-Map-Range (z.B. 660000–670000 beim Daemonenturm) - kommt aus der
                    Server-Dungeon-Konfiguration, nicht aus diesem Tool. Ohne den richtigen Bereich
                    lösen die Boss-Kill-Trigger unten nicht korrekt aus.
                  </p>

                  <div className="space-y-2 rounded-md border border-border p-2">
                    <div className="flex items-center justify-between">
                      <span className="text-xs font-medium text-muted-foreground">
                        Etagen ({dungeonForm.floors.length})
                      </span>
                      <Button variant="outline" size="sm" onClick={addFloor}>
                        <Plus className="size-3.5" />
                        Etage
                      </Button>
                    </div>
                    {dungeonForm.floors.map((floor, index) => (
                      <div key={index} className="space-y-1 rounded-md border border-border p-2">
                        <div className="flex items-center justify-between">
                          <span className="text-xs font-medium">Etage {index + 1}</span>
                          {dungeonForm.floors.length > 1 && (
                            <Button
                              variant="ghost"
                              size="icon-sm"
                              onClick={() => removeFloor(index)}
                            >
                              <X className="size-3.5" />
                            </Button>
                          )}
                        </div>
                        <Field label="Boss-/Trigger-Monster-VNUM (löst nächste Etage aus)">
                          <VnumInput
                            value={floor.bossVnum}
                            onChange={(v) => updateFloor(index, { bossVnum: v })}
                            onPick={() =>
                              openPicker("mob", (v) => updateFloor(index, { bossVnum: String(v) }))
                            }
                          />
                        </Field>
                        <div className="flex gap-2">
                          <Field label="Einstiegs-X">
                            <input
                              type="number"
                              value={floor.entryX}
                              onChange={(e) => updateFloor(index, { entryX: e.target.value })}
                              className="w-20 rounded-md border border-border bg-background px-2 py-1 text-sm"
                            />
                          </Field>
                          <Field label="Einstiegs-Y">
                            <input
                              type="number"
                              value={floor.entryY}
                              onChange={(e) => updateFloor(index, { entryY: e.target.value })}
                              className="w-20 rounded-md border border-border bg-background px-2 py-1 text-sm"
                            />
                          </Field>
                        </div>
                        <Field label="Regen-Datei (Monster-Spawns dieser Etage)">
                          <input
                            value={floor.regenFile}
                            onChange={(e) => updateFloor(index, { regenFile: e.target.value })}
                            placeholder="data/dungeon/meindungeon/etage1_regen.txt"
                            className="w-full rounded-md border border-border bg-background px-2 py-1 text-sm"
                          />
                        </Field>
                      </div>
                    ))}
                    <p className="text-xs text-muted-foreground">
                      Die Regen-Datei muss auf dem Server bereits existieren (eigenes Dateiformat,
                      hier nicht erzeugt) - für „Etage 1" wird sie beim Betreten geladen, für jede
                      weitere Etage beim Töten des Bosses der vorherigen Etage.
                    </p>
                  </div>

                  <div className="flex gap-2">
                    <Field label="Belohnung Item-VNUM (optional, letzte Etage)">
                      <VnumInput
                        value={dungeonForm.rewardItemVnum}
                        onChange={(v) => updateDungeonForm({ rewardItemVnum: v })}
                        onPick={() =>
                          openPicker("item", (v) =>
                            updateDungeonForm({ rewardItemVnum: String(v) }),
                          )
                        }
                      />
                    </Field>
                    <Field label="Anzahl">
                      <input
                        type="number"
                        min={1}
                        value={dungeonForm.rewardItemCount}
                        onChange={(e) =>
                          updateDungeonForm({ rewardItemCount: e.target.value })
                        }
                        className="w-20 rounded-md border border-border bg-background px-2 py-1 text-sm"
                      />
                    </Field>
                  </div>
                  <div className="flex gap-2">
                    <Field label="Belohnung Yang (optional)">
                      <input
                        type="number"
                        value={dungeonForm.rewardMoney}
                        onChange={(e) => updateDungeonForm({ rewardMoney: e.target.value })}
                        className="w-32 rounded-md border border-border bg-background px-2 py-1 text-sm"
                      />
                    </Field>
                    <Field label="Sekunden bis Rauswurf nach Abschluss">
                      <input
                        type="number"
                        min={0}
                        value={dungeonForm.exitDelaySeconds}
                        onChange={(e) =>
                          updateDungeonForm({ exitDelaySeconds: e.target.value })
                        }
                        className="w-24 rounded-md border border-border bg-background px-2 py-1 text-sm"
                      />
                    </Field>
                  </div>
                </>
              )}

              {createError && <p className="text-sm text-destructive">{createError}</p>}
              <Button
                onClick={submitCreate}
                disabled={!newCategoryPreview || !newNamePreview || creatingBusy}
              >
                <Wand2 className="size-4" />
                {creatingBusy ? "Lege an…" : "Anlegen"}
              </Button>
            </div>

            <div className="flex min-w-0 flex-1 flex-col gap-1">
              <span className="text-xs font-medium text-muted-foreground">
                Vorschau (nach dem Anlegen als Code editierbar)
              </span>
              <pre className="min-h-0 flex-1 overflow-auto rounded-md border border-border bg-muted/30 p-2 font-mono text-xs">
                {preview}
              </pre>
            </div>
          </div>
        </Modal>
      )}

      {/* NPC/Mob/Item-Picker */}
      {picker && (
        <Modal onClose={() => setPicker(null)}>
          <div className="space-y-2">
            <div className="flex gap-2">
              <input
                autoFocus
                value={pickerQuery}
                onChange={(e) => setPickerQuery(e.target.value)}
                onKeyDown={(e) => e.key === "Enter" && runPickerSearch()}
                placeholder={
                  picker.kind === "mob"
                    ? "Monster nach Name oder VNUM suchen…"
                    : picker.kind === "npc"
                      ? "NPC nach Name oder VNUM suchen (mob_proto)…"
                      : "Item nach Name oder VNUM suchen…"
                }
                className="flex-1 rounded-md border border-border bg-background px-2 py-1 text-sm"
              />
              <Button variant="outline" onClick={runPickerSearch} disabled={pickerSearching}>
                <Search className="size-4" />
              </Button>
            </div>
            {pickerError && (
              <p className="flex items-start gap-2 text-sm text-destructive">
                <AlertTriangle className="mt-0.5 size-4 shrink-0" />
                <span>{pickerError}</span>
              </p>
            )}
            <div className="max-h-64 space-y-1 overflow-y-auto">
              {pickerResults.map((r) => (
                <div
                  key={r.vnum}
                  className="flex items-center justify-between rounded-md px-2 py-1 text-sm hover:bg-muted"
                >
                  <span>
                    {r.name} <span className="text-muted-foreground">#{r.vnum}</span>
                  </span>
                  <Button size="sm" onClick={() => pickValue(r.vnum)}>
                    Übernehmen
                  </Button>
                </div>
              ))}
              {pickerResults.length === 0 && !pickerSearching && !pickerError && (
                <p className="p-2 text-sm text-muted-foreground">Noch keine Suche.</p>
              )}
            </div>
          </div>
        </Modal>
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

function Field({ label, children }: { label: string; children: React.ReactNode }) {
  return (
    <label className="flex flex-col gap-1 text-xs text-muted-foreground">
      {label}
      {children}
    </label>
  );
}

function VnumInput({
  value,
  onChange,
  onPick,
}: {
  value: string;
  onChange: (value: string) => void;
  onPick: () => void;
}) {
  return (
    <div className="flex gap-1">
      <input
        type="number"
        value={value}
        onChange={(e) => onChange(e.target.value)}
        className="w-24 rounded-md border border-border bg-background px-2 py-1 text-sm"
      />
      <Button variant="outline" size="icon-sm" onClick={onPick} title="Suchen…">
        <Search className="size-3.5" />
      </Button>
    </div>
  );
}

function Modal({
  children,
  onClose,
  wide,
}: {
  children: React.ReactNode;
  onClose: () => void;
  wide?: boolean;
}) {
  return (
    <div className="fixed inset-0 flex items-center justify-center bg-black/50">
      <div className={`rounded-lg border border-border bg-card p-4 ${wide ? "w-[48rem]" : "w-96"}`}>
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
