import { useEffect, useRef } from "react";
import { invoke } from "@tauri-apps/api/core";

const POLL_INTERVAL_MS = 60_000;

interface ProcessUsage {
  cpu_percent: number;
}

interface MemoryInfo {
  total_bytes: number;
  free_bytes: number;
}

interface ServerOverview {
  memory: MemoryInfo | null;
  disk: { capacity_percent: number } | null;
}

/**
 * Best-effort crash alerting: while this desktop app is open, polls the
 * configured server processes (same command as the Dashboard's resource
 * monitoring) and fires a webhook the moment they go from "found" to "not
 * found". Explicitly NOT a real always-on monitor - it stops the instant
 * the app is closed, unlike a proper server-side watchdog. A single failed
 * poll (e.g. a flaky SSH connection) is treated as "can't tell right now",
 * not as a crash, so a momentary network hiccup doesn't fire a false alarm.
 *
 * 2026-08-25: this is also the app's only periodic SSH poll, so it now
 * doubles as the source for the Dashboard's resource-history chart
 * (`log_resource_snapshot`, see resource_history.rs) - logging happens
 * regardless of whether a webhook is configured (previously the entire poll
 * was skipped without one, so history would silently never accumulate for
 * anyone who hasn't set up Discord notifications). The webhook check now
 * only gates the *notification*, not the underlying fetch.
 */
export function CrashWatch() {
  const wasRunning = useRef<boolean | null>(null);

  useEffect(() => {
    const interval = setInterval(async () => {
      let processes: ProcessUsage[] | null = null;
      try {
        processes = await invoke<ProcessUsage[]>("get_server_resource_usage");
      } catch {
        // SSH nicht erreichbar o.ä. - als "unbekannt" behandeln, nicht als Absturz.
      }

      if (processes) {
        const running = processes.length > 0;
        try {
          const webhookUrl = await invoke<string | null>("get_setting", { key: "webhook_url" });
          if (webhookUrl && wasRunning.current === true && !running) {
            await invoke("notify_webhook_message", {
              message:
                "M2Manager: Die überwachten Server-Prozesse sind verschwunden (evtl. Absturz).",
            }).catch(() => {});
          }
        } catch {
          // Webhook-Einstellung nicht lesbar - Benachrichtigung überspringen, kein harter Fehler.
        }
        wasRunning.current = running;

        const cpuPercent = processes.reduce((sum, p) => sum + p.cpu_percent, 0);
        try {
          const overview = await invoke<ServerOverview>("get_server_overview");
          await invoke("log_resource_snapshot", {
            cpuPercent,
            ramUsedBytes: overview.memory ? overview.memory.total_bytes - overview.memory.free_bytes : null,
            ramTotalBytes: overview.memory ? overview.memory.total_bytes : null,
            diskCapacityPercent: overview.disk ? overview.disk.capacity_percent : null,
          }).catch(() => {});
        } catch {
          // Übersicht nicht verfügbar - Snapshot einfach auslassen statt hart zu scheitern.
        }
      }
    }, POLL_INTERVAL_MS);
    return () => clearInterval(interval);
  }, []);

  return null;
}
