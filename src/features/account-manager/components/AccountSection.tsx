import { useEffect, useState } from "react";
import { invoke } from "@tauri-apps/api/core";
import { runAsyncAction } from "@/lib/asyncAction";
import { Button } from "@/components/ui/button";
import {
  Search,
  AlertTriangle,
  Plus,
  KeyRound,
  ChevronLeft,
  ChevronRight,
  Lock,
  LockOpen,
  Coins,
} from "lucide-react";
import { GenericRowEditor } from "@/features/shared/GenericRowEditor";
import type { AccountSummary, BanRecord, ColumnInfo } from "../shared";
import { ResetPasswordDialog } from "./ResetPasswordDialog";
import { BanDialog } from "./BanDialog";
import { CurrencyDialog } from "./CurrencyDialog";

const CURRENCY_EXCLUDED_COLUMNS = new Set(["id", "empire"]);
const NUMERIC_TYPE_PREFIXES = ["int", "tinyint", "smallint", "mediumint", "bigint", "decimal", "float", "double"];

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
export function AccountSection() {
  const [accounts, setAccounts] = useState<AccountSummary[] | null>(null);
  const [total, setTotal] = useState(0);
  const [page, setPage] = useState(0);
  const [search, setSearch] = useState("");
  const [loading, setLoading] = useState(false);
  const [loadError, setLoadError] = useState<string | null>(null);

  const [editingPk, setEditingPk] = useState<string | null>(null);
  const [resetTarget, setResetTarget] = useState<AccountSummary | null>(null);

  const [bans, setBans] = useState<BanRecord[]>([]);
  const [banTarget, setBanTarget] = useState<AccountSummary | null>(null);
  const [currencyColumns, setCurrencyColumns] = useState<ColumnInfo[]>([]);
  const [currencyTarget, setCurrencyTarget] = useState<AccountSummary | null>(null);

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

  async function loadBans() {
    await runAsyncAction(() => invoke<BanRecord[]>("list_account_bans"), {
      onSuccess: setBans,
      onError: setLoadError,
    });
  }

  // Einmalig beim Öffnen: fällige Sperren automatisch aufheben (kein
  // Server-Cron, siehe bans.rs) und die reale Spaltenliste von
  // account.account holen - falls eine Zusatzwährungs-Spalte (z.B.
  // "Drachenmünzen") existiert, taucht sie hier auf, sonst bleibt die
  // Guthaben-Aktion pro Account unsichtbar statt eine geratene Spalte zu
  // zeigen.
  useEffect(() => {
    (async () => {
      await runAsyncAction(() => invoke<number>("process_due_bans"), { onError: () => {} });
      await loadBans();
      await runAsyncAction(
        () => invoke<ColumnInfo[]>("get_table_columns", { database: "account", table: "account" }),
        {
          onSuccess: (cols) =>
            setCurrencyColumns(
              cols.filter(
                (c) =>
                  !c.is_primary_key &&
                  !CURRENCY_EXCLUDED_COLUMNS.has(c.name) &&
                  NUMERIC_TYPE_PREFIXES.some((p) => c.data_type.toLowerCase().startsWith(p)),
              ),
            ),
          onError: () => {},
        },
      );
    })();
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, []);

  function runSearch() {
    setPage(0);
    load();
  }

  async function unban(account: AccountSummary) {
    const activeBan = bans.find((b) => b.account_id === account.id && b.active);
    await runAsyncAction(() => invoke("unban_account", { accountId: account.id, banId: activeBan?.id ?? null }), {
      onSuccess: async () => {
        await Promise.all([load(), loadBans()]);
      },
      onError: setLoadError,
    });
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

      {bans.some((b) => b.active) && (
        <div className="space-y-1 rounded-md border border-amber-500/40 bg-amber-500/10 p-2">
          <p className="text-xs font-medium text-amber-700 dark:text-amber-400">Aktive Sperren</p>
          {bans
            .filter((b) => b.active)
            .map((b) => (
              <div key={b.id} className="flex items-center justify-between gap-2 text-xs">
                <span>
                  <span className="font-medium">{b.login}</span> — {b.reason}
                  {" · "}
                  {b.unban_at ? `automatische Entsperrung: ${new Date(b.unban_at).toLocaleString("de-DE")}` : "dauerhaft"}
                </span>
                <Button
                  size="sm"
                  variant="outline"
                  onClick={() =>
                    runAsyncAction(() => invoke("unban_account", { accountId: b.account_id, banId: b.id }), {
                      onSuccess: async () => {
                        await Promise.all([load(), loadBans()]);
                      },
                      onError: setLoadError,
                    })
                  }
                >
                  <LockOpen className="size-3.5" />
                  Jetzt entsperren
                </Button>
              </div>
            ))}
        </div>
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
                    </Button>{" "}
                    {a.status === "OK" ? (
                      <Button size="sm" variant="outline" onClick={() => setBanTarget(a)}>
                        <Lock className="size-3.5" />
                        Sperren
                      </Button>
                    ) : (
                      <Button size="sm" variant="outline" onClick={() => unban(a)}>
                        <LockOpen className="size-3.5" />
                        Entsperren
                      </Button>
                    )}{" "}
                    {currencyColumns.length > 0 && (
                      <Button size="sm" variant="outline" onClick={() => setCurrencyTarget(a)}>
                        <Coins className="size-3.5" />
                        Guthaben
                      </Button>
                    )}
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

      {banTarget && (
        <BanDialog
          account={banTarget}
          onClose={() => setBanTarget(null)}
          onBanned={async () => {
            setBanTarget(null);
            await Promise.all([load(), loadBans()]);
          }}
        />
      )}

      {currencyTarget && (
        <CurrencyDialog
          account={currencyTarget}
          columns={currencyColumns}
          onClose={() => setCurrencyTarget(null)}
        />
      )}

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
