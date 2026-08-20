import { useEffect, useState } from "react";
import { invoke } from "@tauri-apps/api/core";
import { Button } from "@/components/ui/button";
import { ConnStatusIcon, saveSetting, type TestState } from "./shared";

export function NotificationsTab({ onSaved }: { onSaved: (label: string) => void }) {
  const [webhookUrl, setWebhookUrl] = useState("");
  const [webhookTestState, setWebhookTestState] = useState<TestState>("idle");
  const [webhookTestError, setWebhookTestError] = useState<string | null>(null);

  useEffect(() => {
    invoke<string | null>("get_setting", { key: "webhook_url" })
      .then((v) => setWebhookUrl(v ?? ""))
      .catch(() => {});
  }, []);

  async function commitWebhookUrl() {
    await saveSetting("webhook_url", webhookUrl.trim());
    onSaved("Webhook-URL gespeichert");
  }

  async function testWebhook() {
    setWebhookTestState("testing");
    setWebhookTestError(null);
    try {
      await commitWebhookUrl();
      await invoke("send_test_webhook");
      setWebhookTestState("ok");
    } catch (e) {
      setWebhookTestState("error");
      setWebhookTestError(String(e));
    }
  }

  return (
    <div className="space-y-4">
      <h2 className="text-base font-semibold">Benachrichtigungen</h2>
      <section className="space-y-1">
        <label className="flex items-center gap-2 text-sm">
          Webhook-URL (Discord o.ä.)
          <input
            value={webhookUrl}
            onChange={(e) => setWebhookUrl(e.target.value)}
            onBlur={commitWebhookUrl}
            placeholder="https://discord.com/api/webhooks/…"
            className="w-96 rounded-md border border-border bg-background px-2 py-1 text-sm"
          />
        </label>
        <p className="text-xs text-muted-foreground">
          Wird bei fehlgeschlagenen Datenbank-Backups und (solange die App geöffnet ist) beim
          Verschwinden der überwachten Server-Prozesse benachrichtigt. Leer lassen deaktiviert
          Benachrichtigungen.
        </p>
        <div className="flex items-center gap-2 pt-1">
          <Button
            variant="outline"
            onClick={testWebhook}
            disabled={!webhookUrl.trim() || webhookTestState === "testing"}
          >
            {webhookTestState === "testing" ? "Sende…" : "Testnachricht senden"}
          </Button>
          <ConnStatusIcon state={webhookTestState} />
        </div>
        {webhookTestError && <p className="text-sm text-destructive">{webhookTestError}</p>}
      </section>
    </div>
  );
}
