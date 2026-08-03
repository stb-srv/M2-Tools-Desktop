import { useEffect, useMemo, useState } from "react";
import { invoke } from "@tauri-apps/api/core";
import { runAsyncAction } from "@/lib/asyncAction";
import { Button } from "@/components/ui/button";
import { AlertTriangle, CheckCircle2, Undo2 } from "lucide-react";

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

const NUMERIC_TYPE_PREFIXES = ["int", "tinyint", "smallint", "mediumint", "bigint", "decimal", "float", "double"];

function isNumericType(dataType: string): boolean {
  return NUMERIC_TYPE_PREFIXES.some((p) => dataType.toLowerCase().startsWith(p));
}

/**
 * Generic, schema-agnostic row editor built directly on the same
 * introspection the Database Explorer uses (`get_table_columns` /
 * `get_table_row` / `update_table_row`). Used where this app doesn't have a
 * verified, purpose-built schema for a table (mob_proto, account, player) -
 * rather than guessing column semantics, it shows every real column with a
 * type-appropriate input and auto-detects the primary key column from
 * `is_primary_key` instead of assuming a column name like "id" or "vnum".
 *
 * Caveat shown to the user: binary/varbinary columns are round-tripped via
 * the same Windows-1252 encoding the rest of the app uses for those columns
 * (see db/explorer.rs), but a column that's plain UTF-8 despite an
 * ambiguous type would not be an issue this component can detect.
 */
export function GenericRowEditor({
  database,
  table,
  pkValue,
  onClose,
  onSaved,
}: {
  database: string;
  table: string;
  pkValue: string;
  onClose: () => void;
  onSaved?: () => void;
}) {
  const [columns, setColumns] = useState<ColumnInfo[] | null>(null);
  const [loadError, setLoadError] = useState<string | null>(null);
  const [values, setValues] = useState<Record<string, string | null>>({});
  const [original, setOriginal] = useState<Record<string, string | null>>({});
  const [lastSaved, setLastSaved] = useState<Record<string, string | null> | null>(null);

  const [saving, setSaving] = useState(false);
  const [saveError, setSaveError] = useState<string | null>(null);
  const [saveOk, setSaveOk] = useState(false);
  const [confirmOpen, setConfirmOpen] = useState(false);

  const pkColumn = useMemo(
    () => columns?.filter((c) => c.is_primary_key).map((c) => c.name) ?? [],
    [columns],
  );

  useEffect(() => {
    load();
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, [database, table, pkValue]);

  async function load() {
    setColumns(null);
    setLoadError(null);
    setSaveOk(false);
    setSaveError(null);
    setLastSaved(null);
    await runAsyncAction(
      async () => {
        const cols = await invoke<ColumnInfo[]>("get_table_columns", { database, table });
        const pk = cols.filter((c) => c.is_primary_key);
        if (pk.length !== 1) {
          throw new Error(
            pk.length === 0
              ? "Diese Tabelle hat keinen Primärschlüssel - generischer Editor nicht möglich."
              : "Diese Tabelle hat einen zusammengesetzten Primärschlüssel - generischer Editor unterstützt das nicht.",
          );
        }
        const row = await invoke<TableRows | null>("get_table_row", {
          database,
          table,
          pkColumn: pk[0].name,
          pkValue,
        });
        if (!row) throw new Error("Zeile nicht gefunden.");
        const rowValues: Record<string, string | null> = {};
        row.columns.forEach((name, i) => (rowValues[name] = row.rows[0][i]));
        return { cols, rowValues };
      },
      {
        onSuccess: ({ cols, rowValues }) => {
          setColumns(cols);
          setValues(rowValues);
          setOriginal(rowValues);
        },
        onError: setLoadError,
      },
    );
  }

  function setField(name: string, value: string) {
    setValues((prev) => ({ ...prev, [name]: value }));
    setSaveOk(false);
  }

  const changedColumns = useMemo(
    () => Object.keys(values).filter((k) => values[k] !== original[k]),
    [values, original],
  );

  async function save() {
    if (pkColumn.length !== 1 || changedColumns.length === 0) return;
    setConfirmOpen(false);
    const changes = changedColumns.map((name) => [name, values[name]] as [string, string | null]);
    await runAsyncAction(
      () =>
        invoke("update_table_row", {
          database,
          table,
          pkColumn: pkColumn[0],
          pkValue,
          changes,
        }),
      {
        onStart: () => {
          setSaving(true);
          setSaveError(null);
        },
        onSuccess: () => {
          setLastSaved(original);
          setOriginal(values);
          setSaveOk(true);
          onSaved?.();
        },
        onError: setSaveError,
        onFinally: () => setSaving(false),
      },
    );
  }

  async function rollback() {
    if (!lastSaved || pkColumn.length !== 1) return;
    const changes = Object.keys(lastSaved)
      .filter((k) => lastSaved[k] !== original[k])
      .map((name) => [name, lastSaved[name]] as [string, string | null]);
    if (changes.length === 0) return;
    await runAsyncAction(
      () =>
        invoke("update_table_row", {
          database,
          table,
          pkColumn: pkColumn[0],
          pkValue,
          changes,
        }),
      {
        onStart: () => setSaving(true),
        onSuccess: () => {
          setValues(lastSaved);
          setOriginal(lastSaved);
          setLastSaved(null);
          setSaveOk(false);
        },
        onError: setSaveError,
        onFinally: () => setSaving(false),
      },
    );
  }

  return (
    <div className="fixed inset-0 z-40 flex items-center justify-center bg-black/50">
      <div className="flex max-h-[85vh] w-[42rem] flex-col gap-3 rounded-lg border border-border bg-card p-4">
        <div className="flex items-center justify-between">
          <p className="text-sm font-medium">
            {database}.{table} — {pkValue}
          </p>
          <Button variant="ghost" size="sm" onClick={onClose}>
            Schließen
          </Button>
        </div>

        {loadError && (
          <p className="flex items-start gap-2 text-sm text-destructive">
            <AlertTriangle className="mt-0.5 size-4 shrink-0" />
            <span>{loadError}</span>
          </p>
        )}
        {!columns && !loadError && <p className="text-sm text-muted-foreground">Lade…</p>}

        {columns && (
          <>
            <p className="text-xs text-muted-foreground">
              Generischer Editor - jede Spalte roh, ohne bekannte Spaltensemantik. Text-Spalten mit
              binärer Kollation (z.B. <code>locale_name</code>) werden Windows-1252-kodiert
              gespeichert, passend zur restigen App.
            </p>
            <div className="min-h-0 flex-1 space-y-2 overflow-y-auto pr-1">
              {columns.map((col) => (
                <label key={col.name} className="flex items-center gap-2 text-sm">
                  <span
                    className={`w-40 shrink-0 truncate font-mono text-xs ${
                      col.is_primary_key ? "font-semibold" : "text-muted-foreground"
                    }`}
                    title={col.data_type}
                  >
                    {col.name}
                    {col.is_primary_key && " (PK)"}
                  </span>
                  <input
                    type={isNumericType(col.data_type) ? "number" : "text"}
                    value={values[col.name] ?? ""}
                    disabled={col.is_primary_key}
                    onChange={(e) => setField(col.name, e.target.value)}
                    className="w-full rounded-md border border-border bg-background px-2 py-1 text-sm disabled:opacity-60"
                  />
                </label>
              ))}
            </div>

            {saveError && <p className="text-sm text-destructive">{saveError}</p>}
            {saveOk && (
              <p className="flex items-center gap-1 text-sm text-green-600">
                <CheckCircle2 className="size-4" /> Gespeichert.
              </p>
            )}

            <div className="flex items-center justify-between">
              <span className="text-xs text-muted-foreground">
                {changedColumns.length > 0
                  ? `${changedColumns.length} Feld(er) geändert`
                  : "Keine Änderungen"}
              </span>
              <div className="flex gap-2">
                {lastSaved && (
                  <Button variant="outline" size="sm" onClick={rollback} disabled={saving}>
                    <Undo2 className="size-3.5" />
                    Letztes Speichern rückgängig
                  </Button>
                )}
                <Button
                  size="sm"
                  onClick={() => setConfirmOpen(true)}
                  disabled={changedColumns.length === 0 || saving}
                >
                  {saving ? "Speichere…" : "Speichern"}
                </Button>
              </div>
            </div>
          </>
        )}

        {confirmOpen && (
          <div className="fixed inset-0 z-50 flex items-center justify-center bg-black/50">
            <div className="w-96 space-y-3 rounded-lg border border-border bg-card p-4">
              <p className="flex items-start gap-2 text-sm">
                <AlertTriangle className="mt-0.5 size-4 shrink-0 text-destructive" />
                <span>
                  {changedColumns.length} Spalte(n) in <code>{database}.{table}</code> werden direkt
                  in der Live-Datenbank überschrieben, ohne Backup (DB-Zeilen, keine Datei).
                </span>
              </p>
              <div className="flex justify-end gap-2">
                <Button variant="outline" onClick={() => setConfirmOpen(false)}>
                  Abbrechen
                </Button>
                <Button onClick={save}>Speichern</Button>
              </div>
            </div>
          </div>
        )}
      </div>
    </div>
  );
}
