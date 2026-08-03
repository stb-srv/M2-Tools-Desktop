import { useEffect, useState } from "react";
import { invoke } from "@tauri-apps/api/core";
import { runAsyncAction } from "@/lib/asyncAction";
import { Button } from "@/components/ui/button";
import { Search, Plus, Trash2, X, RefreshCw, CheckCircle2, AlertTriangle } from "lucide-react";

interface LocaleEntry {
  key: string;
  value: string;
}

// "[ENTER]" is the game's own literal in-game line-break marker (same
// convention as questTemplates.ts::luaString) - shown as real newlines in
// the textarea and converted back on save.
function toDisplay(value: string) {
  return value.replace(/\[ENTER\]/g, "\n");
}
function toStored(display: string) {
  return display.replace(/\r\n|\n/g, "[ENTER]");
}

export function LocaleEditor() {
  const [namespaces, setNamespaces] = useState<string[] | null>(null);
  const [nsLoading, setNsLoading] = useState(false);
  const [nsError, setNsError] = useState<string | null>(null);
  const [search, setSearch] = useState("");

  const [selected, setSelected] = useState<string | null>(null);
  const [entries, setEntries] = useState<LocaleEntry[] | null>(null);
  const [entriesLoading, setEntriesLoading] = useState(false);
  const [entriesError, setEntriesError] = useState<string | null>(null);
  const [dirty, setDirty] = useState(false);

  const [saving, setSaving] = useState(false);
  const [saveOk, setSaveOk] = useState<string | null>(null);
  const [saveError, setSaveError] = useState<string | null>(null);
  const [saveConfirm, setSaveConfirm] = useState(false);

  const [newKey, setNewKey] = useState("");

  const [creating, setCreating] = useState(false);
  const [newNsName, setNewNsName] = useState("");
  const [newNsPreview, setNewNsPreview] = useState("");
  const [createError, setCreateError] = useState<string | null>(null);
  const [creatingBusy, setCreatingBusy] = useState(false);

  useEffect(() => {
    loadNamespaces();
  }, []);

  useEffect(() => {
    if (!creating || !newNsName.trim()) {
      setNewNsPreview("");
      return;
    }
    invoke<string>("sanitize_locale_namespace", { name: newNsName })
      .then(setNewNsPreview)
      .catch(() => setNewNsPreview(""));
  }, [newNsName, creating]);

  async function loadNamespaces() {
    await runAsyncAction(() => invoke<string[]>("list_locale_namespaces"), {
      onStart: () => {
        setNsLoading(true);
        setNsError(null);
      },
      onSuccess: setNamespaces,
      onError: setNsError,
      onFinally: () => setNsLoading(false),
    });
  }

  async function openNamespace(name: string) {
    setSelected(name);
    setEntries(null);
    setEntriesError(null);
    setDirty(false);
    setSaveOk(null);
    setSaveError(null);
    await runAsyncAction(() => invoke<LocaleEntry[]>("read_locale_namespace", { namespace: name }), {
      onStart: () => setEntriesLoading(true),
      onSuccess: setEntries,
      onError: setEntriesError,
      onFinally: () => setEntriesLoading(false),
    });
  }

  function updateEntry(index: number, patch: Partial<LocaleEntry>) {
    setEntries((prev) => prev!.map((e, i) => (i === index ? { ...e, ...patch } : e)));
    setDirty(true);
    setSaveOk(null);
  }

  function removeEntry(index: number) {
    setEntries((prev) => prev!.filter((_, i) => i !== index));
    setDirty(true);
    setSaveOk(null);
  }

  function addEntry() {
    const key = newKey.trim();
    if (!key) return;
    setEntries((prev) => [...(prev ?? []), { key, value: "" }]);
    setNewKey("");
    setDirty(true);
    setSaveOk(null);
  }

  async function saveNamespace() {
    if (!selected || !entries) return;
    setSaveConfirm(false);
    await runAsyncAction(
      () => invoke<string | null>("write_locale_namespace", { namespace: selected, entries }),
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

  async function submitCreate() {
    if (!newNsPreview) return;
    setCreatingBusy(true);
    setCreateError(null);
    try {
      await invoke("create_locale_namespace", { namespace: newNsPreview });
      setCreating(false);
      setNewNsName("");
      await loadNamespaces();
      await openNamespace(newNsPreview);
    } catch (e) {
      setCreateError(String(e));
    } finally {
      setCreatingBusy(false);
    }
  }

  const filtered = (namespaces ?? []).filter((n) =>
    n.toLowerCase().includes(search.toLowerCase()),
  );

  return (
    <div className="flex h-full flex-col gap-3">
      <div>
        <h1 className="text-2xl font-semibold">Locale-String-Verwaltung</h1>
        <p className="text-sm text-muted-foreground">
          Bearbeitet <code>share/translate.lua</code> - die Textbausteine, die Quests über{" "}
          <code>gameforge.questname._key</code> referenzieren. Speichern ändert nur den
          ausgewählten Namespace, der Rest der ~8800-Zeilen-Datei bleibt unangetastet.
        </p>
      </div>

      <div className="flex min-h-0 flex-1 gap-4">
        <div className="flex w-72 shrink-0 flex-col gap-2">
          <div className="flex gap-2">
            <div className="relative flex-1">
              <Search className="absolute left-2 top-1/2 size-3.5 -translate-y-1/2 text-muted-foreground" />
              <input
                value={search}
                onChange={(e) => setSearch(e.target.value)}
                placeholder="Namespace suchen…"
                className="w-full rounded-md border border-border bg-background py-1 pl-7 pr-2 text-sm"
              />
            </div>
            <Button variant="outline" size="icon" onClick={() => setCreating(true)}>
              <Plus className="size-4" />
            </Button>
            <Button variant="outline" size="icon" onClick={loadNamespaces} disabled={nsLoading}>
              <RefreshCw className={`size-4 ${nsLoading ? "animate-spin" : ""}`} />
            </Button>
          </div>
          {nsError && <p className="text-sm text-destructive">{nsError}</p>}
          <div className="flex-1 space-y-1 overflow-y-auto">
            {filtered.map((name) => (
              <button
                key={name}
                onClick={() => openNamespace(name)}
                className={`block w-full truncate rounded-md px-2 py-1 text-left text-sm hover:bg-muted ${
                  selected === name ? "bg-muted" : ""
                }`}
              >
                gameforge.{name}
              </button>
            ))}
            {namespaces && filtered.length === 0 && (
              <p className="p-2 text-sm text-muted-foreground">Keine Namespaces gefunden.</p>
            )}
          </div>
        </div>

        <div className="flex min-w-0 flex-1 flex-col gap-2">
          {!selected && (
            <div className="flex flex-1 flex-col items-center justify-center rounded-lg border border-dashed border-border">
              <p className="text-sm text-muted-foreground">
                Wähle links einen Namespace aus oder lege über „+" einen neuen an.
              </p>
            </div>
          )}

          {selected && (
            <>
              <div className="flex items-center justify-between">
                <code className="text-sm">gameforge.{selected}</code>
                <div className="flex items-center gap-2">
                  {saveOk && (
                    <span className="flex items-center gap-1 text-sm text-green-600">
                      <CheckCircle2 className="size-4" /> {saveOk}
                    </span>
                  )}
                  <Button size="sm" onClick={() => setSaveConfirm(true)} disabled={saving || !dirty}>
                    {saving ? "Speichere…" : "Speichern"}
                  </Button>
                </div>
              </div>
              {saveError && <p className="text-sm text-destructive">{saveError}</p>}
              {entriesError && <p className="text-sm text-destructive">{entriesError}</p>}
              {entriesLoading && <p className="text-sm text-muted-foreground">Lade…</p>}

              {entries && (
                <div className="flex-1 space-y-2 overflow-y-auto">
                  {entries.map((entry, index) => (
                    <div key={index} className="space-y-1 rounded-md border border-border p-2">
                      <div className="flex items-center justify-between">
                        <code className="text-xs text-muted-foreground">
                          gameforge.{selected}.{entry.key}
                        </code>
                        <Button variant="ghost" size="icon-sm" onClick={() => removeEntry(index)}>
                          <Trash2 className="size-3.5" />
                        </Button>
                      </div>
                      <textarea
                        value={toDisplay(entry.value)}
                        onChange={(e) => updateEntry(index, { value: toStored(e.target.value) })}
                        rows={entry.value.length > 60 ? 3 : 1}
                        className="w-full rounded-md border border-border bg-background px-2 py-1 text-sm"
                      />
                    </div>
                  ))}
                  {entries.length === 0 && (
                    <p className="p-2 text-sm text-muted-foreground">Noch keine Einträge.</p>
                  )}

                  <div className="flex gap-2 rounded-md border border-dashed border-border p-2">
                    <input
                      value={newKey}
                      onChange={(e) => setNewKey(e.target.value)}
                      onKeyDown={(e) => e.key === "Enter" && addEntry()}
                      placeholder="Neuer Key (z.B. _010_npcChat)"
                      className="flex-1 rounded-md border border-border bg-background px-2 py-1 text-sm"
                    />
                    <Button variant="outline" onClick={addEntry} disabled={!newKey.trim()}>
                      <Plus className="size-4" />
                      Key hinzufügen
                    </Button>
                  </div>
                </div>
              )}
            </>
          )}
        </div>
      </div>

      {creating && (
        <Modal onClose={() => setCreating(false)}>
          <div className="space-y-3">
            <p className="text-sm font-medium">Neuen Namespace anlegen</p>
            <input
              autoFocus
              value={newNsName}
              onChange={(e) => setNewNsName(e.target.value)}
              placeholder="z.B. meine_quest"
              className="w-full rounded-md border border-border bg-background px-2 py-1 text-sm"
            />
            {newNsName.trim() && (
              <p className="text-xs text-muted-foreground">
                Wird gespeichert als: <code>gameforge.{newNsPreview || "…"}</code>
              </p>
            )}
            {createError && <p className="text-sm text-destructive">{createError}</p>}
            <Button onClick={submitCreate} disabled={!newNsPreview || creatingBusy}>
              {creatingBusy ? "Lege an…" : "Anlegen"}
            </Button>
          </div>
        </Modal>
      )}

      {saveConfirm && (
        <div className="fixed inset-0 flex items-center justify-center bg-black/50">
          <div className="w-96 space-y-3 rounded-lg border border-border bg-card p-4">
            <p className="flex items-start gap-2 text-sm">
              <AlertTriangle className="mt-0.5 size-4 shrink-0 text-destructive" />
              <span>
                <code>translate.lua</code> wird gesichert, danach werden nur die Zeilen von{" "}
                <code>gameforge.{selected}</code> überschrieben.
              </span>
            </p>
            <div className="flex justify-end gap-2">
              <Button variant="outline" onClick={() => setSaveConfirm(false)}>
                Abbrechen
              </Button>
              <Button onClick={saveNamespace}>Speichern</Button>
            </div>
          </div>
        </div>
      )}
    </div>
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
