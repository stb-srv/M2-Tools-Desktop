import { useEffect, useRef, useState } from "react";
import { invoke } from "@tauri-apps/api/core";
import { listen } from "@tauri-apps/api/event";
import { runAsyncAction } from "@/lib/asyncAction";
import { openManual } from "@/lib/manual";
import { Button } from "@/components/ui/button";
import {
  Megaphone,
  HelpCircle,
  RefreshCw,
  Plus,
  Pencil,
  Trash2,
  X,
  PlayCircle,
  Eraser,
  Info,
} from "lucide-react";
import { generateBroadcastQuest, type BroadcastMessage } from "./broadcastQuest";

const QUEST_RELATIVE_PATH = "Broadcast/Broadcast_System.lua";

interface ServerCommandResult {
  output: string;
  exit_status: number | null;
}

export function BroadcastSystem() {
  const [messages, setMessages] = useState<BroadcastMessage[] | null>(null);
  const [loading, setLoading] = useState(false);
  const [error, setError] = useState<string | null>(null);
  const [busyId, setBusyId] = useState<number | null>(null);

  const [editingId, setEditingId] = useState<number | null>(null);
  const [text, setText] = useState("");
  const [intervalMinutes, setIntervalMinutes] = useState("30");

  const [deploying, setDeploying] = useState(false);
  const [deployLog, setDeployLog] = useState<string[]>([]);
  const [deployOpen, setDeployOpen] = useState(false);
  const logRef = useRef<HTMLPreElement>(null);

  async function load() {
    await runAsyncAction(() => invoke<BroadcastMessage[]>("list_broadcast_messages"), {
      onStart: () => {
        setLoading(true);
        setError(null);
      },
      onSuccess: setMessages,
      onError: setError,
      onFinally: () => setLoading(false),
    });
  }

  useEffect(() => {
    load();
  }, []);

  // Live-Ausgabe des Server-Befehls kommt als Ereignis-Strom (index.sh ist ein
  // interaktives Menüskript, dessen Ausgabe der Backend-Befehl live weiterreicht,
  // siehe `run_server_command`/`server-output`) - ohne diesen Listener kam beim
  // ersten echten Test gar nichts an, bis der Befehl komplett fertig war, und
  // dann alles auf einmal als ein einziger unformatierter Textblock ("Log nur
  // sehr schwer zu sehen"). Gleiches Muster wie Quest Builder/Server-Steuerung.
  useEffect(() => {
    const unlisten = listen<string>("server-output", (event) => {
      setDeployLog((prev) => [...prev, event.payload]);
    });
    return () => {
      unlisten.then((fn) => fn());
    };
  }, []);

  useEffect(() => {
    logRef.current?.scrollTo({ top: logRef.current.scrollHeight });
  }, [deployLog]);

  function resetForm() {
    setEditingId(null);
    setText("");
    setIntervalMinutes("30");
  }

  function startEdit(message: BroadcastMessage) {
    setEditingId(message.id);
    setText(message.text);
    setIntervalMinutes(String(message.interval_minutes));
  }

  async function saveMessage() {
    const trimmed = text.trim();
    const minutes = Number(intervalMinutes);
    if (!trimmed || !Number.isFinite(minutes) || minutes < 1) return;

    await runAsyncAction(
      () =>
        editingId !== null
          ? invoke("update_broadcast_message", { id: editingId, text: trimmed, intervalMinutes: minutes })
          : invoke("create_broadcast_message", { text: trimmed, intervalMinutes: minutes }),
      {
        onStart: () => setError(null),
        onSuccess: async () => {
          resetForm();
          await load();
        },
        onError: setError,
      },
    );
  }

  async function toggleEnabled(message: BroadcastMessage) {
    await runAsyncAction(
      () => invoke("set_broadcast_message_enabled", { id: message.id, enabled: !message.enabled }),
      {
        onStart: () => {
          setBusyId(message.id);
          setError(null);
        },
        onSuccess: load,
        onError: setError,
        onFinally: () => setBusyId(null),
      },
    );
  }

  async function deleteMessage(id: number) {
    await runAsyncAction(() => invoke("delete_broadcast_message", { id }), {
      onStart: () => {
        setBusyId(id);
        setError(null);
      },
      onSuccess: async () => {
        if (editingId === id) resetForm();
        await load();
      },
      onError: setError,
      onFinally: () => setBusyId(null),
    });
  }

  // Gleicher Mechanismus wie Quest Builders "Kompilieren & Neuladen"
  // (QuestBuilder.tsx::runDeploy): dieselbe `server_cmd_reload_quests`-
  // Einstellung + `run_server_command`, kein separater Weg. Neu ist nur der
  // Schritt davor, der die generierte Datei über die schon vorhandenen
  // Quest-Builder-Commands schreibt (anlegen beim ersten Mal, sonst
  // überschreiben).
  async function runDeploy() {
    setDeployOpen(true);
    setDeploying(true);
    setError(null);
    setDeployLog((prev) => [...prev, "\n[Generiere Quest-Datei…]\n"]);
    try {
      const content = generateBroadcastQuest(messages ?? []);

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
          category: "Broadcast",
          name: "Broadcast_System",
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

  const preview = generateBroadcastQuest(messages ?? []);
  const activeCount = (messages ?? []).filter((m) => m.enabled).length;

  return (
    <div className="flex h-full max-w-4xl flex-col gap-4 pb-10">
      <div className="flex items-center justify-between">
        <div className="flex items-center gap-2">
          <h1 className="flex items-center gap-2 text-2xl font-semibold">
            <Megaphone className="size-6" /> Broadcast-System
          </h1>
          <Button variant="ghost" size="icon-sm" title="Hilfe zu diesem Modul" onClick={() => openManual("broadcast-system")}>
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
          Jede Nachricht bekommt ihr eigenes Intervall und wird per <code>notice_all()</code> an alle Spieler
          auf allen Cores gesendet (kleine Ansagezeile, kein großes Banner - dafür gibt es auf diesem Server
          aktuell keinen funktionierenden globalen Befehl). Ein Timer wird erst beim ersten Login nach einem
          „Deploy & Neuladen" scharfgeschaltet, nicht sofort beim Speichern hier.
        </span>
      </div>

      {error && <p className="whitespace-pre-wrap text-sm text-destructive">{error}</p>}

      <div className="space-y-2 rounded-lg border border-border p-4">
        <h2 className="text-sm font-medium text-muted-foreground">
          {editingId !== null ? "Nachricht bearbeiten" : "Neue Nachricht"}
        </h2>
        <textarea
          value={text}
          onChange={(e) => setText(e.target.value)}
          placeholder="Ansagetext…"
          rows={2}
          className="w-full rounded-md border border-border bg-background px-2 py-1 text-sm"
        />
        <div className="flex items-center gap-2">
          <label className="flex items-center gap-2 text-sm text-muted-foreground">
            Alle
            <input
              type="number"
              min={1}
              value={intervalMinutes}
              onChange={(e) => setIntervalMinutes(e.target.value)}
              className="w-20 rounded-md border border-border bg-background px-2 py-1 text-sm"
            />
            Minuten
          </label>
          <Button size="sm" onClick={saveMessage} disabled={!text.trim() || Number(intervalMinutes) < 1}>
            {editingId !== null ? <Pencil className="size-3.5" /> : <Plus className="size-3.5" />}
            {editingId !== null ? "Speichern" : "Hinzufügen"}
          </Button>
          {editingId !== null && (
            <Button size="sm" variant="ghost" onClick={resetForm}>
              <X className="size-3.5" />
              Abbrechen
            </Button>
          )}
        </div>
      </div>

      <div className="space-y-2 rounded-lg border border-border p-4">
        <h2 className="text-sm font-medium text-muted-foreground">
          Nachrichten ({activeCount} von {(messages ?? []).length} aktiv)
        </h2>
        {(messages ?? []).length === 0 && (
          <p className="text-sm text-muted-foreground">Noch keine Nachrichten angelegt.</p>
        )}
        {(messages ?? []).map((m) => (
          <div key={m.id} className="flex items-start gap-2 border-t border-border py-2 first:border-t-0">
            <button
              title={m.enabled ? "Aktiv - klicken zum Deaktivieren" : "Deaktiviert - klicken zum Aktivieren"}
              onClick={() => toggleEnabled(m)}
              disabled={busyId === m.id}
              className={`mt-0.5 size-4 shrink-0 rounded-full border ${
                m.enabled ? "border-green-600 bg-green-600" : "border-border bg-transparent"
              }`}
            />
            <div className="min-w-0 flex-1">
              <p className={`whitespace-pre-wrap text-sm ${m.enabled ? "" : "text-muted-foreground line-through"}`}>
                {m.text}
              </p>
              <p className="text-xs text-muted-foreground">Alle {m.interval_minutes} Minuten</p>
            </div>
            <Button size="icon-sm" variant="ghost" onClick={() => startEdit(m)}>
              <Pencil className="size-3.5" />
            </Button>
            <Button size="icon-sm" variant="ghost" disabled={busyId === m.id} onClick={() => deleteMessage(m.id)}>
              <Trash2 className="size-3.5" />
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

      <div className="space-y-1">
        <div className="flex items-center justify-between">
          <Button onClick={runDeploy} disabled={deploying}>
            <PlayCircle className="size-4" />
            {deploying ? "Wird eingespielt…" : "Deploy & Neuladen"}
          </Button>
          {deployLog.length > 0 && (
            <Button variant="outline" size="sm" onClick={() => setDeployOpen(true)}>
              Ausgabe anzeigen
            </Button>
          )}
        </div>
        <p className="text-xs text-muted-foreground">
          Schreibt <code>{QUEST_RELATIVE_PATH}</code> auf den Server und führt denselben Befehl aus wie
          „Quests reloaden" in der Server-Steuerung.
        </p>
      </div>

      {deployOpen && (
        <div className="fixed inset-0 flex items-center justify-center bg-black/50">
          <div className="flex max-h-[70vh] w-[36rem] flex-col gap-3 rounded-lg border border-border bg-card p-4">
            <div className="flex items-center justify-between">
              <p className="text-sm font-medium">Broadcast-System einspielen</p>
              <div className="flex items-center gap-1">
                <Button variant="ghost" size="icon-sm" onClick={() => setDeployLog([])} disabled={!deployLog.length} title="Ausgabe leeren">
                  <Eraser className="size-3.5" />
                </Button>
                <button onClick={() => setDeployOpen(false)} className="text-muted-foreground hover:text-foreground">
                  <X className="size-4" />
                </button>
              </div>
            </div>
            <pre
              ref={logRef}
              className="min-h-0 flex-1 overflow-auto whitespace-pre-wrap rounded-md border border-border bg-muted/30 p-2 font-mono text-xs"
            >
              {deployLog.length ? deployLog.join("") : "…"}
            </pre>
          </div>
        </div>
      )}
    </div>
  );
}
