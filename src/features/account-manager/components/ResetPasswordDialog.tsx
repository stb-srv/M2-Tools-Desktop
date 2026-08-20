import { useState } from "react";
import { invoke } from "@tauri-apps/api/core";
import { runAsyncAction } from "@/lib/asyncAction";
import { Button } from "@/components/ui/button";
import { CheckCircle2 } from "lucide-react";
import type { AccountSummary } from "../shared";

// Passwörter sind ein Einweg-Hash (MySQL PASSWORD()) - live gegen echte
// Accounts verifiziert (41-Zeichen "*..."-Hash), nicht umkehrbar. "Auslesen"
// ist daher technisch unmöglich; stattdessen wird ein neues Passwort
// gesetzt, genau wie es der echte Login-Check erwartet.
export function ResetPasswordDialog({ account, onClose }: { account: AccountSummary; onClose: () => void }) {
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
