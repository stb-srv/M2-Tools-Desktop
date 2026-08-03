import { useEffect, useRef } from "react";
import { invoke } from "@tauri-apps/api/core";

const POLL_INTERVAL_MS = 60_000;

/**
 * Best-effort crash alerting: while this desktop app is open, polls the
 * configured server processes (same command as the Dashboard's resource
 * monitoring) and fires a webhook the moment they go from "found" to "not
 * found". Explicitly NOT a real always-on monitor - it stops the instant
 * the app is closed, unlike a proper server-side watchdog. A single failed
 * poll (e.g. a flaky SSH connection) is treated as "can't tell right now",
 * not as a crash, so a momentary network hiccup doesn't fire a false alarm.
 */
export function CrashWatch() {
  const wasRunning = useRef<boolean | null>(null);

  useEffect(() => {
    const interval = setInterval(async () => {
      try {
        const webhookUrl = await invoke<string | null>("get_setting", { key: "webhook_url" });
        if (!webhookUrl) return;

        const processes = await invoke<unknown[]>("get_server_resource_usage");
        const running = processes.length > 0;

        if (wasRunning.current === true && !running) {
          await invoke("notify_webhook_message", {
            message:
              "M2Manager: Die überwachten Server-Prozesse sind verschwunden (evtl. Absturz).",
          }).catch(() => {});
        }
        wasRunning.current = running;
      } catch {
        // SSH nicht erreichbar o.ä. - als "unbekannt" behandeln, nicht als Absturz.
      }
    }, POLL_INTERVAL_MS);
    return () => clearInterval(interval);
  }, []);

  return null;
}
