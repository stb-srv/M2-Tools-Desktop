import { useEffect, useRef, useState } from "react";
import { useTranslation } from "react-i18next";
import { invoke } from "@tauri-apps/api/core";
import { listen } from "@tauri-apps/api/event";
import { Button } from "@/components/ui/button";
import { Play, Square, Trash2, RefreshCw, RotateCcw, Settings2, Eraser, HelpCircle } from "lucide-react";
import { openManual } from "@/lib/manual";

type ActionId = "start" | "stop" | "clearLogs" | "reloadQuests";

interface ServerAction {
  id: ActionId;
  labelKey: string;
  icon: typeof Play;
  variant: "default" | "destructive" | "outline";
  /** Destructive actions ask before running - stopping a live server or
   *  wiping logs can't be undone. */
  confirm?: string;
}

const ACTIONS: ServerAction[] = [
  { id: "start", labelKey: "serverControl.startServer", icon: Play, variant: "default" },
  {
    id: "stop",
    labelKey: "serverControl.stopServer",
    icon: Square,
    variant: "destructive",
    confirm: "Der Server wird heruntergefahren und alle Spieler werden getrennt.",
  },
  {
    id: "clearLogs",
    labelKey: "serverControl.clearLogs",
    icon: Trash2,
    variant: "destructive",
    confirm: "Alle Logdateien werden unwiderruflich gelöscht.",
  },
  {
    id: "reloadQuests",
    labelKey: "serverControl.reloadQuests",
    icon: RefreshCw,
    variant: "outline",
  },
];

const SETTING_KEY: Record<ActionId, string> = {
  start: "server_cmd_start",
  stop: "server_cmd_stop",
  clearLogs: "server_cmd_clear_logs",
  reloadQuests: "server_cmd_reload_quests",
};

const DEFAULT_WORKDIR = "/usr/home/game";

// The typical Metin2 control script prints a menu and reads a number from
// stdin, so each action is that script fed its menu choice.
function defaultCommand(workdir: string, choice: number) {
  return `cd ${workdir} && echo ${choice} | sh index.sh`;
}

const DEFAULT_CHOICE: Record<ActionId, number> = {
  start: 1,
  stop: 2,
  clearLogs: 3,
  reloadQuests: 4,
};

export function ServerControl() {
  const { t } = useTranslation();

  const [commands, setCommands] = useState<Record<ActionId, string>>({
    start: "",
    stop: "",
    clearLogs: "",
    reloadQuests: "",
  });
  const [workdir, setWorkdir] = useState(DEFAULT_WORKDIR);
  const [editing, setEditing] = useState(false);
  const [running, setRunning] = useState<ActionId | "restart" | null>(null);
  const [log, setLog] = useState<string[]>([]);
  const [error, setError] = useState<string | null>(null);
  const [confirmAction, setConfirmAction] = useState<ServerAction | null>(null);
  const [restartConfirm, setRestartConfirm] = useState(false);
  const [restartDelay, setRestartDelay] = useState("5");

  const logRef = useRef<HTMLPreElement>(null);

  useEffect(() => {
    (async () => {
      const savedWorkdir =
        (await invoke<string | null>("get_setting", { key: "server_workdir" }).catch(
          () => null,
        )) || DEFAULT_WORKDIR;
      setWorkdir(savedWorkdir);

      const loaded: Record<string, string> = {};
      for (const action of ACTIONS) {
        const saved = await invoke<string | null>("get_setting", {
          key: SETTING_KEY[action.id],
        }).catch(() => null);
        loaded[action.id] =
          saved || defaultCommand(savedWorkdir, DEFAULT_CHOICE[action.id]);
      }
      setCommands(loaded as Record<ActionId, string>);

      const savedDelay = await invoke<string | null>("get_setting", {
        key: "server_restart_delay_seconds",
      }).catch(() => null);
      if (savedDelay) setRestartDelay(savedDelay);
    })();
  }, []);

  useEffect(() => {
    const unlisten = listen<string>("server-output", (event) => {
      setLog((prev) => [...prev, event.payload]);
    });
    return () => {
      unlisten.then((fn) => fn());
    };
  }, []);

  useEffect(() => {
    logRef.current?.scrollTo({ top: logRef.current.scrollHeight });
  }, [log]);

  function requestAction(action: ServerAction) {
    if (action.confirm) {
      setConfirmAction(action);
    } else {
      execute(action);
    }
  }

  async function runCommand(command: string) {
    setLog((prev) => [...prev, `\n$ ${command}\n`]);
    const result = await invoke<{ output: string; exit_status: number | null }>(
      "run_server_command",
      { command },
    );
    if (result.exit_status !== null && result.exit_status !== 0) {
      setLog((prev) => [...prev, `\n[Beendet mit Code ${result.exit_status}]\n`]);
    }
  }

  async function execute(action: ServerAction) {
    setConfirmAction(null);
    setRunning(action.id);
    setError(null);
    try {
      await runCommand(commands[action.id]);
    } catch (e) {
      setError(String(e));
    } finally {
      setRunning(null);
    }
  }

  function sleep(ms: number) {
    return new Promise((resolve) => setTimeout(resolve, ms));
  }

  async function executeRestart() {
    setRestartConfirm(false);
    setRunning("restart");
    setError(null);
    try {
      await runCommand(commands.stop);
      const delaySeconds = Number(restartDelay) || 0;
      if (delaySeconds > 0) {
        setLog((prev) => [...prev, `\n[Warte ${delaySeconds}s vor dem Start…]\n`]);
        await sleep(delaySeconds * 1000);
      }
      await runCommand(commands.start);
    } catch (e) {
      setError(String(e));
    } finally {
      setRunning(null);
    }
  }

  async function saveCommand(id: ActionId, value: string) {
    setCommands((prev) => ({ ...prev, [id]: value }));
    await invoke("set_setting", { key: SETTING_KEY[id], value }).catch(() => {});
  }

  async function saveWorkdir(value: string) {
    setWorkdir(value);
    await invoke("set_setting", { key: "server_workdir", value }).catch(() => {});
  }

  async function saveRestartDelay(value: string) {
    setRestartDelay(value);
    if (/^\d+$/.test(value)) {
      await invoke("set_setting", { key: "server_restart_delay_seconds", value }).catch(() => {});
    }
  }

  async function resetToDefaults() {
    for (const action of ACTIONS) {
      await saveCommand(action.id, defaultCommand(workdir, DEFAULT_CHOICE[action.id]));
    }
  }

  return (
    <div className="flex h-full flex-col gap-4">
      <div className="flex items-center justify-between">
        <div className="flex items-center gap-2">
          <h1 className="text-2xl font-semibold">{t("serverControl.title")}</h1>
          <Button variant="ghost" size="icon-sm" title="Hilfe zu diesem Modul" onClick={() => openManual("server-control")}>
            <HelpCircle className="size-4" />
          </Button>
        </div>
        <Button variant="ghost" onClick={() => setEditing((v) => !v)}>
          <Settings2 className="size-4" />
          Befehle {editing ? "ausblenden" : "anpassen"}
        </Button>
      </div>

      <div className="flex flex-wrap gap-2">
        {ACTIONS.map((action) => {
          const Icon = action.icon;
          return (
            <Button
              key={action.id}
              variant={action.variant}
              disabled={running !== null || !commands[action.id]}
              onClick={() => requestAction(action)}
            >
              <Icon className="size-4" />
              {running === action.id ? "Läuft…" : t(action.labelKey)}
            </Button>
          );
        })}
        <Button
          variant="destructive"
          disabled={running !== null || !commands.start || !commands.stop}
          onClick={() => setRestartConfirm(true)}
        >
          <RotateCcw className="size-4" />
          {running === "restart" ? "Startet neu…" : "Neustarten"}
        </Button>
      </div>

      {editing && (
        <div className="space-y-3 rounded-lg border border-border bg-card p-3">
          <div className="space-y-1">
            <label className="text-xs font-medium text-muted-foreground">
              Arbeitsverzeichnis
            </label>
            <input
              value={workdir}
              onChange={(e) => saveWorkdir(e.target.value)}
              className="w-full rounded-md border border-border bg-background px-2 py-1 font-mono text-sm"
            />
          </div>
          {ACTIONS.map((action) => (
            <div key={action.id} className="space-y-1">
              <label className="text-xs font-medium text-muted-foreground">
                {t(action.labelKey)}
              </label>
              <input
                value={commands[action.id]}
                onChange={(e) => saveCommand(action.id, e.target.value)}
                className="w-full rounded-md border border-border bg-background px-2 py-1 font-mono text-sm"
              />
            </div>
          ))}
          <div className="space-y-1">
            <label className="text-xs font-medium text-muted-foreground">
              Wartezeit vor Neustart (Sekunden)
            </label>
            <input
              value={restartDelay}
              onChange={(e) => saveRestartDelay(e.target.value)}
              className="w-20 rounded-md border border-border bg-background px-2 py-1 font-mono text-sm"
            />
            <p className="text-xs text-muted-foreground">
              Zeit zwischen "Server/Channel schließen" und "Server/Channel starten" beim
              Neustarten, damit die Prozesse sauber beendet sind.
            </p>
          </div>
          <div className="flex items-center justify-between">
            <p className="text-xs text-muted-foreground">
              Voreingestellt für ein Menü-Skript, das die Auswahl von stdin liest.
            </p>
            <Button variant="outline" size="sm" onClick={resetToDefaults}>
              Zurücksetzen
            </Button>
          </div>
        </div>
      )}

      {error && <p className="text-sm text-destructive">{error}</p>}

      <div className="flex min-h-0 flex-1 flex-col gap-1">
        <div className="flex items-center justify-between">
          <span className="text-xs font-medium text-muted-foreground">Ausgabe</span>
          <Button variant="ghost" size="sm" onClick={() => setLog([])} disabled={!log.length}>
            <Eraser className="size-3.5" />
            Leeren
          </Button>
        </div>
        <pre
          ref={logRef}
          className="min-h-0 flex-1 overflow-auto whitespace-pre-wrap rounded-lg border border-border bg-muted/30 p-3 font-mono text-xs"
        >
          {log.length ? log.join("") : "Noch keine Ausgabe."}
        </pre>
      </div>

      {confirmAction && (
        <div className="fixed inset-0 flex items-center justify-center bg-black/50">
          <div className="w-96 space-y-3 rounded-lg border border-border bg-card p-4">
            <p className="text-sm font-medium">{t(confirmAction.labelKey)}</p>
            <p className="text-sm text-muted-foreground">{confirmAction.confirm}</p>
            <pre className="overflow-x-auto rounded-md bg-muted/50 p-2 font-mono text-xs">
              {commands[confirmAction.id]}
            </pre>
            <div className="flex justify-end gap-2">
              <Button variant="outline" onClick={() => setConfirmAction(null)}>
                {t("common.cancel")}
              </Button>
              <Button variant="destructive" onClick={() => execute(confirmAction)}>
                Ausführen
              </Button>
            </div>
          </div>
        </div>
      )}

      {restartConfirm && (
        <div className="fixed inset-0 flex items-center justify-center bg-black/50">
          <div className="w-96 space-y-3 rounded-lg border border-border bg-card p-4">
            <p className="text-sm font-medium">Neustarten</p>
            <p className="text-sm text-muted-foreground">
              Der Server wird heruntergefahren, alle Spieler werden getrennt, und nach{" "}
              {restartDelay || 0}s wieder gestartet.
            </p>
            <pre className="overflow-x-auto rounded-md bg-muted/50 p-2 font-mono text-xs">
              {commands.stop}
              {"\n"}
              {commands.start}
            </pre>
            <div className="flex justify-end gap-2">
              <Button variant="outline" onClick={() => setRestartConfirm(false)}>
                {t("common.cancel")}
              </Button>
              <Button variant="destructive" onClick={executeRestart}>
                Ausführen
              </Button>
            </div>
          </div>
        </div>
      )}
    </div>
  );
}
