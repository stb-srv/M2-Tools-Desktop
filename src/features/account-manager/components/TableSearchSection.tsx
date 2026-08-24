import { useState } from "react";
import { invoke } from "@tauri-apps/api/core";
import { runAsyncAction } from "@/lib/asyncAction";
import { Button } from "@/components/ui/button";
import { Search, AlertTriangle, Wrench } from "lucide-react";
import { GenericRowEditor } from "@/features/shared/GenericRowEditor";
import type { ColumnInfo, TableRows, TableTarget } from "../shared";
import { PlayerToolsDialog } from "./PlayerToolsDialog";

export function TableSearchSection({
  title,
  target,
  defaultColumn,
  placeholder,
  playerTools,
}: {
  title: string;
  target: TableTarget;
  defaultColumn: string;
  placeholder: string;
  /** Zeigt einen "Werkzeuge"-Button (Yang gutschreiben, Position setzen) je
   * Treffer - nur sinnvoll, wenn `target` player.player ist. */
  playerTools?: boolean;
}) {
  const [column, setColumn] = useState(defaultColumn);
  const [query, setQuery] = useState("");
  const [result, setResult] = useState<TableRows | null>(null);
  const [searching, setSearching] = useState(false);
  const [searchError, setSearchError] = useState<string | null>(null);
  const [editingPk, setEditingPk] = useState<string | null>(null);
  const [toolsTarget, setToolsTarget] = useState<{ pk: string; label: string } | null>(null);

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

  async function resolvePk(row: (string | null)[]): Promise<string> {
    if (!result) throw new Error("Kein Suchergebnis.");
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
  }

  async function pickRow(row: (string | null)[]) {
    await runAsyncAction(() => resolvePk(row), { onSuccess: setEditingPk, onError: setSearchError });
  }

  async function openTools(row: (string | null)[]) {
    if (!result) return;
    const nameIndex = result.columns.indexOf("name");
    const label = nameIndex >= 0 ? (row[nameIndex] ?? "") : "";
    await runAsyncAction(() => resolvePk(row), {
      onSuccess: (pk) => setToolsTarget({ pk, label }),
      onError: setSearchError,
    });
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
                  <td className="whitespace-nowrap px-2 py-1 text-right">
                    <Button size="sm" variant="outline" onClick={() => pickRow(row)}>
                      Bearbeiten
                    </Button>{" "}
                    {playerTools && (
                      <Button size="sm" variant="outline" onClick={() => openTools(row)}>
                        <Wrench className="size-3.5" />
                        Werkzeuge
                      </Button>
                    )}
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
          activityModule="account-manager"
        />
      )}

      {toolsTarget && (
        <PlayerToolsDialog
          playerId={toolsTarget.pk}
          playerLabel={toolsTarget.label}
          onClose={() => setToolsTarget(null)}
        />
      )}
    </section>
  );
}
