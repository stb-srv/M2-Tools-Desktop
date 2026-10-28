import { useEffect, useState } from "react";
import { invoke } from "@tauri-apps/api/core";
import { Button } from "@/components/ui/button";
import { Webhook } from "lucide-react";
import {
  ConnStatusIcon,
  SettingRow,
  SettingsCard,
  saveSetting,
  saveSettingWithFeedback,
  type TestState,
} from "./shared";

export function NotificationsTab() {
  const [webhookUrl, setWebhookUrl] = useState("");
  const [stored, setStored] = useState("");
  const [webhookTestState, setWebhookTestState] = useState<TestState>("idle");
  const [webhookTestError, setWebhookTestError] = useState<string | null>(null);

  useEffect(() => {
    invoke<string | null>("get_setting", { key: "webhook_url" })
      .then((v) => {
        setWebhookUrl(v ?? "");
        setStored(v ?? "");
      })
      .catch(() => {});
  }, []);

  async function commitWebhookUrl() {
    const value = webhookUrl.trim();
    if (value === stored) return;
    if (await saveSettingWithFeedback("webhook_url", value, "Webhook-URL")) setStored(value);
  }

  async function testWebhook() {
    setWebhookTestState("testing");
    setWebhookTestError(null);
    try {
      await saveSetting("webhook_url", webhookUrl.trim());
      setStored(webhookUrl.trim());
      await invoke("send_test_webhook");
      setWebhookTestState("ok");
    } catch (e) {
      setWebhookTestState("error");
      setWebhookTestError(String(e));
    }
  }

  return (
    <div className="space-y-4">
      <SettingsCard title="Webhook (Discord o.ä.)" icon={Webhook}>
        <SettingRow
          label="Webhook-URL"
          hint="Meldet fehlgeschlagene Datenbank-Backups und (solange die App offen ist) wenn die überwachten Server-Prozesse verschwinden. Leer lassen schaltet Benachrichtigungen ab."
        >
          <input
            value={webhookUrl}
            onChange={(e) => setWebhookUrl(e.target.value)}
            onBlur={commitWebhookUrl}
            placeholder="https://discord.com/api/webhooks/…"
            className="w-full max-w-2xl rounded-md border border-border bg-background px-3 py-2 text-sm"
          />
          <div className="flex items-center gap-2 pt-1">
            <Button
              variant="outline"
              size="sm"
              onClick={testWebhook}
              disabled={!webhookUrl.trim() || webhookTestState === "testing"}
            >
              {webhookTestState === "testing" ? "Sende…" : "Testnachricht senden"}
            </Button>
            <ConnStatusIcon state={webhookTestState} />
          </div>
          {webhookTestError && <p className="text-sm text-destructive">{webhookTestError}</p>}
        </SettingRow>
      </SettingsCard>
    </div>
  );
}
