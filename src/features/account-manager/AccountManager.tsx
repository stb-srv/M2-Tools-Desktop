import { useState } from "react";
import { invoke } from "@tauri-apps/api/core";
import { runAsyncAction } from "@/lib/asyncAction";
import { Button } from "@/components/ui/button";
import { Search, AlertTriangle, Users, Plus, Trash2 } from "lucide-react";
import { GenericRowEditor } from "@/features/shared/GenericRowEditor";

interface ColumnInfo {
  name: string;
  data_type: string;
  is_nullable: boolean;
  is_primary_key: boolean;
}

interface TableRows {
  columns: string[];
  rows: (string | null)[][];
  total_rows: number;
}

interface TableTarget {
  database: string;
  table: string;
}

// Verified table locations from earlier live-DB introspection (see
// [[m2manager-db-schema]] memory) - account/player data lives in these two
// tables on this core. The search column and primary key are NOT assumed
// beyond that: search column is user-editable (schemas vary in which column
// holds the login/character name), and the primary key is auto-detected at
// runtime by GenericRowEditor via `is_primary_key`, never guessed as "id".
const ACCOUNT_TABLE: TableTarget = { database: "account", table: "account" };
const PLAYER_TABLE: TableTarget = { database: "player", table: "player" };
const ITEM_TABLE: TableTarget = { database: "player", table: "item" };

export function AccountManager() {
  return (
    <div className="max-w-4xl space-y-8 pb-10">
      <div className="flex items-center gap-2">
        <Users className="size-6 text-muted-foreground" />
        <h1 className="text-2xl font-semibold">Account-/Spieler-Verwaltung</h1>
      </div>
      <p className="text-sm text-muted-foreground">
        Sucht und bearbeitet Zeilen direkt in <code>account.account</code> und{" "}
        <code>player.player</code> (generischer Editor, siehe Mob-Proto-Editor für dieselbe
        Technik). Damit lassen sich z.B. GM-Level, Sperrstatus oder Position ändern - welche
        Spalte das konkret ist, hängt vom Core ab und wird hier nicht geraten, sondern als reale
        Spalte mit echtem Wert angezeigt. Es gibt keinen Kanal zu einem laufenden Spielprozess -
        Änderungen an bereits online befindlichen Spielern wirken daher unter Umständen erst nach
        Neuanmeldung, nicht sofort live.
      </p>

      <TableSearchSection
        title="Accounts"
        target={ACCOUNT_TABLE}
        defaultColumn="login"
        placeholder="Login suchen…"
      />
      <TableSearchSection
        title="Spieler"
        target={PLAYER_TABLE}
        defaultColumn="name"
        placeholder="Charaktername suchen…"
      />
      <GiveItemSection />
    </div>
  );
}

function TableSearchSection({
  title,
  target,
  defaultColumn,
  placeholder,
}: {
  title: string;
  target: TableTarget;
  defaultColumn: string;
  placeholder: string;
}) {
  const [column, setColumn] = useState(defaultColumn);
  const [query, setQuery] = useState("");
  const [result, setResult] = useState<TableRows | null>(null);
  const [searching, setSearching] = useState(false);
  const [searchError, setSearchError] = useState<string | null>(null);
  const [editingPk, setEditingPk] = useState<string | null>(null);

  async function runSearch() {
    if (!query.trim()) return;
    await runAsyncAction(
      () =>
        invoke<TableRows>("search_table_rows", {
          database: target.database,
          table: target.table,
          column,
          query: query.trim(),
        }),
      {
        onStart: () => {
          setSearching(true);
          setSearchError(null);
        },
        onSuccess: setResult,
        onError: setSearchError,
        onFinally: () => setSearching(false),
      },
    );
  }

  async function pickRow(row: (string | null)[]) {
    if (!result) return;
    await runAsyncAction(
      async () => {
        const cols = await invoke<ColumnInfo[]>("get_table_columns", {
          database: target.database,
          table: target.table,
        });
        const pk = cols.find((c) => c.is_primary_key);
        if (!pk) throw new Error("Diese Tabelle hat keinen Primärschlüssel.");
        const index = result.columns.indexOf(pk.name);
        const value = row[index];
        if (value === null) throw new Error("Primärschlüssel-Wert ist NULL.");
        return value;
      },
      { onSuccess: setEditingPk, onError: setSearchError },
    );
  }

  return (
    <section className="space-y-2 rounded-lg border border-border p-4">
      <h2 className="text-sm font-medium text-muted-foreground">{title}</h2>
      <div className="flex gap-2">
        <input
          value={column}
          onChange={(e) => setColumn(e.target.value)}
          title="Suchspalte"
          className="w-32 rounded-md border border-border bg-background px-2 py-1 text-sm"
        />
        <input
          value={query}
          onChange={(e) => setQuery(e.target.value)}
          onKeyDown={(e) => e.key === "Enter" && runSearch()}
          placeholder={placeholder}
          className="flex-1 rounded-md border border-border bg-background px-2 py-1 text-sm"
        />
        <Button variant="outline" onClick={runSearch} disabled={searching}>
          <Search className="size-4" />
        </Button>
      </div>

      {searchError && (
        <p className="flex items-start gap-2 text-sm text-destructive">
          <AlertTriangle className="mt-0.5 size-4 shrink-0" />
          <span>{searchError}</span>
        </p>
      )}

      {result && result.rows.length > 0 && (
        <div className="overflow-x-auto rounded-md border border-border">
          <table className="w-full text-xs">
            <thead className="bg-muted/40">
              <tr>
                {result.columns.slice(0, 6).map((c) => (
                  <th key={c} className="px-2 py-1 text-left font-medium">
                    {c}
                  </th>
                ))}
                <th />
              </tr>
            </thead>
            <tbody>
              {result.rows.map((row, i) => (
                <tr key={i} className="border-t border-border">
                  {row.slice(0, 6).map((cell, j) => (
                    <td key={j} className="max-w-32 truncate px-2 py-1">
                      {cell ?? <span className="text-muted-foreground">NULL</span>}
                    </td>
                  ))}
                  <td className="px-2 py-1 text-right">
                    <Button size="sm" variant="outline" onClick={() => pickRow(row)}>
                      Bearbeiten
                    </Button>
                  </td>
                </tr>
              ))}
            </tbody>
          </table>
          {result.columns.length > 6 && (
            <p className="border-t border-border p-1.5 text-[10px] text-muted-foreground">
              Nur die ersten 6 von {result.columns.length} Spalten angezeigt - "Bearbeiten" zeigt
              alle.
            </p>
          )}
        </div>
      )}
      {result && result.rows.length === 0 && (
        <p className="text-sm text-muted-foreground">Keine Treffer.</p>
      )}

      {editingPk !== null && (
        <GenericRowEditor
          database={target.database}
          table={target.table}
          pkValue={editingPk}
          onClose={() => setEditingPk(null)}
        />
      )}
    </section>
  );
}

// Generic insert/delete on player.item - deliberately not core-specific
// (window/pos/count semantics for items were never verified against a real
// server, unlike item_proto). The user fills in every column themselves;
// this just saves writing raw SQL and adds a confirmation step.
function GiveItemSection() {
  const [columns, setColumns] = useState<ColumnInfo[] | null>(null);
  const [loadError, setLoadError] = useState<string | null>(null);
  const [values, setValues] = useState<Record<string, string>>({});
  const [open, setOpen] = useState(false);
  const [confirmOpen, setConfirmOpen] = useState(false);
  const [saving, setSaving] = useState(false);
  const [saveError, setSaveError] = useState<string | null>(null);
  const [saveOk, setSaveOk] = useState(false);

  const [deleteId, setDeleteId] = useState("");
  const [deleteColumn, setDeleteColumn] = useState("id");
  const [deleteConfirm, setDeleteConfirm] = useState(false);
  const [deleteBusy, setDeleteBusy] = useState(false);
  const [deleteError, setDeleteError] = useState<string | null>(null);
  const [deleteOk, setDeleteOk] = useState(false);

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
        onSuccess: () => setSaveOk(true),
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
        onSuccess: () => setDeleteOk(true),
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
