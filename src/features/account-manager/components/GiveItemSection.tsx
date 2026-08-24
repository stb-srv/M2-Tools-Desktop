import { useState } from "react";
import { invoke } from "@tauri-apps/api/core";
import { runAsyncAction } from "@/lib/asyncAction";
import { logActivity } from "@/lib/logActivity";
import { Button } from "@/components/ui/button";
import { Plus, Trash2, AlertTriangle } from "lucide-react";
import { EntityBrowser } from "@/features/shared/EntityBrowser";
import { ITEM_TABLE, type ColumnInfo } from "../shared";
import { MiniPlayerSearch } from "./MiniPlayerSearch";

// Errät anhand des Namensmusters eine wahrscheinliche Spalte aus der echten,
// live geholten Spaltenliste - nur als Ausfüllhilfe für die Picker unten,
// niemals als Ersatz für die generische Eingabe (Bedeutung der übrigen
// Spalten bleibt core-spezifisch unverifiziert, siehe Kommentar unten).
function guessColumn(columns: ColumnInfo[], patterns: RegExp[]): string | null {
  for (const p of patterns) {
    const hit = columns.find((c) => p.test(c.name));
    if (hit) return hit.name;
  }
  return null;
}

// Generic insert/delete on player.item - deliberately not core-specific
// (window/pos/count semantics for items were never verified against a real
// server, unlike item_proto). The user fills in every column themselves;
// this just saves writing raw SQL and adds a confirmation step. Die
// Item-/Spieler-Picker unten sind reine Ausfüllhilfen (siehe guessColumn) -
// finden sie keine passende Spalte, wird der Wert stattdessen in die
// Zwischenablage kopiert statt eine falsche Spalte zu raten.
export function GiveItemSection() {
  const [columns, setColumns] = useState<ColumnInfo[] | null>(null);
  const [loadError, setLoadError] = useState<string | null>(null);
  const [values, setValues] = useState<Record<string, string>>({});
  const [open, setOpen] = useState(false);
  const [confirmOpen, setConfirmOpen] = useState(false);
  const [saving, setSaving] = useState(false);
  const [saveError, setSaveError] = useState<string | null>(null);
  const [saveOk, setSaveOk] = useState(false);
  const [pickerNote, setPickerNote] = useState<string | null>(null);

  const [deleteId, setDeleteId] = useState("");
  const [deleteColumn, setDeleteColumn] = useState("id");
  const [deleteConfirm, setDeleteConfirm] = useState(false);
  const [deleteBusy, setDeleteBusy] = useState(false);
  const [deleteError, setDeleteError] = useState<string | null>(null);
  const [deleteOk, setDeleteOk] = useState(false);

  function applyPickedValue(patterns: RegExp[], value: string, label: string) {
    if (!columns) return;
    const col = guessColumn(columns, patterns);
    if (col) {
      setValues((prev) => ({ ...prev, [col]: value }));
      setPickerNote(`${label} in Spalte "${col}" übernommen.`);
    } else {
      navigator.clipboard.writeText(value).catch(() => {});
      setPickerNote(`Keine passende Spalte gefunden - "${value}" in die Zwischenablage kopiert, bitte manuell einfügen.`);
    }
  }

  async function openForm() {
    setOpen(true);
    setSaveOk(false);
    await runAsyncAction(
      () =>
        invoke<ColumnInfo[]>("get_table_columns", {
          database: ITEM_TABLE.database,
          table: ITEM_TABLE.table,
        }),
      {
        onSuccess: (cols) => {
          setColumns(cols);
          setValues(Object.fromEntries(cols.map((c) => [c.name, ""])));
        },
        onError: setLoadError,
      },
    );
  }

  async function submitInsert() {
    setConfirmOpen(false);
    const entries: [string, string | null][] = Object.entries(values)
      .filter(([, v]) => v.trim() !== "")
      .map(([k, v]) => [k, v]);
    await runAsyncAction(
      () =>
        invoke("insert_table_row", {
          database: ITEM_TABLE.database,
          table: ITEM_TABLE.table,
          values: entries,
        }),
      {
        onStart: () => {
          setSaving(true);
          setSaveError(null);
        },
        onSuccess: () => {
          setSaveOk(true);
          logActivity("account-manager", "give-item", `Neue Zeile in player.item eingefügt (${entries.length} Spalte(n) gesetzt)`, "item");
        },
        onError: setSaveError,
        onFinally: () => setSaving(false),
      },
    );
  }

  async function submitDelete() {
    setDeleteConfirm(false);
    await runAsyncAction(
      () =>
        invoke("delete_table_row", {
          database: ITEM_TABLE.database,
          table: ITEM_TABLE.table,
          pkColumn: deleteColumn,
          pkValue: deleteId,
        }),
      {
        onStart: () => {
          setDeleteBusy(true);
          setDeleteError(null);
        },
        onSuccess: () => {
          setDeleteOk(true);
          logActivity("account-manager", "remove-item", `Zeile in player.item gelöscht (${deleteColumn}=${deleteId})`, "item", deleteId);
        },
        onError: setDeleteError,
        onFinally: () => setDeleteBusy(false),
      },
    );
  }

  return (
    <section className="space-y-2 rounded-lg border border-border p-4">
      <h2 className="text-sm font-medium text-muted-foreground">
        Item geben / entfernen (player.item)
      </h2>
      <p className="text-xs text-muted-foreground">
        Window/Position/Zähler-Bedeutung von <code>player.item</code> ist core-spezifisch und
        nicht verifiziert - trage die Spalten so ein, wie sie auf diesem Server erwartet werden
        (z.B. Besitzer-Spalte mit der ID aus der Spieler-Suche oben füllen).
      </p>

      <div className="flex gap-2">
        <Button variant="outline" size="sm" onClick={openForm}>
          <Plus className="size-3.5" />
          Neue Item-Zeile einfügen…
        </Button>
      </div>

      {open && (
        <div className="space-y-2 rounded-md border border-border p-3">
          {loadError && <p className="text-sm text-destructive">{loadError}</p>}
          {!columns && !loadError && (
            <p className="text-sm text-muted-foreground">Lade Spalten…</p>
          )}
          {columns && (
            <>
              <div className="flex flex-wrap items-center gap-2 rounded-md border border-border bg-muted/30 p-2">
                <EntityBrowser
                  kind="item"
                  pickLabel="Vnum übernehmen"
                  onPick={(row) => applyPickedValue([/^vnum$/i, /item_vnum/i, /vnum/i], String(row.vnum), `Item-Vnum ${row.vnum}`)}
                />
                <MiniPlayerSearch
                  onPick={(id, name) =>
                    applyPickedValue([/owner/i, /player_id/i, /char.*id/i, /account_id/i], id, `Spieler-ID von "${name}"`)
                  }
                />
              </div>
              {pickerNote && <p className="text-xs text-muted-foreground">{pickerNote}</p>}
              <div className="grid grid-cols-2 gap-2">
                {columns.map((c) => (
                  <label key={c.name} className="flex flex-col gap-0.5 text-xs">
                    <span className="font-mono text-muted-foreground">{c.name}</span>
                    <input
                      value={values[c.name] ?? ""}
                      onChange={(e) =>
                        setValues((prev) => ({ ...prev, [c.name]: e.target.value }))
                      }
                      className="rounded-md border border-border bg-background px-2 py-1 text-sm"
                    />
                  </label>
                ))}
              </div>
              {saveError && <p className="text-sm text-destructive">{saveError}</p>}
              {saveOk && <p className="text-sm text-green-600">Zeile eingefügt.</p>}
              <Button size="sm" onClick={() => setConfirmOpen(true)} disabled={saving}>
                {saving ? "Füge ein…" : "Einfügen"}
              </Button>
            </>
          )}
        </div>
      )}

      <div className="flex flex-wrap items-end gap-2 rounded-md border border-border p-3">
        <label className="flex flex-col gap-0.5 text-xs text-muted-foreground">
          Primärschlüssel-Spalte
          <input
            value={deleteColumn}
            onChange={(e) => setDeleteColumn(e.target.value)}
            className="w-32 rounded-md border border-border bg-background px-2 py-1 text-sm"
          />
        </label>
        <label className="flex flex-col gap-0.5 text-xs text-muted-foreground">
          Wert der zu löschenden Zeile
          <input
            value={deleteId}
            onChange={(e) => setDeleteId(e.target.value)}
            className="w-40 rounded-md border border-border bg-background px-2 py-1 text-sm"
          />
        </label>
        <Button
          variant="destructive"
          size="sm"
          onClick={() => setDeleteConfirm(true)}
          disabled={!deleteId.trim() || deleteBusy}
        >
          <Trash2 className="size-3.5" />
          {deleteBusy ? "Lösche…" : "Item-Zeile löschen"}
        </Button>
      </div>
      {deleteError && <p className="text-sm text-destructive">{deleteError}</p>}
      {deleteOk && <p className="text-sm text-green-600">Zeile gelöscht.</p>}

      {confirmOpen && (
        <div className="fixed inset-0 z-50 flex items-center justify-center bg-black/50">
          <div className="w-96 space-y-3 rounded-lg border border-border bg-card p-4">
            <p className="flex items-start gap-2 text-sm">
              <AlertTriangle className="mt-0.5 size-4 shrink-0 text-destructive" />
              <span>Eine neue Zeile wird direkt in player.item eingefügt, ohne Rückfrage danach.</span>
            </p>
            <div className="flex justify-end gap-2">
              <Button variant="outline" onClick={() => setConfirmOpen(false)}>
                Abbrechen
              </Button>
              <Button onClick={submitInsert}>Einfügen</Button>
            </div>
          </div>
        </div>
      )}

      {deleteConfirm && (
        <div className="fixed inset-0 z-50 flex items-center justify-center bg-black/50">
          <div className="w-96 space-y-3 rounded-lg border border-border bg-card p-4">
            <p className="flex items-start gap-2 text-sm">
              <AlertTriangle className="mt-0.5 size-4 shrink-0 text-destructive" />
              <span>
                Zeile mit <code>{deleteColumn}</code> = <code>{deleteId}</code> wird endgültig aus
                player.item gelöscht.
              </span>
            </p>
            <div className="flex justify-end gap-2">
              <Button variant="outline" onClick={() => setDeleteConfirm(false)}>
                Abbrechen
              </Button>
              <Button variant="destructive" onClick={submitDelete}>
                Löschen
              </Button>
            </div>
          </div>
        </div>
      )}
    </section>
  );
}
