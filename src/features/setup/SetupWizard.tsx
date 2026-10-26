import { useState } from "react";
import { invoke } from "@tauri-apps/api/core";
import { open } from "@tauri-apps/plugin-dialog";
import { Button } from "@/components/ui/button";
import { FolderOpen } from "lucide-react";
import { EMPTY_MYSQL, EMPTY_SSH, sshComplete, type MysqlValues, type SshValues } from "@/lib/connection";
import { useConnectionProfiles } from "@/store/connectionProfiles";
import {
  MysqlFields,
  MysqlTestButton,
  SshFields,
  SshTestButton,
  TestResult,
  TextField,
  type TestState,
} from "@/features/connection/ConnectionFields";

type Step = "profile" | "ssh" | "mysql" | "client";

const STEPS: Step[] = ["profile", "ssh", "mysql", "client"];
const STEP_LABELS: Record<Step, string> = {
  profile: "Server-Profil",
  ssh: "SSH-Verbindung",
  mysql: "MySQL-Verbindung",
  client: "Client-Ordner",
};

export function SetupWizard({ onComplete }: { onComplete: () => void }) {
  const [stepIndex, setStepIndex] = useState(0);
  const step = STEPS[stepIndex];

  const [profileName, setProfileName] = useState("");
  const [ssh, setSsh] = useState<SshValues>(EMPTY_SSH);
  const [mysql, setMysql] = useState<MysqlValues>(EMPTY_MYSQL);

  const [clientPath, setClientPath] = useState("");
  const [clientTest, setClientTest] = useState<TestState>("idle");
  const [clientError, setClientError] = useState<string | null>(null);

  const [finishing, setFinishing] = useState(false);
  const [finishError, setFinishError] = useState<string | null>(null);

  async function pickClientPath() {
    const selected = await open({ directory: true, title: "Metin2-Client-Ordner auswählen" });
    if (typeof selected === "string") {
      setClientPath(selected);
      setClientTest("idle");
    }
  }

  async function testClientPath() {
    setClientTest("testing");
    setClientError(null);
    try {
      const ok = await invoke<boolean>("check_client_path", { path: clientPath });
      if (ok) {
        setClientTest("ok");
      } else {
        setClientTest("error");
        setClientError("granny2.dll wurde in diesem Ordner nicht gefunden.");
      }
    } catch (e) {
      setClientTest("error");
      setClientError(String(e));
    }
  }

  function next() {
    if (step === "ssh" && !mysql.host) setMysql({ ...mysql, host: ssh.host });
    setStepIndex((i) => i + 1);
  }

  async function finish() {
    setFinishing(true);
    setFinishError(null);
    try {
      // Ohne SSH-Daten kein Profil - das lässt sich später auf der Startseite anlegen.
      if (profileName.trim() && sshComplete(ssh)) {
        await useConnectionProfiles.getState().create(profileName.trim(), ssh, mysql);
      }
      if (clientPath) await invoke("set_setting", { key: "client_path", value: clientPath });
      await invoke("set_setting", { key: "setup_completed", value: "true" });
      onComplete();
    } catch (e) {
      setFinishError(String(e));
    } finally {
      setFinishing(false);
    }
  }

  return (
    <div className="flex h-screen items-center justify-center bg-background p-6">
      <div className="w-full max-w-lg space-y-6 rounded-lg border border-border bg-card p-6">
        <div>
          <h1 className="text-xl font-semibold">Willkommen bei M2Manager</h1>
          <p className="text-sm text-muted-foreground">
            Ein paar Angaben, bevor es losgeht ({stepIndex + 1}/{STEPS.length}: {STEP_LABELS[step]})
          </p>
        </div>

        <div className="flex gap-1">
          {STEPS.map((s, i) => (
            <div key={s} className={`h-1 flex-1 rounded-full ${i <= stepIndex ? "bg-primary" : "bg-muted"}`} />
          ))}
        </div>

        {step === "profile" && (
          <div className="space-y-3">
            <p className="text-sm text-muted-foreground">
              Gib deinem Server einen Namen. Unter diesem Profil speichert M2Manager die
              Zugangsdaten - weitere Server legst du später direkt auf der Startseite an.
            </p>
            <TextField
              label="Profilname"
              value={profileName}
              onChange={setProfileName}
              placeholder="z.B. Live-Server oder Testserver"
              autoFocus
            />
          </div>
        )}

        {step === "ssh" && (
          <div className="space-y-3">
            <p className="text-sm text-muted-foreground">
              SSH-Zugang zum Gameserver (Server starten/stoppen, Logs, Quests, Server-Dateien).
            </p>
            <SshFields value={ssh} onChange={setSsh} />
            <SshTestButton value={ssh} />
          </div>
        )}

        {step === "mysql" && (
          <div className="space-y-3">
            <p className="text-sm text-muted-foreground">
              MySQL-Zugang (Item-, Shop- und Mob-Editor, Accounts, Statistiken).
            </p>
            <MysqlFields value={mysql} onChange={setMysql} />
            <MysqlTestButton value={mysql} />
          </div>
        )}

        {step === "client" && (
          <div className="space-y-3">
            <p className="text-sm text-muted-foreground">
              Der Ordner deines Metin2-Clients (dort liegt <code>granny2.dll</code>) - für
              3D-Modelle und Item-Icons. Kannst du auch später in den Einstellungen setzen.
            </p>
            <div className="flex items-center gap-2">
              <Button variant="outline" onClick={pickClientPath}>
                <FolderOpen className="size-4" />
                Ordner auswählen
              </Button>
              <span className="truncate text-sm text-muted-foreground">
                {clientPath || "Kein Ordner ausgewählt"}
              </span>
            </div>
            <div className="flex items-start gap-3">
              <Button
                variant="outline"
                size="sm"
                onClick={testClientPath}
                disabled={!clientPath || clientTest === "testing"}
              >
                {clientTest === "testing" ? "Prüfe…" : "Prüfen"}
              </Button>
              {clientTest === "ok" ? (
                <span className="text-sm text-green-600">granny2.dll gefunden</span>
              ) : (
                <TestResult state={clientTest} error={clientError} />
              )}
            </div>
          </div>
        )}

        {finishError && <p className="text-sm text-destructive">{finishError}</p>}

        <div className="flex justify-between pt-2">
          <Button
            variant="ghost"
            onClick={() => setStepIndex((i) => Math.max(0, i - 1))}
            disabled={stepIndex === 0 || finishing}
          >
            Zurück
          </Button>
          {stepIndex < STEPS.length - 1 ? (
            <Button onClick={next} disabled={step === "profile" && !profileName.trim()}>
              Weiter
            </Button>
          ) : (
            <Button onClick={finish} disabled={finishing}>
              {finishing ? "Speichere…" : "Fertig"}
            </Button>
          )}
        </div>
      </div>
    </div>
  );
}
