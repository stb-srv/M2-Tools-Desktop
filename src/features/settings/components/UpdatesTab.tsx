import { useEffect, useState } from "react";
import { getVersion } from "@tauri-apps/api/app";
import { check, type Update } from "@tauri-apps/plugin-updater";
import { relaunch } from "@tauri-apps/plugin-process";
import { Button } from "@/components/ui/button";
import { useUpdateStore } from "@/store/updateStore";

type UpdateStatus = "idle" | "checking" | "upToDate" | "error" | "available";

// Nutzt die Plugin-API direkt (check()/Update.downloadAndInstall()) statt
// eigener Tauri-Commands - der Authorization-Header fürs private Repo wird
// bereits einmalig beim Plugin-Setup gesetzt (siehe build_updater_plugin()
// in lib.rs), gilt automatisch für jede Anfrage dieses Plugins.
export function UpdatesTab() {
  const [currentVersion, setCurrentVersion] = useState("");
  const [status, setStatus] = useState<UpdateStatus>("idle");
  const [error, setError] = useState<string | null>(null);
  const [update, setUpdate] = useState<Update | null>(null);
  const [downloading, setDownloading] = useState(false);
  const [downloadedBytes, setDownloadedBytes] = useState(0);
  const [totalBytes, setTotalBytes] = useState<number | null>(null);
  const [installed, setInstalled] = useState(false);
  const [restarting, setRestarting] = useState(false);

  useEffect(() => {
    getVersion()
      .then(setCurrentVersion)
      .catch(() => {});
  }, []);

  async function runCheck() {
    setStatus("checking");
    setError(null);
    setInstalled(false);
    try {
      const result = await check();
      setUpdate(result);
      if (result) {
        setStatus("available");
        useUpdateStore.getState().setAvailable({ version: result.version, notes: result.body ?? null });
      } else {
        setStatus("upToDate");
        useUpdateStore.getState().setAvailable(null);
      }
    } catch (e) {
      setStatus("error");
      setError(String(e));
    }
  }

  async function installNow() {
    if (!update) return;
    setDownloading(true);
    setDownloadedBytes(0);
    setTotalBytes(null);
    setError(null);
    try {
      await update.downloadAndInstall((event) => {
        if (event.event === "Started") setTotalBytes(event.data.contentLength ?? null);
        else if (event.event === "Progress") setDownloadedBytes((b) => b + event.data.chunkLength);
        else if (event.event === "Finished") setInstalled(true);
      });
      setInstalled(true);
    } catch (e) {
      setError(String(e));
    } finally {
      setDownloading(false);
    }
  }

  async function restart() {
    setRestarting(true);
    await relaunch();
  }

  return (
    <div className="space-y-4">
      <h2 className="text-base font-semibold">Updates</h2>
      <section className="space-y-3 rounded-lg border border-border bg-card p-4">
        <p className="text-sm text-muted-foreground">
          Aktuelle Version: <span className="font-medium text-foreground">{currentVersion || "…"}</span>
        </p>
        <Button variant="outline" onClick={runCheck} disabled={status === "checking" || downloading}>
          {status === "checking" ? "Suche…" : "Nach Updates suchen"}
        </Button>

        {status === "upToDate" && <p className="text-sm text-green-600">Du hast bereits die neueste Version.</p>}
        {status === "error" && <p className="text-sm text-destructive">{error}</p>}

        {status === "available" && update && !installed && (
          <div className="space-y-2 rounded-md border border-blue-500/40 bg-blue-500/10 p-3">
            <p className="text-sm font-medium">Update verfügbar: v{update.version}</p>
            {update.body && <p className="whitespace-pre-wrap text-xs text-muted-foreground">{update.body}</p>}
            {!downloading ? (
              <Button onClick={installNow}>Jetzt installieren</Button>
            ) : (
              <p className="text-xs text-muted-foreground">
                Lade herunter…{" "}
                {totalBytes ? `${Math.round((downloadedBytes / totalBytes) * 100)}%` : `${downloadedBytes} Bytes`}
              </p>
            )}
          </div>
        )}

        {installed && (
          <div className="space-y-2 rounded-md border border-green-500/40 bg-green-500/10 p-3">
            <p className="text-sm">Installiert. Neustart nötig, um die neue Version zu verwenden.</p>
            <Button onClick={restart} disabled={restarting}>
              {restarting ? "Starte neu…" : "Jetzt neu starten"}
            </Button>
          </div>
        )}
      </section>
    </div>
  );
}
