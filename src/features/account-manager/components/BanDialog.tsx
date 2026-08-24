import { useState } from "react";
import { invoke } from "@tauri-apps/api/core";
import { runAsyncAction } from "@/lib/asyncAction";
import { logActivity } from "@/lib/logActivity";
import { Button } from "@/components/ui/button";
import { AlertTriangle } from "lucide-react";
import type { AccountSummary } from "../shared";

// Sperrt einen Account mit einer frei formulierten Nachricht - die landet
// wörtlich als Fehlermeldung am Login-Screen des Spielers (verifiziert:
// der Login-Server vergleicht `status` nur auf "OK", jeder andere Wert wird
// unverändert an den Client geschickt). Die Dauer ist eine reine
// M2Manager-Zeitsteuerung (siehe bans.rs) - kein Server-Cron, wirkt nur
// wenn die App läuft.
export function BanDialog({
  account,
  onClose,
  onBanned,
}: {
  account: AccountSummary;
  onClose: () => void;
  onBanned: () => void;
}) {
  const [message, setMessage] = useState("Gesperrt - bitte kontaktiere den Support.");
  const [days, setDays] = useState("");
  const [busy, setBusy] = useState(false);
  const [error, setError] = useState<string | null>(null);

  async function submit() {
    await runAsyncAction(
      () =>
        invoke("ban_account", {
          accountId: account.id,
          login: account.login,
          message: message.trim(),
          days: days.trim() ? Number(days) : null,
        }),
      {
        onStart: () => {
          setBusy(true);
          setError(null);
        },
        onSuccess: () => {
          const duration = days.trim() ? `${days.trim()} Tag(e)` : "dauerhaft";
          logActivity(
            "account-manager",
            "ban",
            `Account '${account.login}' gesperrt (${duration}): ${message.trim()}`,
            "account",
            String(account.id),
          );
          onBanned();
        },
        onError: setError,
        onFinally: () => setBusy(false),
      },
    );
  }

  return (
    <div className="fixed inset-0 z-50 flex items-center justify-center bg-black/50">
      <div className="w-96 space-y-3 rounded-lg border border-border bg-card p-4">
        <p className="text-sm font-medium">Account sperren: {account.login}</p>
        <label className="flex flex-col gap-1 text-xs text-muted-foreground">
          Nachricht (wird wörtlich am Login-Bildschirm angezeigt)
          <textarea
            autoFocus
            value={message}
            onChange={(e) => setMessage(e.target.value)}
            rows={3}
            className="rounded-md border border-border bg-background px-2 py-1 text-sm"
          />
        </label>
        <label className="flex flex-col gap-1 text-xs text-muted-foreground">
          Dauer in Tagen (leer = dauerhaft, keine automatische Entsperrung)
          <input
            type="number"
            min={1}
            value={days}
            onChange={(e) => setDays(e.target.value)}
            placeholder="dauerhaft"
            className="w-32 rounded-md border border-border bg-background px-2 py-1 text-sm"
          />
        </label>
        <p className="flex items-start gap-2 text-xs text-muted-foreground">
          <AlertTriangle className="mt-0.5 size-3.5 shrink-0" />
          Automatische Entsperrung greift nur, solange/wann immer M2Manager läuft - es gibt keinen
          Server-seitigen Cron dafür.
        </p>
        {error && <p className="text-sm text-destructive">{error}</p>}
        <div className="flex justify-end gap-2">
          <Button variant="outline" onClick={onClose}>
            Abbrechen
          </Button>
          <Button variant="destructive" disabled={!message.trim() || busy} onClick={submit}>
            {busy ? "Sperre…" : "Sperren"}
          </Button>
        </div>
      </div>
    </div>
  );
}
