import { useEffect, useRef, useState } from "react";
import { invoke } from "@tauri-apps/api/core";
import { runAsyncAction } from "@/lib/asyncAction";
import { openManual } from "@/lib/manual";
import { Button } from "@/components/ui/button";
import { Moon, Sun, Snowflake, HelpCircle, RefreshCw, PlayCircle, Eraser, Info, Copy } from "lucide-react";
import { gmCommandsFor, generateWeatherQuest, type WeatherState } from "./weatherQuest";

const QUEST_RELATIVE_PATH = "Weather/Weather_System.lua";

interface ServerCommandResult {
  output: string;
  exit_status: number | null;
}

export function WeatherControl() {
  const [state, setState] = useState<WeatherState | null>(null);
  const [loading, setLoading] = useState(false);
  const [error, setError] = useState<string | null>(null);
  const [busy, setBusy] = useState(false);

  const [deploying, setDeploying] = useState(false);
  const [deployLog, setDeployLog] = useState<string[]>([]);
  const logRef = useRef<HTMLPreElement>(null);

  async function load() {
    await runAsyncAction(() => invoke<WeatherState>("get_weather_state"), {
      onStart: () => {
        setLoading(true);
        setError(null);
      },
      onSuccess: setState,
      onError: setError,
      onFinally: () => setLoading(false),
    });
  }

  useEffect(() => {
    load();
  }, []);

  useEffect(() => {
    logRef.current?.scrollTo({ top: logRef.current.scrollHeight });
  }, [deployLog]);

  async function toggle(field: "night_enabled" | "snow_enabled") {
    if (!state) return;
    const next = { ...state, [field]: !state[field] };
    await runAsyncAction(
      () =>
        invoke<WeatherState>("set_weather_state", {
          nightEnabled: next.night_enabled,
          snowEnabled: next.snow_enabled,
        }),
      {
        onStart: () => {
          setBusy(true);
          setError(null);
        },
        onSuccess: setState,
        onError: setError,
        onFinally: () => setBusy(false),
      },
    );
  }

  // Gleicher Mechanismus wie Broadcast-Systems "Deploy & Neuladen"
  // (BroadcastSystem.tsx::runDeploy): dieselbe `server_cmd_reload_quests`-
  // Einstellung + `run_server_command`, kein separater Weg.
  async function runDeploy() {
    if (!state) return;
    setDeploying(true);
    setError(null);
    setDeployLog((prev) => [...prev, "\n[Generiere Quest-Datei…]\n"]);
    try {
      const content = generateWeatherQuest(state);

      let fileExists = true;
      try {
        await invoke("read_quest_file", { relativePath: QUEST_RELATIVE_PATH });
      } catch {
        fileExists = false;
      }

      if (fileExists) {
        await invoke("write_quest_file", { relativePath: QUEST_RELATIVE_PATH, content });
      } else {
        await invoke("create_quest_file", {
          category: "Weather",
          name: "Weather_System",
          content,
          extension: "lua",
        });
      }
      setDeployLog((prev) => [...prev, "[Quest-Datei geschrieben. Lade auf dem Server neu…]\n"]);

      const workdir =
        (await invoke<string | null>("get_setting", { key: "server_workdir" }).catch(() => null)) ||
        "/usr/home/game";
      const command =
        (await invoke<string | null>("get_setting", { key: "server_cmd_reload_quests" }).catch(
          () => null,
        )) || `cd ${workdir} && echo 4 | sh index.sh`;

      const result = await invoke<ServerCommandResult>("run_server_command", { command });
      setDeployLog((prev) => [...prev, result.output]);
      if (result.exit_status !== null && result.exit_status !== 0) {
        setDeployLog((prev) => [...prev, `\n[Beendet mit Code ${result.exit_status}]\n`]);
      } else {
        setDeployLog((prev) => [...prev, "\n[Fertig]\n"]);
      }
    } catch (e) {
      setError(String(e));
      setDeployLog((prev) => [...prev, `\nFehler: ${String(e)}\n`]);
    } finally {
      setDeploying(false);
    }
  }

  const preview = state ? generateWeatherQuest(state) : "";
  const gmCommands = state ? gmCommandsFor(state) : [];

  return (
    <div className="flex h-full max-w-4xl flex-col gap-4 pb-10">
      <div className="flex items-center justify-between">
        <div className="flex items-center gap-2">
          <h1 className="flex items-center gap-2 text-2xl font-semibold">
            <Moon className="size-6" /> Tag/Nacht & Schnee
          </h1>
          <Button variant="ghost" size="icon-sm" title="Hilfe zu diesem Modul" onClick={() => openManual("weather-control")}>
            <HelpCircle className="size-4" />
          </Button>
        </div>
        <Button variant="outline" onClick={load} disabled={loading}>
          <RefreshCw className={`size-4 ${loading ? "animate-spin" : ""}`} />
          Neu laden
        </Button>
      </div>

      <div className="flex items-start gap-2 rounded-lg border border-amber-500/40 bg-amber-500/10 p-3 text-sm text-amber-700 dark:text-amber-400">
        <Info className="mt-0.5 size-4 shrink-0" />
        <span>
          Regen gibt es auf diesem Server-Code nicht (kein Feature im Quellcode, weder Client noch Server) - nur
          Tag/Nacht und Schnee sind über echte Server-Flags steuerbar. Die Änderung greift server-weit sofort für
          alle Online-Spieler, sobald nach „Deploy & Neuladen" irgendein Spieler das nächste Mal einloggt (kein
          Server-Neustart nötig). Für sofortige Wirkung ohne auf einen Login zu warten, kannst du stattdessen die
          GM-Befehle unten direkt im Spiel eingeben.
        </span>
      </div>

      {error && <p className="whitespace-pre-wrap text-sm text-destructive">{error}</p>}

      {state && (
        <div className="space-y-3 rounded-lg border border-border p-4">
          <button
            onClick={() => toggle("night_enabled")}
            disabled={busy}
            className="flex w-full items-center justify-between gap-2 rounded-md border border-border px-3 py-2 text-left hover:bg-muted/50"
          >
            <span className="flex items-center gap-2 text-sm font-medium">
              {state.night_enabled ? <Moon className="size-4" /> : <Sun className="size-4" />}
              {state.night_enabled ? "Nacht erzwungen" : "Tag (normal)"}
            </span>
            <span
              className={`size-4 shrink-0 rounded-full border ${
                state.night_enabled ? "border-green-600 bg-green-600" : "border-border bg-transparent"
              }`}
            />
          </button>
          <button
            onClick={() => toggle("snow_enabled")}
            disabled={busy}
            className="flex w-full items-center justify-between gap-2 rounded-md border border-border px-3 py-2 text-left hover:bg-muted/50"
          >
            <span className="flex items-center gap-2 text-sm font-medium">
              <Snowflake className="size-4" />
              Schnee {state.snow_enabled ? "aktiv" : "aus"}
            </span>
            <span
              className={`size-4 shrink-0 rounded-full border ${
                state.snow_enabled ? "border-green-600 bg-green-600" : "border-border bg-transparent"
              }`}
            />
          </button>
        </div>
      )}

      <div className="space-y-2 rounded-lg border border-border p-4">
        <h2 className="text-sm font-medium text-muted-foreground">GM-Befehle (sofortige Wirkung, manuell im Spiel)</h2>
        {gmCommands.map((cmd) => (
          <div key={cmd} className="flex items-center gap-2">
            <code className="flex-1 rounded-md border border-border bg-muted/30 px-2 py-1 font-mono text-xs">{cmd}</code>
            <Button size="icon-sm" variant="ghost" title="Kopieren" onClick={() => navigator.clipboard.writeText(cmd)}>
              <Copy className="size-3.5" />
            </Button>
          </div>
        ))}
      </div>

      <div className="space-y-2 rounded-lg border border-border p-4">
        <h2 className="text-sm font-medium text-muted-foreground">Vorschau (generierte Quest-Datei)</h2>
        <pre className="max-h-64 overflow-auto rounded-md border border-border bg-muted/30 p-2 font-mono text-xs">
          {preview}
        </pre>
      </div>

      <div className="flex min-h-0 flex-1 flex-col gap-1">
        <div className="flex items-center justify-between">
          <Button onClick={runDeploy} disabled={deploying || !state}>
            <PlayCircle className="size-4" />
            {deploying ? "Wird eingespielt…" : "Deploy & Neuladen"}
          </Button>
          <Button variant="ghost" size="sm" onClick={() => setDeployLog([])} disabled={!deployLog.length}>
            <Eraser className="size-3.5" />
            Ausgabe leeren
          </Button>
        </div>
        <p className="text-xs text-muted-foreground">
          Schreibt <code>{QUEST_RELATIVE_PATH}</code> auf den Server und führt denselben Befehl aus wie
          „Quests reloaden" in der Server-Steuerung.
        </p>
        <pre
          ref={logRef}
          className="min-h-0 flex-1 overflow-auto whitespace-pre-wrap rounded-lg border border-border bg-muted/30 p-3 font-mono text-xs"
        >
          {deployLog.length ? deployLog.join("") : "Noch keine Ausgabe."}
        </pre>
      </div>
    </div>
  );
}
