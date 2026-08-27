import { useEffect, useRef, useState } from "react";
import { invoke } from "@tauri-apps/api/core";
import { runAsyncAction } from "@/lib/asyncAction";
import { logActivity } from "@/lib/logActivity";
import { Button } from "@/components/ui/button";
import { Plus, Trash2, RefreshCw, AlertTriangle, Info, Beaker, HelpCircle, Search, Undo2 } from "lucide-react";
import { openManual } from "@/lib/manual";
import { EntityBrowser } from "@/features/shared/EntityBrowser";
import { reportSectionDirty } from "@/store/navigation";
import { useSaveShortcut } from "@/lib/useSaveShortcut";
import { toast } from "@/components/ui/toast";

interface CubeValue {
  vnum: number;
  count: number;
}

interface CubeRecipe {
  npc_vnums: number[];
  items: CubeValue[];
  rewards: CubeValue[];
  percent: number;
  gold: number;
}

function Field({ label, children }: { label: string; children: React.ReactNode }) {
  return (
    <label className="flex flex-col gap-1 text-xs text-muted-foreground">
      {label}
      {children}
    </label>
  );
}

function EntityLabel({
  vnum,
  kind,
  icons,
  ensureIcon,
}: {
  vnum: number;
  kind: "item" | "mob";
  icons: Record<number, string | null>;
  ensureIcon: (vnum: number, kind: "item" | "mob") => void;
}) {
  useEffect(() => {
    ensureIcon(vnum, kind);
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, [vnum, kind]);

  return (
    <span className="flex items-center gap-1.5">
      {kind === "item" && icons[vnum] && (
        <img src={icons[vnum]!} alt="" className="size-5 [image-rendering:pixelated]" />
      )}
      <span className="text-xs text-muted-foreground">#{vnum}</span>
    </span>
  );
}

// Picks a vnum via the shared EntityBrowser, shown inline below a
// "Hinzufügen…"-toggle - same pattern as BoxEditor's ItemRefEditor/
// BoxItemPicker, kept collapsed by default since a cube recipe can have
// several NPC/material/reward rows at once and an always-open picker per
// row would be a lot of vertical space.
function AddPicker({ kind, onPick }: { kind: "item" | "mob"; onPick: (vnum: number) => void }) {
  const [open, setOpen] = useState(false);
  return (
    <div className="flex flex-col gap-1">
      <Button variant="outline" size="sm" onClick={() => setOpen((o) => !o)}>
        <Search className="size-3.5" /> {kind === "mob" ? "NPC suchen…" : "Item suchen…"}
      </Button>
      {open && (
        <div className="w-96 rounded-md border border-border bg-card p-2 shadow-md">
          <EntityBrowser
            kind={kind}
            pickLabel="Hinzufügen"
            autoFocus
            maxHeightClass="max-h-56"
            onPick={(r) => {
              onPick(r.vnum);
              setOpen(false);
            }}
          />
        </div>
      )}
    </div>
  );
}

function ValueListEditor({
  title,
  values,
  onAdd,
  onChangeCount,
  onRemove,
  icons,
  ensureIcon,
}: {
  title: string;
  values: CubeValue[];
  onAdd: (vnum: number) => void;
  onChangeCount: (index: number, count: number) => void;
  onRemove: (index: number) => void;
  icons: Record<number, string | null>;
  ensureIcon: (vnum: number, kind: "item" | "mob") => void;
}) {
  return (
    <div className="space-y-2">
      <h3 className="text-sm font-medium text-muted-foreground">
        {title} ({values.length})
      </h3>
      {values.map((v, i) => (
        <div key={i} className="flex items-center gap-2 rounded-md border border-border p-2">
          <EntityLabel vnum={v.vnum} kind="item" icons={icons} ensureIcon={ensureIcon} />
          <Field label="Anzahl">
            <input
              type="number"
              value={v.count}
              onChange={(e) => onChangeCount(i, Number(e.target.value) || 0)}
              className="w-20 rounded-md border border-border bg-background px-2 py-1 text-xs"
            />
          </Field>
          <Button variant="outline" size="sm" onClick={() => onRemove(i)}>
            <Trash2 className="size-3.5" />
          </Button>
        </div>
      ))}
      <AddPicker kind="item" onPick={onAdd} />
    </div>
  );
}

export function CubeEditor() {
  const [recipes, setRecipes] = useState<CubeRecipe[] | null>(null);
  const [loading, setLoading] = useState(false);
  const [loadError, setLoadError] = useState<string | null>(null);
  const [selectedIndex, setSelectedIndex] = useState<number | null>(null);
  const [icons, setIcons] = useState<Record<number, string | null>>({});

  const [deleteConfirm, setDeleteConfirm] = useState<number | null>(null);
  const [saving, setSaving] = useState(false);
  const [saveConfirm, setSaveConfirm] = useState(false);

  const [undoConfirm, setUndoConfirm] = useState(false);
  const [undoing, setUndoing] = useState(false);

  const loadedSnapshot = useRef("");
  const dirty = recipes !== null && JSON.stringify(recipes) !== loadedSnapshot.current;
  useEffect(() => {
    reportSectionDirty("cube-editor", dirty);
    return () => reportSectionDirty("cube-editor", false);
  }, [dirty]);

  useEffect(() => {
    load();
  }, []);

  async function load() {
    setSelectedIndex(null);
    await runAsyncAction(() => invoke<CubeRecipe[]>("read_cube_file"), {
      onStart: () => {
        setLoading(true);
        setLoadError(null);
      },
      onSuccess: (loaded) => {
        setRecipes(loaded);
        loadedSnapshot.current = JSON.stringify(loaded);
      },
      onError: setLoadError,
      onFinally: () => setLoading(false),
    });
  }

  function ensureIcon(vnum: number, kind: "item" | "mob") {
    if (kind !== "item" || vnum in icons) return;
    invoke<string | null>("get_item_icon", { vnum })
      .then((url) => setIcons((prev) => ({ ...prev, [vnum]: url })))
      .catch(() => setIcons((prev) => ({ ...prev, [vnum]: null })));
  }

  function updateRecipe(index: number, patch: Partial<CubeRecipe>) {
    setRecipes((prev) => prev!.map((r, i) => (i === index ? { ...r, ...patch } : r)));
  }

  function addRecipe() {
    setRecipes((prev) => [...(prev ?? []), { npc_vnums: [], items: [], rewards: [], percent: 100, gold: 0 }]);
    setSelectedIndex(recipes?.length ?? 0);
  }

  function deleteRecipe(index: number) {
    setRecipes((prev) => prev!.filter((_, i) => i !== index));
    setSelectedIndex(null);
    setDeleteConfirm(null);
  }

  async function save() {
    setSaveConfirm(false);
    await runAsyncAction(() => invoke<string | null>("write_cube_file", { recipes }), {
      onStart: () => setSaving(true),
      onSuccess: (backupPath) => {
        toast.success(backupPath ? `Gespeichert (Backup: ${backupPath})` : "Gespeichert.");
        loadedSnapshot.current = JSON.stringify(recipes);
        logActivity("cube-editor", "save", `cube.txt gespeichert (${recipes?.length ?? 0} Rezept(e))`, "file");
      },
      onError: (e) => toast.error(e),
      onFinally: () => setSaving(false),
    });
  }

  useSaveShortcut("cube-editor", dirty && !saving, () => setSaveConfirm(true));

  // "Letzte Änderung rückgängig machen" - jedes Speichern legt serverseitig
  // schon ein Backup an (siehe write_cube_file), das restauriert
  // undo_cube_write hier nur noch. Betrifft nur bereits gespeicherte Stände
  // auf dem Server, nicht unfertige lokale Eingaben - dafür reicht "Neu
  // laden" bzw. einfach nicht speichern.
  async function undo() {
    setUndoConfirm(false);
    await runAsyncAction(() => invoke<string>("undo_cube_write"), {
      onStart: () => setUndoing(true),
      onSuccess: (backupPath) => {
        toast.success(`Wiederhergestellt aus: ${backupPath}`);
        logActivity("cube-editor", "restore", `cube.txt auf vorherigen Stand zurückgesetzt (${backupPath})`, "file");
        load();
      },
      onError: (e) => toast.error(e),
      onFinally: () => setUndoing(false),
    });
  }

  const selected = selectedIndex !== null ? recipes?.[selectedIndex] : null;

  return (
    <div className="max-w-5xl space-y-6 pb-10">
      <div className="flex items-center gap-2">
        <Beaker className="size-6 text-muted-foreground" />
        <h1 className="text-2xl font-semibold">Cube-Editor</h1>
        <Button variant="ghost" size="icon-sm" title="Hilfe zu diesem Modul" onClick={() => openManual("cube-editor")}>
          <HelpCircle className="size-4" />
        </Button>
      </div>
      <p className="text-sm text-muted-foreground">
        Bearbeitet <code>cube.txt</code> - die Rezepte für das "Cube" (Verwandlung/Kombination) an einem
        NPC: bestimmte NPCs nehmen Material-Items entgegen und geben mit einer Erfolgschance eine
        Belohnung zurück, optional gegen Gold.
      </p>
      <p className="flex items-start gap-2 rounded-lg border border-amber-500/40 bg-amber-500/10 p-3 text-sm text-amber-700 dark:text-amber-400">
        <Info className="mt-0.5 size-4 shrink-0" />
        <span>
          Anders als der Kisten-/Aufwertungs-Editor braucht <code>cube.txt</code> laut Server-Quellcode
          <strong> keinen Neustart</strong> - der Ingame-GM-Befehl <code>/reload c</code> lädt die Rezepte
          sofort neu (<code>Cube_init()</code>). Das genaue Datei-Format wurde diese Session nur gegen den
          Quellcode geprüft, nicht gegen eine echte <code>cube.txt</code> byte-verifiziert - beim ersten
          Speichern also die Datei vor/nach dem Schreiben einmal gegenkontrollieren.
        </span>
      </p>

      <div className="flex items-center gap-2">
        <Button variant="outline" onClick={load} disabled={loading}>
          <RefreshCw className={`size-4 ${loading ? "animate-spin" : ""}`} /> Neu laden
        </Button>
        <Button variant="outline" onClick={addRecipe} disabled={!recipes}>
          <Plus className="size-4" /> Neues Rezept
        </Button>
        <Button onClick={() => setSaveConfirm(true)} disabled={saving || !recipes}>
          {saving ? "Speichere…" : "Speichern"}
        </Button>
        <Button variant="outline" onClick={() => setUndoConfirm(true)} disabled={undoing || !recipes}>
          <Undo2 className="size-4" /> {undoing ? "Setze zurück…" : "Letzte Änderung rückgängig machen"}
        </Button>
      </div>
      {loadError && <p className="text-sm text-destructive">{loadError}</p>}

      {recipes && (
        <div className="flex gap-4">
          <div className="w-64 shrink-0 space-y-1 rounded-lg border border-border p-2">
            {recipes.length === 0 && <p className="p-2 text-sm text-muted-foreground">Keine Rezepte.</p>}
            {recipes.map((r, i) => (
              <button
                key={i}
                onClick={() => setSelectedIndex(i)}
                className={`flex w-full items-center gap-2 rounded-md px-2 py-1.5 text-left text-sm ${
                  selectedIndex === i ? "bg-primary text-primary-foreground" : "hover:bg-muted"
                }`}
              >
                <Beaker className="size-4 shrink-0" />
                <span className="flex-1 truncate">
                  Rezept #{i + 1}
                  {r.rewards[0] && ` → #${r.rewards[0].vnum}`}
                </span>
                <span className="text-xs opacity-70">{r.percent}%</span>
              </button>
            ))}
          </div>

          <div className="flex-1 space-y-4 rounded-lg border border-border p-4">
            {!selected && <p className="text-sm text-muted-foreground">Rezept links auswählen oder neu anlegen.</p>}
            {selected && selectedIndex !== null && (
              <>
                <div className="flex flex-wrap items-end gap-3">
                  <Field label="Erfolgschance (%)">
                    <input
                      type="number"
                      min={0}
                      max={100}
                      value={selected.percent}
                      onChange={(e) => updateRecipe(selectedIndex, { percent: Number(e.target.value) || 0 })}
                      className="w-24 rounded-md border border-border bg-background px-2 py-1 text-sm"
                    />
                  </Field>
                  <Field label="Gold-Kosten">
                    <input
                      type="number"
                      min={0}
                      value={selected.gold}
                      onChange={(e) => updateRecipe(selectedIndex, { gold: Number(e.target.value) || 0 })}
                      className="w-32 rounded-md border border-border bg-background px-2 py-1 text-sm"
                    />
                  </Field>
                  <Button variant="destructive" size="sm" onClick={() => setDeleteConfirm(selectedIndex)}>
                    <Trash2 className="size-3.5" /> Rezept löschen
                  </Button>
                </div>

                <div className="space-y-2">
                  <h3 className="text-sm font-medium text-muted-foreground">
                    Anbietende NPCs ({selected.npc_vnums.length})
                  </h3>
                  {selected.npc_vnums.map((vnum, i) => (
                    <div key={i} className="flex items-center gap-2 rounded-md border border-border p-2">
                      <EntityLabel vnum={vnum} kind="mob" icons={icons} ensureIcon={ensureIcon} />
                      <Button
                        variant="outline"
                        size="sm"
                        onClick={() =>
                          updateRecipe(selectedIndex, {
                            npc_vnums: selected.npc_vnums.filter((_, ni) => ni !== i),
                          })
                        }
                      >
                        <Trash2 className="size-3.5" />
                      </Button>
                    </div>
                  ))}
                  <AddPicker
                    kind="mob"
                    onPick={(vnum) => updateRecipe(selectedIndex, { npc_vnums: [...selected.npc_vnums, vnum] })}
                  />
                </div>

                <ValueListEditor
                  title="Benötigte Materialien"
                  values={selected.items}
                  icons={icons}
                  ensureIcon={ensureIcon}
                  onAdd={(vnum) => updateRecipe(selectedIndex, { items: [...selected.items, { vnum, count: 1 }] })}
                  onChangeCount={(i, count) =>
                    updateRecipe(selectedIndex, {
                      items: selected.items.map((v, vi) => (vi === i ? { ...v, count } : v)),
                    })
                  }
                  onRemove={(i) =>
                    updateRecipe(selectedIndex, { items: selected.items.filter((_, vi) => vi !== i) })
                  }
                />

                <ValueListEditor
                  title="Belohnung"
                  values={selected.rewards}
                  icons={icons}
                  ensureIcon={ensureIcon}
                  onAdd={(vnum) => updateRecipe(selectedIndex, { rewards: [...selected.rewards, { vnum, count: 1 }] })}
                  onChangeCount={(i, count) =>
                    updateRecipe(selectedIndex, {
                      rewards: selected.rewards.map((v, vi) => (vi === i ? { ...v, count } : v)),
                    })
                  }
                  onRemove={(i) =>
                    updateRecipe(selectedIndex, { rewards: selected.rewards.filter((_, vi) => vi !== i) })
                  }
                />
              </>
            )}
          </div>
        </div>
      )}

      {deleteConfirm !== null && (
        <div className="fixed inset-0 flex items-center justify-center bg-black/50">
          <div className="w-96 space-y-3 rounded-lg border border-border bg-card p-4">
            <p className="flex items-center gap-2 text-sm font-medium">
              <AlertTriangle className="size-4 text-destructive" /> Rezept #{deleteConfirm + 1} wirklich entfernen?
            </p>
            <p className="text-xs text-muted-foreground">
              Wird erst beim "Speichern" wirklich auf den Server geschrieben - bis dahin rückgängig machbar
              durch nicht speichern/neu laden.
            </p>
            <div className="flex justify-end gap-2">
              <Button variant="outline" onClick={() => setDeleteConfirm(null)}>
                Abbrechen
              </Button>
              <Button variant="destructive" onClick={() => deleteRecipe(deleteConfirm)}>
                Entfernen
              </Button>
            </div>
          </div>
        </div>
      )}

      {saveConfirm && (
        <div className="fixed inset-0 flex items-center justify-center bg-black/50">
          <div className="w-96 space-y-3 rounded-lg border border-border bg-card p-4">
            <p className="text-sm font-medium">cube.txt jetzt überschreiben?</p>
            <p className="text-xs text-muted-foreground">
              Ein Backup der aktuellen Datei wird vorher auf dem Server angelegt.
            </p>
            <div className="flex justify-end gap-2">
              <Button variant="outline" onClick={() => setSaveConfirm(false)}>
                Abbrechen
              </Button>
              <Button onClick={save}>Speichern</Button>
            </div>
          </div>
        </div>
      )}

      {undoConfirm && (
        <div className="fixed inset-0 flex items-center justify-center bg-black/50">
          <div className="w-96 space-y-3 rounded-lg border border-border bg-card p-4">
            <p className="flex items-start gap-2 text-sm font-medium">
              <AlertTriangle className="mt-0.5 size-4 shrink-0 text-destructive" />
              Letzte gespeicherte Änderung an cube.txt rückgängig machen?
            </p>
            <p className="text-xs text-muted-foreground">
              Stellt die zuletzt gesicherte Vorgängerversion wieder her (der aktuelle Stand wird dabei
              selbst zuerst gesichert). Betrifft nur, was zuletzt tatsächlich gespeichert wurde - nicht
              unfertige, noch ungespeicherte Eingaben hier im Editor.
            </p>
            <div className="flex justify-end gap-2">
              <Button variant="outline" onClick={() => setUndoConfirm(false)}>
                Abbrechen
              </Button>
              <Button variant="destructive" onClick={undo} disabled={undoing}>
                {undoing ? "Setze zurück…" : "Rückgängig machen"}
              </Button>
            </div>
          </div>
        </div>
      )}
    </div>
  );
}
