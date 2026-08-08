import { useEffect, useState } from "react";
import { invoke } from "@tauri-apps/api/core";
import { runAsyncAction } from "@/lib/asyncAction";
import { Button } from "@/components/ui/button";
import { Search, AlertTriangle, Users, Plus, Trash2, KeyRound, ChevronLeft, ChevronRight, CheckCircle2, HelpCircle } from "lucide-react";
import { GenericRowEditor } from "@/features/shared/GenericRowEditor";
import { openManual } from "@/lib/manual";

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
const PLAYER_TABLE: TableTarget = { database: "player", table: "player" };
const ITEM_TABLE: TableTarget = { database: "player", table: "item" };

export function AccountManager() {
  return (
    <div className="max-w-4xl space-y-8 pb-10">
      <div className="flex items-center gap-2">
        <Users className="size-6 text-muted-foreground" />
        <h1 className="text-2xl font-semibold">Account-/Spieler-Verwaltung</h1>
        <Button variant="ghost" size="icon-sm" title="Hilfe zu diesem Modul" onClick={() => openManual("account-manager")}>
          <HelpCircle className="size-4" />
        </Button>
      </div>
      <p className="text-sm text-muted-foreground">
        Es gibt keinen Kanal zu einem laufenden Spielprozess - Änderungen an bereits online
        befindlichen Spielern wirken daher unter Umständen erst nach Neuanmeldung, nicht sofort
        live.
      </p>

      <AccountSection />
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

interface AccountSummary {
  id: number;
  login: string;
  email: string;
  status: string;
  empire: number;
  create_time: string;
  last_play: string | null;
}

const EMPIRE_LABELS: Record<number, string> = {
  0: "Noch nicht gewählt",
  1: "Shinsoo (Rot)",
  2: "Chunjo (Gelb)",
  3: "Jinno (Blau)",
};

const PAGE_SIZE = 20;

// Eigene, gezielte Commands statt des generischen DB-Explorer-Zeilen-CRUD -
// Passwörter müssen über MySQLs eigene PASSWORD()-Funktion gesetzt werden
// (verifiziert gegen den echten Server-Quellcode: der Login-Check vergleicht
// PASSWORD(eingegebenes_pw) mit der gespeicherten Spalte), das kann der
// generische insert_table_row/update_table_row-Pfad nicht leisten - deshalb
// hier ein eigener kleiner Abschnitt statt TableSearchSection.
function AccountSection() {
  const [accounts, setAccounts] = useState<AccountSummary[] | null>(null);
  const [total, setTotal] = useState(0);
  const [page, setPage] = useState(0);
  const [search, setSearch] = useState("");
  const [loading, setLoading] = useState(false);
  const [loadError, setLoadError] = useState<string | null>(null);

  const [editingPk, setEditingPk] = useState<string | null>(null);
  const [resetTarget, setResetTarget] = useState<AccountSummary | null>(null);

  const [creating, setCreating] = useState(false);
  const [newLogin, setNewLogin] = useState("");
  const [newPassword, setNewPassword] = useState("");
  const [newEmpire, setNewEmpire] = useState("0");
  const [createBusy, setCreateBusy] = useState(false);
  const [createError, setCreateError] = useState<string | null>(null);

  async function load() {
    await runAsyncAction(
      async () => {
        const [rows, count] = await Promise.all([
          invoke<AccountSummary[]>("list_accounts", { search, limit: PAGE_SIZE, offset: page * PAGE_SIZE }),
          invoke<number>("count_accounts", { search }),
        ]);
        return { rows, count };
      },
      {
        onStart: () => {
          setLoading(true);
          setLoadError(null);
        },
        onSuccess: ({ rows, count }) => {
          setAccounts(rows);
          setTotal(count);
        },
        onError: setLoadError,
        onFinally: () => setLoading(false),
      },
    );
  }

  useEffect(() => {
    load();
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, [page]);

  function runSearch() {
    setPage(0);
    load();
  }

  async function submitCreate() {
    await runAsyncAction(
      () =>
        invoke("create_account", {
          login: newLogin.trim(),
          password: newPassword,
          empire: newEmpire === "0" ? null : Number(newEmpire),
        }),
      {
        onStart: () => {
          setCreateBusy(true);
          setCreateError(null);
        },
        onSuccess: async () => {
          setCreating(false);
          setNewLogin("");
          setNewPassword("");
          setNewEmpire("0");
          setPage(0);
          await load();
        },
        onError: setCreateError,
        onFinally: () => setCreateBusy(false),
      },
    );
  }

  const canCreate = newLogin.trim().length > 0 && newPassword.length > 0;
  const totalPages = Math.max(1, Math.ceil(total / PAGE_SIZE));

  return (
    <section className="space-y-2 rounded-lg border border-border p-4">
      <div className="flex items-center justify-between">
        <h2 className="text-sm font-medium text-muted-foreground">Accounts</h2>
        <Button size="sm" onClick={() => setCreating(true)}>
          <Plus className="size-3.5" />
          Neuer Account…
        </Button>
      </div>

      <div className="flex gap-2">
        <input
          value={search}
          onChange={(e) => setSearch(e.target.value)}
          onKeyDown={(e) => e.key === "Enter" && runSearch()}
          placeholder="Login suchen (leer = alle anzeigen)…"
          className="flex-1 rounded-md border border-border bg-background px-2 py-1 text-sm"
        />
        <Button variant="outline" onClick={runSearch} disabled={loading}>
          <Search className="size-4" />
        </Button>
      </div>

      {loadError && (
        <p className="flex items-start gap-2 text-sm text-destructive">
          <AlertTriangle className="mt-0.5 size-4 shrink-0" />
          <span>{loadError}</span>
        </p>
      )}

      {accounts && (
        <div className="overflow-x-auto rounded-md border border-border">
          <table className="w-full text-xs">
            <thead className="bg-muted/40">
              <tr>
                <th className="px-2 py-1 text-left font-medium">ID</th>
                <th className="px-2 py-1 text-left font-medium">Login</th>
                <th className="px-2 py-1 text-left font-medium">E-Mail</th>
                <th className="px-2 py-1 text-left font-medium">Status</th>
                <th className="px-2 py-1 text-left font-medium">Reich</th>
                <th className="px-2 py-1 text-left font-medium">Erstellt</th>
                <th className="px-2 py-1 text-left font-medium">Zuletzt gespielt</th>
                <th />
              </tr>
            </thead>
            <tbody>
              {accounts.map((a) => (
                <tr key={a.id} className="border-t border-border">
                  <td className="px-2 py-1">{a.id}</td>
                  <td className="px-2 py-1 font-medium">{a.login}</td>
                  <td className="max-w-32 truncate px-2 py-1">{a.email || "—"}</td>
                  <td className="px-2 py-1">{a.status}</td>
                  <td className="px-2 py-1">{EMPIRE_LABELS[a.empire] ?? a.empire}</td>
                  <td className="px-2 py-1">{a.create_time}</td>
                  <td className="px-2 py-1">{a.last_play ?? "—"}</td>
                  <td className="whitespace-nowrap px-2 py-1 text-right">
                    <Button size="sm" variant="outline" onClick={() => setEditingPk(String(a.id))}>
                      Bearbeiten
                    </Button>{" "}
                    <Button size="sm" variant="outline" onClick={() => setResetTarget(a)}>
                      <KeyRound className="size-3.5" />
                      Passwort
                    </Button>
                  </td>
                </tr>
              ))}
              {accounts.length === 0 && (
                <tr>
                  <td colSpan={8} className="px-2 py-3 text-center text-muted-foreground">
                    Keine Accounts gefunden.
                  </td>
                </tr>
              )}
            </tbody>
          </table>
          <div className="flex items-center justify-between border-t border-border p-1.5 text-[10px] text-muted-foreground">
            <span>
              {total} Account{total === 1 ? "" : "s"} - Seite {page + 1} von {totalPages}
            </span>
            <div className="flex gap-1">
              <Button variant="ghost" size="icon-sm" disabled={page === 0} onClick={() => setPage((p) => p - 1)}>
                <ChevronLeft className="size-3.5" />
              </Button>
              <Button
                variant="ghost"
                size="icon-sm"
                disabled={page + 1 >= totalPages}
                onClick={() => setPage((p) => p + 1)}
              >
                <ChevronRight className="size-3.5" />
              </Button>
            </div>
          </div>
        </div>
      )}

      {editingPk !== null && (
        <GenericRowEditor
          database="account"
          table="account"
          pkValue={editingPk}
          onClose={() => setEditingPk(null)}
          onSaved={load}
        />
      )}

      {resetTarget && <ResetPasswordDialog account={resetTarget} onClose={() => setResetTarget(null)} />}

      {creating && (
        <div className="fixed inset-0 z-50 flex items-center justify-center bg-black/50">
          <div className="w-96 space-y-3 rounded-lg border border-border bg-card p-4">
            <p className="text-sm font-medium">Neuer Account</p>
            <label className="flex flex-col gap-1 text-xs text-muted-foreground">
              Login
              <input
                autoFocus
                value={newLogin}
                onChange={(e) => setNewLogin(e.target.value)}
                className="rounded-md border border-border bg-background px-2 py-1 text-sm"
              />
            </label>
            <label className="flex flex-col gap-1 text-xs text-muted-foreground">
              Passwort
              <input
                type="text"
                value={newPassword}
                onChange={(e) => setNewPassword(e.target.value)}
                className="rounded-md border border-border bg-background px-2 py-1 text-sm"
              />
            </label>
            <label className="flex flex-col gap-1 text-xs text-muted-foreground">
              Reich (optional, sonst wählt der Spieler beim ersten Login)
              <select
                value={newEmpire}
                onChange={(e) => setNewEmpire(e.target.value)}
                className="rounded-md border border-border bg-background px-2 py-1 text-sm"
              >
                {Object.entries(EMPIRE_LABELS).map(([value, label]) => (
                  <option key={value} value={value}>
                    {label}
                  </option>
                ))}
              </select>
            </label>
            {createError && <p className="text-sm text-destructive">{createError}</p>}
            <div className="flex justify-end gap-2">
              <Button variant="outline" onClick={() => setCreating(false)}>
                Abbrechen
              </Button>
              <Button disabled={!canCreate || createBusy} onClick={submitCreate}>
                {createBusy ? "Lege an…" : "Anlegen"}
              </Button>
            </div>
          </div>
        </div>
      )}
    </section>
  );
}

// Passwörter sind ein Einweg-Hash (MySQL PASSWORD()) - live gegen echte
// Accounts verifiziert (41-Zeichen "*..."-Hash), nicht umkehrbar. "Auslesen"
// ist daher technisch unmöglich; stattdessen wird ein neues Passwort
// gesetzt, genau wie es der echte Login-Check erwartet.
function ResetPasswordDialog({ account, onClose }: { account: AccountSummary; onClose: () => void }) {
  const [password, setPassword] = useState("");
  const [busy, setBusy] = useState(false);
  const [error, setError] = useState<string | null>(null);
  const [ok, setOk] = useState(false);

  async function submit() {
    await runAsyncAction(() => invoke("reset_account_password", { id: account.id, newPassword: password }), {
      onStart: () => {
        setBusy(true);
        setError(null);
      },
      onSuccess: () => setOk(true),
      onError: setError,
      onFinally: () => setBusy(false),
    });
  }

  return (
    <div className="fixed inset-0 z-50 flex items-center justify-center bg-black/50">
      <div className="w-96 space-y-3 rounded-lg border border-border bg-card p-4">
        <p className="text-sm font-medium">Passwort zurücksetzen: {account.login}</p>
        <p className="text-xs text-muted-foreground">
          Passwörter sind als Einweg-Hash gespeichert und können nicht ausgelesen werden - hier
          lässt sich nur ein neues setzen.
        </p>
        <label className="flex flex-col gap-1 text-xs text-muted-foreground">
          Neues Passwort
          <input
            autoFocus
            type="text"
            value={password}
            onChange={(e) => setPassword(e.target.value)}
            className="rounded-md border border-border bg-background px-2 py-1 text-sm"
          />
        </label>
        {error && <p className="text-sm text-destructive">{error}</p>}
        {ok && (
          <p className="flex items-center gap-1 text-sm text-green-600">
            <CheckCircle2 className="size-4" /> Passwort gesetzt.
          </p>
        )}
        <div className="flex justify-end gap-2">
          <Button variant="outline" onClick={onClose}>
            {ok ? "Schließen" : "Abbrechen"}
          </Button>
          {!ok && (
            <Button disabled={!password || busy} onClick={submit}>
              {busy ? "Setze…" : "Setzen"}
            </Button>
          )}
        </div>
      </div>
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
