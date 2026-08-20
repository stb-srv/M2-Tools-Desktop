import { useState } from "react";
import { invoke } from "@tauri-apps/api/core";
import { runAsyncAction } from "@/lib/asyncAction";
import { Button } from "@/components/ui/button";
import { CheckCircle2 } from "lucide-react";
import type { AccountSummary, ColumnInfo } from "../shared";

// Additive Guthaben-Anpassung einer numerischen account.account-Spalte, die
// zur Laufzeit real gefunden wurde (siehe AccountSection - keine geratenen
// Spaltennamen wie "coins"/"cash", das Backend validiert zusätzlich erneut
// gegen die echte Spaltenliste).
export function CurrencyDialog({
  account,
  columns,
  onClose,
}: {
  account: AccountSummary;
  columns: ColumnInfo[];
  onClose: () => void;
}) {
  const [column, setColumn] = useState(columns[0]?.name ?? "");
  const [delta, setDelta] = useState("");
  const [busy, setBusy] = useState(false);
  const [error, setError] = useState<string | null>(null);
  const [newValue, setNewValue] = useState<number | null>(null);

  async function submit() {
    if (!column || !delta.trim()) return;
    await runAsyncAction(
      () => invoke<number>("adjust_account_numeric_column", { accountId: account.id, column, delta: Number(delta) }),
      {
        onStart: () => {
          setBusy(true);
          setError(null);
          setNewValue(null);
        },
        onSuccess: setNewValue,
        onError: setError,
        onFinally: () => setBusy(false),
      },
    );
  }

  return (
    <div className="fixed inset-0 z-50 flex items-center justify-center bg-black/50">
      <div className="w-80 space-y-3 rounded-lg border border-border bg-card p-4">
        <p className="text-sm font-medium">Guthaben anpassen: {account.login}</p>
        <label className="flex flex-col gap-1 text-xs text-muted-foreground">
          Spalte
          <select
            value={column}
            onChange={(e) => setColumn(e.target.value)}
            className="rounded-md border border-border bg-background px-2 py-1 text-sm"
          >
            {columns.map((c) => (
              <option key={c.name} value={c.name}>
                {c.name}
              </option>
            ))}
          </select>
        </label>
        <label className="flex flex-col gap-1 text-xs text-muted-foreground">
          Änderung (negativ zum Abziehen)
          <input
            type="number"
            autoFocus
            value={delta}
            onChange={(e) => setDelta(e.target.value)}
            className="rounded-md border border-border bg-background px-2 py-1 text-sm"
          />
        </label>
        {error && <p className="text-sm text-destructive">{error}</p>}
        {newValue !== null && (
          <p className="flex items-center gap-1 text-sm text-green-600">
            <CheckCircle2 className="size-4" /> Neuer Wert: {newValue}
          </p>
        )}
        <div className="flex justify-end gap-2">
          <Button variant="outline" onClick={onClose}>
            {newValue !== null ? "Schließen" : "Abbrechen"}
          </Button>
          {newValue === null && (
            <Button disabled={!column || !delta.trim() || busy} onClick={submit}>
              {busy ? "Speichere…" : "Anwenden"}
            </Button>
          )}
        </div>
      </div>
    </div>
  );
}
