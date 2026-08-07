import { useEffect, useRef, useState } from "react";
import { invoke } from "@tauri-apps/api/core";
import { listen } from "@tauri-apps/api/event";
import { Button } from "@/components/ui/button";
import {
  Wrench,
  RefreshCw,
  Hammer,
  UploadCloud,
  RotateCcw,
  Settings2,
  AlertTriangle,
  CheckCircle2,
  XCircle,
  Eraser,
} from "lucide-react";

interface DeployRecord {
  id: number;
  kind: "deploy" | "rollback";
  targets: string[];
  created_at: string;
  game_backup_path: string | null;
  db_backup_path: string | null;
  note: string | null;
  success: boolean | null;
  rolled_back_from: number | null;
}

interface BuildTargets {
  libs: string[];
  bins: string[];
}

interface ServerCommandResult {
  output: string;
  exit_status: number | null;
}

const DEPLOY_PHRASE = "SERVER ERSETZEN";
const ROLLBACK_PHRASE = "ROLLBACK";

const SETTING_DEFAULTS: Record<string, string> = {
  build_live_source_root: "/usr/home/source/server",
  build_scratch_source_root: "/usr/home/m2manager_build/server",
  build_live_game_binary: "/usr/home/source/server/game/game",
  build_live_db_binary: "/usr/home/source/server/db/db",
  deploy_liveness_wait_seconds: "8",
  deploy_liveness_recheck_seconds: "5",
};

function Field({ label, children }: { label: string; children: React.ReactNode }) {
  return (
    <label className="flex flex-col gap-1 text-xs text-muted-foreground">
      {label}
      {children}
    </label>
  );
}

function formatDate(iso: string) {
  try {
    return new Date(iso).toLocaleString();
  } catch {
    return iso;
  }
}

export function BuildDeploy() {
  const [settings, setSettings] = useState<Record<string, string>>(SETTING_DEFAULTS);
  const [editingSettings, setEditingSettings] = useState(false);

  const [targets, setTargets] = useState<BuildTargets | null>(null);
  const [selected, setSelected] = useState<Set<string>>(new Set());
  const [cleanBuild, setCleanBuild] = useState(true);

  const [log, setLog] = useState<string[]>([]);
  const [logOpen, setLogOpen] = useState(false);
  const logRef = useRef<HTMLPreElement>(null);

  const [syncing, setSyncing] = useState(false);
  const [building, setBuilding] = useState(false);
  const [deploying, setDeploying] = useState(false);
  const [rollingBack, setRollingBack] = useState(false);

  const [buildOk, setBuildOk] = useState<boolean | null>(null);
  const [note, setNote] = useState("");
  const [error, setError] = useState<string | null>(null);

  const [deployConfirming, setDeployConfirming] = useState(false);
  const [deployConfirmText, setDeployConfirmText] = useState("");
  const [rollbackConfirming, setRollbackConfirming] = useState<DeployRecord | "latest" | null>(null);
  const [rollbackConfirmText, setRollbackConfirmText] = useState("");

  const [history, setHistory] = useState<DeployRecord[]>([]);
  const [lastResult, setLastResult] = useState<DeployRecord | null>(null);

  useEffect(() => {
    (async () => {
      const loaded: Record<string, string> = { ...SETTING_DEFAULTS };
      for (const key of Object.keys(SETTING_DEFAULTS)) {
        const saved = await invoke<string | null>("get_setting", { key }).catch(() => null);
        if (saved) loaded[key] = saved;
      }
      setSettings(loaded);

      const t = await invoke<BuildTargets>("list_build_targets").catch(() => null);
      if (t) {
        setTargets(t);
        setSelected(new Set([...t.libs, ...t.bins]));
      }

      await refreshHistory();
    })();
  }, []);

  useEffect(() => {
    const unlisten = listen<string>("build-deploy-output", (event) => {
      setLog((prev) => [...prev, event.payload]);
    });
    return () => {
      unlisten.then((fn) => fn());
    };
  }, []);

  useEffect(() => {
    logRef.current?.scrollTo({ top: logRef.current.scrollHeight });
  }, [log]);

  async function refreshHistory() {
    const rows = await invoke<DeployRecord[]>("list_deploy_history").catch(() => []);
    setHistory(rows);
  }

  async function saveSetting(key: string, value: string) {
    setSettings((prev) => ({ ...prev, [key]: value }));
    await invoke("set_setting", { key, value }).catch(() => {});
  }

  function toggleTarget(name: string) {
    setSelected((prev) => {
      const next = new Set(prev);
      if (next.has(name)) next.delete(name);
      else next.add(name);
      return next;
    });
  }

  async function runSync() {
    setSyncing(true);
    setError(null);
    setLogOpen(true);
    setLog((prev) => [...prev, "\n[Synchronisiere Quellcode-Kopie…]\n"]);
    try {
      const result = await invoke<ServerCommandResult>("sync_build_source");
      if (result.exit_status !== null && result.exit_status !== 0) {
        setLog((prev) => [...prev, `\n[Beendet mit Code ${result.exit_status}]\n`]);
      } else {
        setLog((prev) => [...prev, "\n[Synchronisierung abgeschlossen.]\n"]);
      }
    } catch (e) {
      setError(String(e));
      setLog((prev) => [...prev, `\nFehler: ${String(e)}\n`]);
    } finally {
      setSyncing(false);
    }
  }

  async function runBuild() {
    if (!targets) return;
    setBuilding(true);
    setBuildOk(null);
    setError(null);
    setLogOpen(true);
    setLog((prev) => [...prev, "\n[Baue Server-Quellcode (Arbeitskopie — Live-Server bleibt unberührt)…]\n"]);
    try {
      // Bibliotheken immer vor den Programmdateien, die sie brauchen -
      // Reihenfolge unabhängig von der Klick-Reihenfolge des Nutzers.
      const orderedTargets = [...targets.libs, ...targets.bins].filter((t) => selected.has(t));
      const result = await invoke<ServerCommandResult>("run_source_build", {
        options: {
          source_root: settings.build_scratch_source_root,
          targets: orderedTargets,
          clean: cleanBuild,
        },
      });
      const ok = result.exit_status === null || result.exit_status === 0;
      setBuildOk(ok);
      setLog((prev) => [
        ...prev,
        ok
          ? "\n[Bauen erfolgreich.]\n"
          : `\n[Bauen fehlgeschlagen, Code ${result.exit_status}.]\n`,
      ]);
    } catch (e) {
      setBuildOk(false);
      setError(String(e));
      setLog((prev) => [...prev, `\nFehler: ${String(e)}\n`]);
    } finally {
      setBuilding(false);
    }
  }

  function deployTargets(): string[] {
    if (!targets) return [];
    return targets.bins.filter((t) => selected.has(t));
  }

  async function runDeploy() {
    setDeployConfirming(false);
    setDeployConfirmText("");
    setDeploying(true);
    setError(null);
    setLogOpen(true);
    try {
      const record = await invoke<DeployRecord>("run_deploy", { targets: deployTargets(), note });
      setLastResult(record);
      await refreshHistory();
    } catch (e) {
      setError(String(e));
      setLog((prev) => [...prev, `\nFehler: ${String(e)}\n`]);
    } finally {
      setDeploying(false);
    }
  }

  async function runRollback() {
    const target = rollbackConfirming;
    setRollbackConfirming(null);
    setRollbackConfirmText("");
    setRollingBack(true);
    setError(null);
    setLogOpen(true);
    try {
      const deployId = target === "latest" || target === null ? undefined : target.id;
      const record = await invoke<DeployRecord>("run_rollback", { deployId });
      setLastResult(record);
      await refreshHistory();
    } catch (e) {
      setError(String(e));
      setLog((prev) => [...prev, `\nFehler: ${String(e)}\n`]);
    } finally {
      setRollingBack(false);
    }
  }

  const busy = syncing || building || deploying || rollingBack;
  const rollbackTargetLabel =
    rollbackConfirming === "latest"
      ? "letzter Einspiel-Vorgang"
      : rollbackConfirming
        ? `Eintrag #${rollbackConfirming.id} (${formatDate(rollbackConfirming.created_at)})`
        : "";

  return (
    <div className="max-w-4xl space-y-6 pb-10">
      <div className="flex items-center justify-between">
        <h1 className="flex items-center gap-2 text-2xl font-semibold">
          <Wrench className="size-6" /> Server-Quellcode Bauen & Einspielen
        </h1>
        <Button variant="ghost" onClick={() => setEditingSettings((v) => !v)}>
          <Settings2 className="size-4" />
          Einstellungen {editingSettings ? "ausblenden" : "anzeigen"}
        </Button>
      </div>

      <div className="flex items-start gap-2 rounded-lg border border-amber-500/40 bg-amber-500/10 p-3 text-sm text-amber-700 dark:text-amber-400">
        <AlertTriangle className="mt-0.5 size-4 shrink-0" />
        <span>
          Diese Aktion betrifft <strong>ALLE Channels und den Login-Server gleichzeitig</strong> — sie teilen
          sich per Symlink eine einzige Programmdatei (live geprüft). Es gibt keinen Testserver und keine
          Möglichkeit, einen einzelnen Channel isoliert zu testen. Bauen (unten) ist gefahrlos — es läuft
          ausschließlich in einer separaten Arbeitskopie und rührt den Live-Server nicht an. Nur der
          "Jetzt live einspielen"-Schritt ersetzt echte Programmdateien.
        </span>
      </div>

      {editingSettings && (
        <div className="grid grid-cols-1 gap-3 rounded-lg border border-border bg-card p-3 sm:grid-cols-2">
          {Object.keys(SETTING_DEFAULTS).map((key) => (
            <Field key={key} label={key}>
              <input
                value={settings[key] ?? ""}
                onChange={(e) => saveSetting(key, e.target.value)}
                className="w-full rounded-md border border-border bg-background px-2 py-1 font-mono text-xs"
              />
            </Field>
          ))}
        </div>
      )}

      {error && <p className="whitespace-pre-wrap text-sm text-destructive">{error}</p>}

      {/* Schritt 1: Synchronisieren */}
      <section className="space-y-2 rounded-lg border border-border p-4">
        <h2 className="text-sm font-medium text-muted-foreground">Schritt 1: Quellcode-Kopie aktualisieren</h2>
        <p className="text-xs text-muted-foreground">
          Ungefährlich — liest nur vom Live-Quellbaum ({settings.build_live_source_root}), schreibt nur in die
          separate Arbeitskopie ({settings.build_scratch_source_root}).
        </p>
        <Button variant="outline" onClick={runSync} disabled={busy}>
          <RefreshCw className={`size-4 ${syncing ? "animate-spin" : ""}`} />
          {syncing ? "Synchronisiere…" : "Quellcode-Kopie aktualisieren"}
        </Button>
      </section>

      {/* Schritt 2: Bauen */}
      <section className="space-y-3 rounded-lg border border-border p-4">
        <h2 className="text-sm font-medium text-muted-foreground">Schritt 2: Bauen (Arbeitskopie — Live-Server bleibt unberührt)</h2>
        {targets && (
          <div className="flex flex-wrap gap-3">
            {[...targets.libs, ...targets.bins].map((t) => (
              <label key={t} className="flex items-center gap-1.5 text-sm">
                <input type="checkbox" checked={selected.has(t)} onChange={() => toggleTarget(t)} />
                {t}
              </label>
            ))}
          </div>
        )}
        <label className="flex items-center gap-1.5 text-sm">
          <input type="checkbox" checked={cleanBuild} onChange={(e) => setCleanBuild(e.target.checked)} />
          Sauber neu bauen (gmake clean vor jedem Ziel)
        </label>
        {targets && (selected.size < targets.libs.length + targets.bins.length) && (
          <p className="flex items-center gap-1 text-xs text-amber-600 dark:text-amber-400">
            <AlertTriangle className="size-3.5 shrink-0" />
            Bei Teil-Builds können veraltete .a-Bibliotheken verlinkt werden — im Zweifel alles auswählen.
          </p>
        )}
        <Button onClick={runBuild} disabled={busy || selected.size === 0}>
          <Hammer className="size-4" />
          {building ? "Baue…" : "Bauen (Arbeitskopie — Live-Server bleibt unberührt)"}
        </Button>

        {buildOk === true && (
          <div className="space-y-2 rounded-md border border-green-500/40 bg-green-500/10 p-3">
            <p className="flex items-center gap-1 text-sm text-green-700 dark:text-green-400">
              <CheckCircle2 className="size-4" /> Bauen erfolgreich.
            </p>
            {deployTargets().length === 0 ? (
              <p className="text-xs text-muted-foreground">
                Keine Programmdatei ("game"/"db") ausgewählt — nur Bibliotheken gebaut, nichts zum Einspielen.
              </p>
            ) : (
              <>
                <Field label="Was wurde geändert? (wird im Verlauf gespeichert)">
                  <input
                    value={note}
                    onChange={(e) => setNote(e.target.value)}
                    className="w-full rounded-md border border-border bg-background px-2 py-1 text-sm"
                  />
                </Field>
                <Button variant="destructive" onClick={() => setDeployConfirming(true)} disabled={busy}>
                  <UploadCloud className="size-4" />
                  Jetzt live einspielen…
                </Button>
              </>
            )}
          </div>
        )}
        {buildOk === false && (
          <p className="flex items-center gap-1 text-sm text-destructive">
            <XCircle className="size-4" /> Bauen fehlgeschlagen — Log prüfen, Live-Server wurde nicht angefasst.
          </p>
        )}
      </section>

      {/* Ergebnis + Rollback */}
      {lastResult && (
        <section className="space-y-2 rounded-lg border border-border p-4">
          <h2 className="text-sm font-medium text-muted-foreground">Letztes Ergebnis</h2>
          {lastResult.success === true && (
            <p className="flex items-center gap-1 text-sm text-green-700 dark:text-green-400">
              <CheckCircle2 className="size-4" /> Live-Prüfung erfolgreich ({lastResult.kind}, #{lastResult.id}).
            </p>
          )}
          {lastResult.success === false && (
            <p className="flex items-center gap-1 text-sm text-destructive">
              <XCircle className="size-4" /> Live-Prüfung fehlgeschlagen — Server evtl. nicht sauber gestartet
              (#{lastResult.id}).
            </p>
          )}
          {lastResult.success === null && (
            <p className="text-sm text-muted-foreground">Live-Prüfung läuft noch oder unbekannt.</p>
          )}
          <Button variant="destructive" onClick={() => setRollbackConfirming(lastResult)} disabled={busy}>
            <RotateCcw className="size-4" />
            {rollingBack ? "Rolle zurück…" : "Rückgängig machen"}
          </Button>
        </section>
      )}

      {/* Verlauf */}
      <section className="space-y-2 rounded-lg border border-border p-4">
        <div className="flex items-center justify-between">
          <h2 className="text-sm font-medium text-muted-foreground">Verlauf</h2>
          <Button variant="ghost" size="sm" onClick={refreshHistory}>
            <RefreshCw className="size-3.5" /> Neu laden
          </Button>
        </div>
        {history.length === 0 && <p className="text-sm text-muted-foreground">Noch nichts eingespielt.</p>}
        <div className="space-y-1">
          {history.map((h) => (
            <div
              key={h.id}
              className="flex items-center justify-between gap-2 rounded-md border border-border p-2 text-sm"
            >
              <div className="min-w-0 flex-1">
                <div className="flex items-center gap-2">
                  <span className="font-medium">
                    #{h.id} {h.kind === "deploy" ? "Einspielen" : "Rückgängig-machen"}
                  </span>
                  <span className="text-xs text-muted-foreground">{formatDate(h.created_at)}</span>
                  <span className="text-xs text-muted-foreground">{h.targets.join(", ")}</span>
                  {h.success === true && <CheckCircle2 className="size-3.5 text-green-600" />}
                  {h.success === false && <XCircle className="size-3.5 text-destructive" />}
                </div>
                {h.note && <p className="truncate text-xs text-muted-foreground">{h.note}</p>}
              </div>
              {h.kind === "deploy" && (h.game_backup_path || h.db_backup_path) && (
                <Button variant="outline" size="sm" onClick={() => setRollbackConfirming(h)} disabled={busy}>
                  Auf diesen Stand zurücksetzen
                </Button>
              )}
            </div>
          ))}
        </div>
      </section>

      {/* Log-Modal */}
      {logOpen && (
        <div className="fixed inset-0 flex items-center justify-center bg-black/50">
          <div className="flex max-h-[70vh] w-[42rem] flex-col gap-3 rounded-lg border border-border bg-card p-4">
            <div className="flex items-center justify-between">
              <p className="text-sm font-medium">Ausgabe</p>
              <div className="flex items-center gap-2">
                <Button variant="ghost" size="sm" onClick={() => setLog([])} disabled={!log.length}>
                  <Eraser className="size-3.5" /> Leeren
                </Button>
                <Button variant="ghost" size="sm" onClick={() => setLogOpen(false)} disabled={busy}>
                  Schließen
                </Button>
              </div>
            </div>
            <pre
              ref={logRef}
              className="min-h-0 flex-1 overflow-auto whitespace-pre-wrap rounded-md border border-border bg-muted/30 p-2 font-mono text-xs"
            >
              {log.length ? log.join("") : "…"}
            </pre>
          </div>
        </div>
      )}

      {/* Einspiel-Bestätigung */}
      {deployConfirming && (
        <div className="fixed inset-0 z-10 flex items-center justify-center bg-black/50">
          <div className="w-[28rem] space-y-3 rounded-lg border border-border bg-card p-4">
            <p className="flex items-start gap-2 text-sm font-medium">
              <AlertTriangle className="mt-0.5 size-4 shrink-0 text-destructive" />
              Live-Server ersetzen
            </p>
            <p className="text-sm text-muted-foreground">
              Dies stoppt Server/Channel + Login-Server, sichert die aktuelle(n) Programmdatei(en) (
              {deployTargets().join(", ")}) und ersetzt sie live. Danach wird automatisch neu gestartet.
            </p>
            <p className="text-sm">
              Tippe zum Bestätigen exakt: <code className="rounded bg-muted px-1">{DEPLOY_PHRASE}</code>
            </p>
            <input
              autoFocus
              value={deployConfirmText}
              onChange={(e) => setDeployConfirmText(e.target.value)}
              className="w-full rounded-md border border-border bg-background px-2 py-1 text-sm"
            />
            <div className="flex justify-end gap-2">
              <Button variant="outline" onClick={() => setDeployConfirming(false)}>
                Abbrechen
              </Button>
              <Button variant="destructive" disabled={deployConfirmText !== DEPLOY_PHRASE} onClick={runDeploy}>
                Einspielen
              </Button>
            </div>
          </div>
        </div>
      )}

      {/* Rückgängig-machen-Bestätigung */}
      {rollbackConfirming && (
        <div className="fixed inset-0 z-10 flex items-center justify-center bg-black/50">
          <div className="w-[28rem] space-y-3 rounded-lg border border-border bg-card p-4">
            <p className="flex items-start gap-2 text-sm font-medium">
              <AlertTriangle className="mt-0.5 size-4 shrink-0 text-destructive" />
              Rückgängig machen
            </p>
            <p className="text-sm text-muted-foreground">
              Stellt die gesicherte(n) Programmdatei(en) von {rollbackTargetLabel} wieder her und startet den
              Server neu.
            </p>
            <p className="text-sm">
              Tippe zum Bestätigen exakt: <code className="rounded bg-muted px-1">{ROLLBACK_PHRASE}</code>
            </p>
            <input
              autoFocus
              value={rollbackConfirmText}
              onChange={(e) => setRollbackConfirmText(e.target.value)}
              className="w-full rounded-md border border-border bg-background px-2 py-1 text-sm"
            />
            <div className="flex justify-end gap-2">
              <Button variant="outline" onClick={() => setRollbackConfirming(null)}>
                Abbrechen
              </Button>
              <Button
                variant="destructive"
                disabled={rollbackConfirmText !== ROLLBACK_PHRASE}
                onClick={runRollback}
              >
                Zurückrollen
              </Button>
            </div>
          </div>
        </div>
      )}
    </div>
  );
}
