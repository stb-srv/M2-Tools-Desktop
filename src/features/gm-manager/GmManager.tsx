import { useEffect, useMemo, useState } from "react";
import { invoke } from "@tauri-apps/api/core";
import { runAsyncAction } from "@/lib/asyncAction";
import { Button } from "@/components/ui/button";
import { Shield, Plus, Trash2, RefreshCw, AlertTriangle, Info, HelpCircle, Search } from "lucide-react";
import { openManual } from "@/lib/manual";
import { logActivity } from "@/lib/logActivity";
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

interface AccountSummary {
  id: number;
  login: string;
}

// common.gmlist/common.gmhost verified to exist on this core (see
// [[m2manager_db_schema]] memory, live SHOW TABLES against the real dev
// server) and gmlist's real columns (mID/mAccount/mName/mContactIP/
// mServerIP/mAuthority) verified against the DB core's own query
// (`source/db/src/ClientManager.cpp::__GetAdminInfo`) this session - but the
// exact column *types* were never live-checked against this specific
// database (unlike item_proto/account.account), so this goes through the
// same generic, schema-introspecting commands as Mob-Proto-Editor/Account-
// Verwaltung instead of a purpose-built struct, and refuses to render if the
// expected columns aren't actually there.
const DB = "common";
const TABLE = "gmlist";

// EGMLevels (source/common/length.h), lowest to highest:
// GM_PLAYER < GM_LOW_WIZARD < GM_WIZARD < GM_HIGH_WIZARD < GM_GOD <
// GM_IMPLEMENTOR. Only the 5 above GM_PLAYER are ever stored as a string in
// gmlist.mAuthority (ClientManager.cpp maps exactly these 5 strings, any
// other value is silently skipped - `continue` in __GetAdminInfo).
const AUTHORITY_LEVELS = [
  { value: "LOW_WIZARD", label: "Low Wizard (niedrigster GM-Rang)" },
  { value: "WIZARD", label: "Wizard" },
  { value: "HIGH_WIZARD", label: "High Wizard" },
  { value: "GOD", label: "God" },
  { value: "IMPLEMENTOR", label: "Implementor (höchster Rang, voller Zugriff)" },
];

function AccountPicker({ onPick }: { onPick: (login: string) => void }) {
  const [open, setOpen] = useState(false);
  const [query, setQuery] = useState("");
  const [results, setResults] = useState<AccountSummary[]>([]);
  const [searching, setSearching] = useState(false);

  function runSearch() {
    setSearching(true);
    invoke<AccountSummary[]>("list_accounts", { search: query.trim(), limit: 20, offset: 0 })
      .then(setResults)
      .catch(() => setResults([]))
      .finally(() => setSearching(false));
  }

  return (
    <div className="flex flex-col gap-1">
      <Button type="button" variant="outline" size="sm" onClick={() => setOpen((o) => !o)}>
        <Search className="size-3.5" /> Account suchen…
      </Button>
      {open && (
        <div className="w-72 space-y-1 rounded-md border border-border bg-card p-2 shadow-md">
          <div className="flex gap-1">
            <input
              autoFocus
              value={query}
              onChange={(e) => setQuery(e.target.value)}
              onKeyDown={(e) => e.key === "Enter" && runSearch()}
              placeholder="Login…"
              className="flex-1 rounded-md border border-border bg-background px-2 py-1 text-xs"
            />
            <Button type="button" variant="outline" size="sm" onClick={runSearch} disabled={searching}>
              <Search className="size-3.5" />
            </Button>
          </div>
          <div className="max-h-40 space-y-0.5 overflow-y-auto">
            {results.map((a) => (
              <button
                key={a.id}
                type="button"
                onClick={() => {
                  onPick(a.login);
                  setOpen(false);
                }}
                className="block w-full rounded-md px-2 py-1 text-left text-xs hover:bg-muted"
              >
                {a.login}
              </button>
            ))}
            {results.length === 0 && !searching && (
              <p className="p-1 text-xs text-muted-foreground">Suchen, um Accounts zu finden.</p>
            )}
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

function NewGmDialog({
  hasColumn,
  onClose,
  onCreated,
}: {
  hasColumn: (name: string) => boolean;
  onClose: () => void;
  onCreated: () => void;
}) {
  const [account, setAccount] = useState("");
  const [name, setName] = useState("");
  const [authority, setAuthority] = useState("LOW_WIZARD");
  const [contactIp, setContactIp] = useState("");
  const [serverIp, setServerIp] = useState("ALL");
  const [busy, setBusy] = useState(false);
  const [error, setError] = useState<string | null>(null);

  const canCreate = account.trim().length > 0 && name.trim().length > 0;

  async function submit() {
    const values: [string, string | null][] = [];
    if (hasColumn("mAccount")) values.push(["mAccount", account.trim()]);
    if (hasColumn("mName")) values.push(["mName", name.trim()]);
    if (hasColumn("mAuthority")) values.push(["mAuthority", authority]);
    if (hasColumn("mContactIP")) values.push(["mContactIP", contactIp.trim() || ""]);
    if (hasColumn("mServerIP")) values.push(["mServerIP", serverIp.trim() || "ALL"]);

    await runAsyncAction(() => invoke("insert_table_row", { database: DB, table: TABLE, values }), {
      onStart: () => {
        setBusy(true);
        setError(null);
      },
      onSuccess: () => {
        logActivity("gm-manager", "create", `GM '${name}' (${authority}) für Account '${account}' angelegt`, "gmlist", name);
        onCreated();
        onClose();
      },
      onError: setError,
      onFinally: () => setBusy(false),
    });
  }

  return (
    <div className="fixed inset-0 z-50 flex items-center justify-center bg-black/50">
      <div className="w-96 space-y-3 rounded-lg border border-border bg-card p-4">
        <p className="text-sm font-medium">Neuer GM</p>
        <Field label="Account (login)">
          <input
            autoFocus
            value={account}
            onChange={(e) => setAccount(e.target.value)}
            className="rounded-md border border-border bg-background px-2 py-1 text-sm"
          />
        </Field>
        <AccountPicker onPick={setAccount} />
        <Field label="Charaktername (exakt wie im Spiel, Groß-/Kleinschreibung zählt)">
          <input
            value={name}
            onChange={(e) => setName(e.target.value)}
            className="rounded-md border border-border bg-background px-2 py-1 text-sm"
          />
        </Field>
        <Field label="Rang">
          <select
            value={authority}
            onChange={(e) => setAuthority(e.target.value)}
            className="rounded-md border border-border bg-background px-2 py-1 text-sm"
          >
            {AUTHORITY_LEVELS.map((l) => (
              <option key={l.value} value={l.value}>
                {l.label}
              </option>
            ))}
          </select>
        </Field>
        <Field label="Server-IP (ALL = auf allen Channels gültig)">
          <input
            value={serverIp}
            onChange={(e) => setServerIp(e.target.value)}
            className="rounded-md border border-border bg-background px-2 py-1 text-sm"
          />
        </Field>
        <Field label="Kontakt-IP (optional)">
          <input
            value={contactIp}
            onChange={(e) => setContactIp(e.target.value)}
            className="rounded-md border border-border bg-background px-2 py-1 text-sm"
          />
        </Field>
        {error && <p className="text-sm text-destructive">{error}</p>}
        <div className="flex justify-end gap-2">
          <Button variant="outline" onClick={onClose}>
            Abbrechen
          </Button>
          <Button disabled={!canCreate || busy} onClick={submit}>
            {busy ? "Lege an…" : "Anlegen"}
          </Button>
        </div>
      </div>
    </div>
  );
}

export function GmManager() {
  const [columns, setColumns] = useState<ColumnInfo[] | null>(null);
  const [schemaError, setSchemaError] = useState<string | null>(null);
  const [rows, setRows] = useState<TableRows | null>(null);
  const [loading, setLoading] = useState(false);
  const [loadError, setLoadError] = useState<string | null>(null);

  const [editingPk, setEditingPk] = useState<string | null>(null);
  const [creating, setCreating] = useState(false);
  const [deleteTarget, setDeleteTarget] = useState<{ pk: string; label: string } | null>(null);
  const [deleteError, setDeleteError] = useState<string | null>(null);

  const pkColumn = useMemo(() => columns?.find((c) => c.is_primary_key)?.name ?? null, [columns]);
  const hasColumn = (name: string) => columns?.some((c) => c.name === name) ?? false;
  const colIndex = useMemo(() => {
    const idx: Record<string, number> = {};
    rows?.columns.forEach((c, i) => (idx[c] = i));
    return idx;
  }, [rows]);

  useEffect(() => {
    loadSchema();
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, []);

  async function loadSchema() {
    await runAsyncAction(() => invoke<ColumnInfo[]>("get_table_columns", { database: DB, table: TABLE }), {
      onStart: () => setSchemaError(null),
      onSuccess: async (cols) => {
        setColumns(cols);
        const required = ["mAccount", "mName", "mAuthority"];
        const missing = required.filter((n) => !cols.some((c) => c.name === n));
        if (missing.length > 0) {
          setSchemaError(
            `common.gmlist hat nicht die aus dem Server-Quellcode erwarteten Spalten (fehlt: ${missing.join(", ")}) - dieses Modul wurde diese Session nicht live gegen diese Datenbank geprüft, nur gegen den Quellcode.`,
          );
          return;
        }
        await loadRows();
      },
      onError: setSchemaError,
    });
  }

  async function loadRows() {
    await runAsyncAction(() => invoke<TableRows>("get_table_rows", { database: DB, table: TABLE, limit: 500, offset: 0 }), {
      onStart: () => {
        setLoading(true);
        setLoadError(null);
      },
      onSuccess: setRows,
      onError: setLoadError,
      onFinally: () => setLoading(false),
    });
  }

  async function confirmDelete() {
    if (!deleteTarget || !pkColumn) return;
    await runAsyncAction(
      () => invoke("delete_table_row", { database: DB, table: TABLE, pkColumn, pkValue: deleteTarget.pk }),
      {
        onStart: () => setDeleteError(null),
        onSuccess: async () => {
          logActivity("gm-manager", "delete", `GM-Rechte für '${deleteTarget.label}' entzogen`, "gmlist", deleteTarget.pk);
          setDeleteTarget(null);
          await loadRows();
        },
        onError: setDeleteError,
      },
    );
  }

  return (
    <div className="max-w-4xl space-y-6 pb-10">
      <div className="flex items-center gap-2">
        <Shield className="size-6 text-muted-foreground" />
        <h1 className="text-2xl font-semibold">GM-Verwaltung</h1>
        <Button variant="ghost" size="icon-sm" title="Hilfe zu diesem Modul" onClick={() => openManual("gm-manager")}>
          <HelpCircle className="size-4" />
        </Button>
      </div>
      <p className="text-sm text-muted-foreground">
        Bearbeitet <code>common.gmlist</code> direkt - die Tabelle, aus der der DB-Kern beim Start bzw.
        bei <code>/reload a</code> die GM-Rechte lädt (<code>ClientManager.cpp::__GetAdminInfo</code>).
        Ein GM-Eintrag ist an einen konkreten Charakternamen gebunden, nicht nur an den Account.
      </p>
      <p className="flex items-start gap-2 rounded-lg border border-amber-500/40 bg-amber-500/10 p-3 text-sm text-amber-700 dark:text-amber-400">
        <Info className="mt-0.5 size-4 shrink-0" />
        <span>
          Wirkt nach dem Ingame-Befehl <code>/reload a</code> (kein Server-Neustart nötig,
          quellcode-verifiziert: <code>cmd_gm.cpp</code> löst darüber ein <code>HEADER_GD_RELOAD_ADMIN</code>{" "}
          zum DB-Kern aus). Der Rang wird beim Login rein über Charaktername + Account abgeglichen
          (<code>gm_new_get_level</code> in <code>gm.cpp</code>) - Server-IP wird vom DB-Kern bereits per
          SQL gefiltert (nur Einträge mit <code>ALL</code> oder der passenden Channel-IP werden geladen),
          Kontakt-IP wird zwar gespeichert/geloggt, aber in der geprüften Rang-Prüfung selbst nicht
          ausgewertet.
        </span>
      </p>

      {schemaError && (
        <p className="flex items-start gap-2 rounded-lg border border-destructive/40 bg-destructive/10 p-3 text-sm text-destructive">
          <AlertTriangle className="mt-0.5 size-4 shrink-0" />
          {schemaError}
        </p>
      )}

      {!schemaError && (
        <>
          <div className="flex items-center gap-2">
            <Button variant="outline" onClick={loadRows} disabled={loading}>
              <RefreshCw className={`size-4 ${loading ? "animate-spin" : ""}`} /> Neu laden
            </Button>
            <Button onClick={() => setCreating(true)} disabled={!columns}>
              <Plus className="size-4" /> Neuer GM…
            </Button>
          </div>
          {loadError && <p className="text-sm text-destructive">{loadError}</p>}

          {rows && (
            <div className="overflow-x-auto rounded-md border border-border">
              <table className="w-full text-xs">
                <thead className="bg-muted/40">
                  <tr>
                    <th className="px-2 py-1 text-left font-medium">Account</th>
                    <th className="px-2 py-1 text-left font-medium">Charakter</th>
                    <th className="px-2 py-1 text-left font-medium">Rang</th>
                    <th className="px-2 py-1 text-left font-medium">Server-IP</th>
                    <th className="px-2 py-1 text-left font-medium">Kontakt-IP</th>
                    <th />
                  </tr>
                </thead>
                <tbody>
                  {rows.rows.map((r, i) => {
                    const pkValue = pkColumn ? r[colIndex[pkColumn]] ?? "" : "";
                    const name = r[colIndex["mName"]] ?? "";
                    return (
                      <tr key={i} className="border-t border-border">
                        <td className="px-2 py-1">{r[colIndex["mAccount"]]}</td>
                        <td className="px-2 py-1 font-medium">{name}</td>
                        <td className="px-2 py-1">{r[colIndex["mAuthority"]]}</td>
                        <td className="px-2 py-1">{hasColumn("mServerIP") ? r[colIndex["mServerIP"]] : "—"}</td>
                        <td className="px-2 py-1">{hasColumn("mContactIP") ? r[colIndex["mContactIP"]] || "—" : "—"}</td>
                        <td className="whitespace-nowrap px-2 py-1 text-right">
                          <Button size="sm" variant="outline" onClick={() => setEditingPk(pkValue)} disabled={!pkColumn}>
                            Bearbeiten
                          </Button>{" "}
                          <Button
                            size="sm"
                            variant="outline"
                            onClick={() => setDeleteTarget({ pk: pkValue, label: String(name) })}
                            disabled={!pkColumn}
                          >
                            <Trash2 className="size-3.5" />
                          </Button>
                        </td>
                      </tr>
                    );
                  })}
                  {rows.rows.length === 0 && (
                    <tr>
                      <td colSpan={6} className="px-2 py-3 text-center text-muted-foreground">
                        Keine GM-Einträge.
                      </td>
                    </tr>
                  )}
                </tbody>
              </table>
            </div>
          )}
        </>
      )}

      {creating && columns && (
        <NewGmDialog hasColumn={hasColumn} onClose={() => setCreating(false)} onCreated={loadRows} />
      )}

      {editingPk !== null && pkColumn && (
        <GenericRowEditor
          database={DB}
          table={TABLE}
          pkValue={editingPk}
          onClose={() => setEditingPk(null)}
          onSaved={loadRows}
          activityModule="gm-manager"
        />
      )}

      {deleteTarget && (
        <div className="fixed inset-0 z-50 flex items-center justify-center bg-black/50">
          <div className="w-96 space-y-3 rounded-lg border border-border bg-card p-4">
            <p className="flex items-center gap-2 text-sm font-medium">
              <AlertTriangle className="size-4 text-destructive" /> GM-Rechte für "{deleteTarget.label}" wirklich
              entziehen?
            </p>
            {deleteError && <p className="text-sm text-destructive">{deleteError}</p>}
            <div className="flex justify-end gap-2">
              <Button variant="outline" onClick={() => setDeleteTarget(null)}>
                Abbrechen
              </Button>
              <Button variant="destructive" onClick={confirmDelete}>
                Entziehen
              </Button>
            </div>
          </div>
        </div>
      )}
    </div>
  );
}
