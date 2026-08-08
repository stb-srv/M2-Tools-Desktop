import { useEffect, useState } from "react";
import { useTranslation } from "react-i18next";
import { invoke } from "@tauri-apps/api/core";
import { Button } from "@/components/ui/button";
import { Search, ChevronLeft, ChevronRight, Key, HelpCircle } from "lucide-react";
import { openManual } from "@/lib/manual";

interface TableInfo {
  name: string;
  approx_rows: number;
}

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

const PAGE_SIZE = 50;

export function DbExplorer() {
  const { t } = useTranslation();

  const [connected, setConnected] = useState<boolean | null>(null);
  const [databases, setDatabases] = useState<string[]>([]);
  const [selectedDb, setSelectedDb] = useState<string | null>(null);
  const [tables, setTables] = useState<TableInfo[]>([]);
  const [tableFilter, setTableFilter] = useState("");
  const [selectedTable, setSelectedTable] = useState<string | null>(null);

  const [columns, setColumns] = useState<ColumnInfo[]>([]);
  const [data, setData] = useState<TableRows | null>(null);
  const [page, setPage] = useState(0);
  const [loading, setLoading] = useState(false);
  const [error, setError] = useState<string | null>(null);

  const [searchColumn, setSearchColumn] = useState<string | null>(null);
  const [searchQuery, setSearchQuery] = useState("");
  const [searching, setSearching] = useState(false);

  useEffect(() => {
    invoke<boolean>("is_mysql_connected")
      .then(async (ok) => {
        setConnected(ok);
        if (ok) {
          const dbs = await invoke<string[]>("list_databases");
          setDatabases(dbs);
        }
      })
      .catch(() => setConnected(false));
  }, []);

  useEffect(() => {
    if (!selectedDb) return;
    setSelectedTable(null);
    setData(null);
    invoke<TableInfo[]>("list_tables", { database: selectedDb })
      .then(setTables)
      .catch((e) => setError(String(e)));
  }, [selectedDb]);

  useEffect(() => {
    if (!selectedDb || !selectedTable) return;
    setPage(0);
    setSearchQuery("");
    invoke<ColumnInfo[]>("get_table_columns", {
      database: selectedDb,
      table: selectedTable,
    })
      .then((cols) => {
        setColumns(cols);
        setSearchColumn(cols.find((c) => c.is_primary_key)?.name ?? cols[0]?.name ?? null);
      })
      .catch((e) => setError(String(e)));
  }, [selectedDb, selectedTable]);

  useEffect(() => {
    if (!selectedDb || !selectedTable) return;
    loadPage(page);
  }, [selectedDb, selectedTable, page]);

  async function loadPage(p: number) {
    if (!selectedDb || !selectedTable) return;
    setLoading(true);
    setError(null);
    try {
      const result = await invoke<TableRows>("get_table_rows", {
        database: selectedDb,
        table: selectedTable,
        limit: PAGE_SIZE,
        offset: p * PAGE_SIZE,
      });
      setData(result);
    } catch (e) {
      setError(String(e));
    } finally {
      setLoading(false);
    }
  }

  async function runSearch() {
    if (!selectedDb || !selectedTable || !searchColumn) return;
    if (!searchQuery.trim()) {
      setPage(0);
      loadPage(0);
      return;
    }
    setSearching(true);
    setError(null);
    try {
      const result = await invoke<TableRows>("search_table_rows", {
        database: selectedDb,
        table: selectedTable,
        column: searchColumn,
        query: searchQuery.trim(),
      });
      setData(result);
    } catch (e) {
      setError(String(e));
    } finally {
      setSearching(false);
    }
  }

  if (connected === false) {
    return (
      <div className="space-y-2">
        <div className="flex items-center gap-2">
          <h1 className="text-2xl font-semibold">{t("nav.dbExplorer")}</h1>
          <Button variant="ghost" size="icon-sm" title="Hilfe zu diesem Modul" onClick={() => openManual("db-explorer")}>
            <HelpCircle className="size-4" />
          </Button>
        </div>
        <p className="text-sm text-muted-foreground">
          Keine aktive MySQL-Verbindung. Bitte unter Verbindungen einrichten.
        </p>
      </div>
    );
  }

  const filteredTables = tables.filter((tbl) =>
    tbl.name.toLowerCase().includes(tableFilter.toLowerCase()),
  );
  const totalPages = data ? Math.max(1, Math.ceil(data.total_rows / PAGE_SIZE)) : 1;
  const isSearchResult = data && searchQuery.trim() && data.total_rows === data.rows.length;

  return (
    <div className="flex h-full gap-4">
      <div className="w-48 shrink-0 space-y-2">
        <h2 className="text-xs font-medium text-muted-foreground">Datenbank</h2>
        <div className="space-y-1">
          {databases.map((db) => (
            <button
              key={db}
              onClick={() => setSelectedDb(db)}
              className={`w-full rounded-md px-2 py-1 text-left text-sm hover:bg-muted ${
                selectedDb === db ? "bg-muted font-medium" : ""
              }`}
            >
              {db}
            </button>
          ))}
        </div>
      </div>

      <div className="w-64 shrink-0 space-y-2">
        <h2 className="text-xs font-medium text-muted-foreground">Tabelle</h2>
        {selectedDb && (
          <input
            value={tableFilter}
            onChange={(e) => setTableFilter(e.target.value)}
            placeholder="Filtern…"
            className="w-full rounded-md border border-border bg-background px-2 py-1 text-sm"
          />
        )}
        <div className="max-h-full space-y-1 overflow-y-auto">
          {filteredTables.map((tbl) => (
            <button
              key={tbl.name}
              onClick={() => setSelectedTable(tbl.name)}
              className={`flex w-full items-center justify-between rounded-md px-2 py-1 text-left text-sm hover:bg-muted ${
                selectedTable === tbl.name ? "bg-muted font-medium" : ""
              }`}
            >
              <span className="truncate">{tbl.name}</span>
              <span className="text-xs text-muted-foreground">
                {tbl.approx_rows.toLocaleString("de-DE")}
              </span>
            </button>
          ))}
        </div>
      </div>

      <div className="min-w-0 flex-1 space-y-2">
        {error && <p className="text-sm text-destructive">{error}</p>}
        {!selectedTable && (
          <p className="text-sm text-muted-foreground">
            Datenbank und Tabelle links auswählen.
          </p>
        )}

        {selectedTable && (
          <>
            <div className="flex items-center justify-between">
              <h1 className="text-lg font-semibold">
                {selectedDb}.{selectedTable}
              </h1>
              <div className="flex items-center gap-2">
                <select
                  value={searchColumn ?? ""}
                  onChange={(e) => setSearchColumn(e.target.value)}
                  className="rounded-md border border-border bg-background px-2 py-1 text-sm"
                >
                  {columns.map((c) => (
                    <option key={c.name} value={c.name}>
                      {c.name}
                    </option>
                  ))}
                </select>
                <input
                  value={searchQuery}
                  onChange={(e) => setSearchQuery(e.target.value)}
                  onKeyDown={(e) => e.key === "Enter" && runSearch()}
                  placeholder="Suchen…"
                  className="rounded-md border border-border bg-background px-2 py-1 text-sm"
                />
                <Button variant="outline" size="sm" onClick={runSearch} disabled={searching}>
                  <Search className="size-4" />
                </Button>
              </div>
            </div>

            <div className="overflow-auto rounded-lg border border-border">
              <table className="w-full text-xs">
                <thead className="sticky top-0 bg-card">
                  <tr>
                    {(data?.columns ?? columns.map((c) => c.name)).map((col) => {
                      const isPk = columns.find((c) => c.name === col)?.is_primary_key;
                      return (
                        <th
                          key={col}
                          className="whitespace-nowrap border-b border-border px-2 py-1 text-left font-medium"
                        >
                          <span className="flex items-center gap-1">
                            {isPk && <Key className="size-3 text-muted-foreground" />}
                            {col}
                          </span>
                        </th>
                      );
                    })}
                  </tr>
                </thead>
                <tbody>
                  {data?.rows.map((row, i) => (
                    <tr key={i} className="hover:bg-muted/50">
                      {row.map((cell, j) => (
                        <td
                          key={j}
                          className="max-w-[200px] truncate whitespace-nowrap border-b border-border px-2 py-1"
                          title={cell ?? undefined}
                        >
                          {cell === null ? (
                            <span className="text-muted-foreground italic">NULL</span>
                          ) : (
                            cell
                          )}
                        </td>
                      ))}
                    </tr>
                  ))}
                </tbody>
              </table>
              {loading && <p className="p-2 text-sm text-muted-foreground">Lädt…</p>}
              {data && data.rows.length === 0 && (
                <p className="p-2 text-sm text-muted-foreground">Keine Zeilen.</p>
              )}
            </div>

            {!isSearchResult && (
              <div className="flex items-center justify-between text-sm">
                <span className="text-muted-foreground">
                  {data?.total_rows.toLocaleString("de-DE")} Zeilen · Seite {page + 1}/
                  {totalPages}
                </span>
                <div className="flex gap-1">
                  <Button
                    variant="outline"
                    size="icon-sm"
                    onClick={() => setPage((p) => Math.max(0, p - 1))}
                    disabled={page === 0}
                  >
                    <ChevronLeft className="size-4" />
                  </Button>
                  <Button
                    variant="outline"
                    size="icon-sm"
                    onClick={() => setPage((p) => Math.min(totalPages - 1, p + 1))}
                    disabled={page >= totalPages - 1}
                  >
                    <ChevronRight className="size-4" />
                  </Button>
                </div>
              </div>
            )}
          </>
        )}
      </div>
    </div>
  );
}
