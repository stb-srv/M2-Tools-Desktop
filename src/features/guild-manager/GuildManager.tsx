import { useState } from "react";
import { invoke } from "@tauri-apps/api/core";
import { Castle, Search, AlertTriangle, Swords, Trash2, HelpCircle } from "lucide-react";
import { Button } from "@/components/ui/button";
import { runAsyncAction } from "@/lib/asyncAction";
import { logActivity } from "@/lib/logActivity";
import { openManual } from "@/lib/manual";
import { GenericRowEditor } from "@/features/shared/GenericRowEditor";

interface TableRows {
  columns: string[];
  rows: (string | null)[][];
  total_rows: number;
}

// Reale Tabellennamen aus source/db/src/GuildManager.cpp verifiziert (guild:
// id/name/ladder_point/win/draw/loss/gold/level/master, guild_war_reservation/
// guild_war_bet für Kriege) - siehe src-tauri/src/db/guild.rs für den
// verifizierten Auflösen-Kaskade-Code. Welche Datenbank diese Tabellen
// enthält wurde NICHT live geprüft (Annahme: "player", wie player.player/
// item_proto/shop) - deshalb hier bewusst editierbar statt hart codiert.
const DEFAULT_DATABASE = "player";
const GUILD_TABLE = "guild";
const WAR_TABLE = "guild_war_reservation";

function findColumn(rows: TableRows, name: string): number {
  return rows.columns.indexOf(name);
}

function cell(rows: TableRows, row: (string | null)[], name: string): string | null {
  const i = findColumn(rows, name);
  return i >= 0 ? row[i] : null;
}

export function GuildManager() {
  const [database, setDatabase] = useState(DEFAULT_DATABASE);
  const [query, setQuery] = useState("");
  const [result, setResult] = useState<TableRows | null>(null);
  const [searching, setSearching] = useState(false);
  const [searchError, setSearchError] = useState<string | null>(null);

  const [editingId, setEditingId] = useState<string | null>(null);
  const [warsFor, setWarsFor] = useState<{ id: string; name: string; rows: TableRows } | null>(null);
  const [warsLoading, setWarsLoading] = useState(false);
  const [warsError, setWarsError] = useState<string | null>(null);
  const [disbandTarget, setDisbandTarget] = useState<{ id: string; name: string } | null>(null);
  const [disbanding, setDisbanding] = useState(false);
  const [disbandError, setDisbandError] = useState<string | null>(null);

  async function runSearch() {
    await runAsyncAction(
      () =>
        invoke<TableRows>("search_table_rows", {
          database,
          table: GUILD_TABLE,
          column: "name",
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

  async function showWars(id: string, name: string) {
    setWarsError(null);
    setWarsLoading(true);
    setWarsFor(null);
    try {
      const [byGuild1, byGuild2] = await Promise.all([
        invoke<TableRows>("search_table_rows", { database, table: WAR_TABLE, column: "guild1", query: id }),
        invoke<TableRows>("search_table_rows", { database, table: WAR_TABLE, column: "guild2", query: id }),
      ]);
      const idCol = findColumn(byGuild1, "id");
      const seen = new Set<string>();
      const merged = [...byGuild1.rows, ...byGuild2.rows].filter((row) => {
        const key = idCol >= 0 ? (row[idCol] ?? "") : JSON.stringify(row);
        if (seen.has(key)) return false;
        seen.add(key);
        return true;
      });
      setWarsFor({ id, name, rows: { columns: byGuild1.columns, rows: merged, total_rows: merged.length } });
    } catch (e) {
      setWarsError(String(e));
    } finally {
      setWarsLoading(false);
    }
  }

  async function confirmDisband() {
    if (!disbandTarget) return;
    await runAsyncAction(
      () => invoke("disband_guild", { database, guildId: Number(disbandTarget.id) }),
      {
        onStart: () => {
          setDisbanding(true);
          setDisbandError(null);
        },
        onSuccess: () => {
          logActivity(
            "guild-manager",
            "delete",
            `Gilde "${disbandTarget.name}" (${disbandTarget.id}) aufgelöst`,
            GUILD_TABLE,
            disbandTarget.id,
          );
          setDisbandTarget(null);
          setResult((prev) =>
            prev
              ? { ...prev, rows: prev.rows.filter((r) => cell(prev, r, "id") !== disbandTarget.id) }
              : prev,
          );
        },
        onError: setDisbandError,
        onFinally: () => setDisbanding(false),
      },
    );
  }

  return (
    <div className="max-w-4xl space-y-4">
      <div className="flex items-center gap-2">
        <Castle className="size-6 text-muted-foreground" />
        <h1 className="text-2xl font-semibold">Gilden-Verwaltung</h1>
        <Button variant="ghost" size="icon-sm" title="Hilfe zu diesem Modul" onClick={() => openManual("guild-manager")}>
          <HelpCircle className="size-4" />
        </Button>
      </div>
      <p className="text-sm text-muted-foreground">
        Bearbeitet die Gilden-Tabellen direkt in der Datenbank (generisch, wie Mob-Proto-Editor/
        Account-Verwaltung - jede Spalte roh, ohne geratene Bedeutung). "Auflösen" führt exakt die
        Kaskade des echten Servers aus (<code>source/db/src/ClientManagerGuild.cpp::GuildDisband</code>
        , verifiziert): löscht die Gilde, ihre Rangstufen, Mitglieder und Kommentare, und setzt für
        jedes ehemalige Mitglied dieselbe Beitritts-Sperre-Quest-Flag wie der echte Server. Gildenkriege
        werden dabei bewusst nicht angefasst (der echte Server räumt die beim Auflösen ebenfalls nicht auf).
      </p>

      <div className="flex items-center gap-2 text-xs text-muted-foreground">
        <span>Datenbank:</span>
        <input
          value={database}
          onChange={(e) => setDatabase(e.target.value)}
          title="Welche Datenbank die guild-Tabellen enthält - nicht live gegen den echten Server geprüft, Standard 'player' wie player.player/item_proto/shop"
          className="w-28 rounded-md border border-border bg-background px-2 py-1"
        />
      </div>

      <section className="space-y-2 rounded-lg border border-border p-4">
        <h2 className="text-sm font-medium text-muted-foreground">Gilde suchen</h2>
        <div className="flex gap-2">
          <input
            value={query}
            onChange={(e) => setQuery(e.target.value)}
            onKeyDown={(e) => e.key === "Enter" && runSearch()}
            placeholder="Gildenname (leer = alle)"
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
                  {result.columns.map((c) => (
                    <th key={c} className="px-2 py-1 text-left font-medium">
                      {c}
                    </th>
                  ))}
                  <th />
                </tr>
              </thead>
              <tbody>
                {result.rows.map((row, i) => {
                  const id = cell(result, row, "id") ?? "";
                  const name = cell(result, row, "name") ?? id;
                  return (
                    <tr key={i} className="border-t border-border">
                      {row.map((c, j) => (
                        <td key={j} className="max-w-32 truncate px-2 py-1">
                          {c ?? <span className="text-muted-foreground">NULL</span>}
                        </td>
                      ))}
                      <td className="whitespace-nowrap px-2 py-1 text-right">
                        <Button size="sm" variant="outline" onClick={() => setEditingId(id)}>
                          Bearbeiten
                        </Button>{" "}
                        <Button size="sm" variant="outline" onClick={() => showWars(id, name)}>
                          <Swords className="size-3.5" />
                        </Button>{" "}
                        <Button size="sm" variant="destructive" onClick={() => setDisbandTarget({ id, name })}>
                          <Trash2 className="size-3.5" />
                        </Button>
                      </td>
                    </tr>
                  );
                })}
              </tbody>
            </table>
          </div>
        )}
        {result && result.rows.length === 0 && <p className="text-sm text-muted-foreground">Keine Treffer.</p>}
      </section>

      {editingId !== null && (
        <GenericRowEditor
          database={database}
          table={GUILD_TABLE}
          pkValue={editingId}
          onClose={() => setEditingId(null)}
          activityModule="guild-manager"
        />
      )}

      {(warsLoading || warsFor || warsError) && (
        <div className="fixed inset-0 z-40 flex items-center justify-center bg-black/50">
          <div className="max-h-[70vh] w-[36rem] space-y-3 overflow-y-auto rounded-lg border border-border bg-card p-4">
            <div className="flex items-center justify-between">
              <p className="text-sm font-medium">Gildenkriege{warsFor && ` — ${warsFor.name}`}</p>
              <Button variant="ghost" size="sm" onClick={() => { setWarsFor(null); setWarsError(null); }}>
                Schließen
              </Button>
            </div>
            {warsLoading && <p className="text-sm text-muted-foreground">Lade…</p>}
            {warsError && (
              <p className="flex items-start gap-2 text-sm text-destructive">
                <AlertTriangle className="mt-0.5 size-4 shrink-0" />
                <span>{warsError}</span>
              </p>
            )}
            {warsFor && warsFor.rows.rows.length === 0 && (
              <p className="text-sm text-muted-foreground">Keine Kriege gefunden.</p>
            )}
            {warsFor && warsFor.rows.rows.length > 0 && (
              <table className="w-full text-xs">
                <thead className="bg-muted/40">
                  <tr>
                    {warsFor.rows.columns.map((c) => (
                      <th key={c} className="px-2 py-1 text-left font-medium">
                        {c}
                      </th>
                    ))}
                  </tr>
                </thead>
                <tbody>
                  {warsFor.rows.rows.map((row, i) => (
                    <tr key={i} className="border-t border-border">
                      {row.map((c, j) => (
                        <td key={j} className="max-w-32 truncate px-2 py-1">
                          {c ?? <span className="text-muted-foreground">NULL</span>}
                        </td>
                      ))}
                    </tr>
                  ))}
                </tbody>
              </table>
            )}
          </div>
        </div>
      )}

      {disbandTarget && (
        <div className="fixed inset-0 z-50 flex items-center justify-center bg-black/50">
          <div className="w-96 space-y-3 rounded-lg border border-border bg-card p-4">
            <p className="flex items-start gap-2 text-sm">
              <AlertTriangle className="mt-0.5 size-4 shrink-0 text-destructive" />
              <span>
                Gilde <strong>{disbandTarget.name}</strong> ({disbandTarget.id}) wirklich auflösen? Löscht die
                Gilde, ihre Rangstufen, Mitglieder und Kommentare unwiderruflich in der Live-Datenbank, ohne
                Backup (DB-Zeilen, keine Datei).
              </span>
            </p>
            {disbandError && <p className="text-sm text-destructive">{disbandError}</p>}
            <div className="flex justify-end gap-2">
              <Button variant="outline" onClick={() => setDisbandTarget(null)} disabled={disbanding}>
                Abbrechen
              </Button>
              <Button variant="destructive" onClick={confirmDisband} disabled={disbanding}>
                {disbanding ? "Löse auf…" : "Auflösen"}
              </Button>
            </div>
          </div>
        </div>
      )}
    </div>
  );
}
