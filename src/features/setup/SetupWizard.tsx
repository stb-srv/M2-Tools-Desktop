import { useState } from "react";
import { invoke } from "@tauri-apps/api/core";
import { open } from "@tauri-apps/plugin-dialog";
import { Button } from "@/components/ui/button";
import { FolderOpen, CheckCircle2, XCircle } from "lucide-react";

type Step = "client" | "ssh" | "mysql";
type TestState = "idle" | "testing" | "ok" | "error";

const STEPS: Step[] = ["client", "ssh", "mysql"];
const STEP_LABELS: Record<Step, string> = {
  client: "Client-Pfad",
  ssh: "SSH-Verbindung",
  mysql: "MySQL-Verbindung",
};

export function SetupWizard({ onComplete }: { onComplete: () => void }) {
  const [stepIndex, setStepIndex] = useState(0);
  const step = STEPS[stepIndex];

  const [clientPath, setClientPath] = useState("");
  const [clientTest, setClientTest] = useState<TestState>("idle");
  const [clientError, setClientError] = useState<string | null>(null);

  const [sshHost, setSshHost] = useState("");
  const [sshPort, setSshPort] = useState("22");
  const [sshUser, setSshUser] = useState("");
  const [sshPassword, setSshPassword] = useState("");
  const [sshTest, setSshTest] = useState<TestState>("idle");
  const [sshError, setSshError] = useState<string | null>(null);

  const [mysqlHost, setMysqlHost] = useState("");
  const [mysqlPort, setMysqlPort] = useState("3306");
  const [mysqlUser, setMysqlUser] = useState("");
  const [mysqlPassword, setMysqlPassword] = useState("");
  const [mysqlTest, setMysqlTest] = useState<TestState>("idle");
  const [mysqlError, setMysqlError] = useState<string | null>(null);

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
        await invoke("set_setting", { key: "client_path", value: clientPath });
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

  async function testSsh() {
    setSshTest("testing");
    setSshError(null);
    try {
      await invoke("test_ssh_connection", {
        config: { host: sshHost, port: Number(sshPort), username: sshUser },
        password: sshPassword,
      });
      await invoke("set_setting", { key: "ssh_host", value: sshHost });
      await invoke("set_setting", { key: "ssh_port", value: sshPort });
      await invoke("set_setting", { key: "ssh_username", value: sshUser });
      await invoke("store_credential", { account: "ssh_password", secret: sshPassword });
      setSshTest("ok");
    } catch (e) {
      setSshTest("error");
      setSshError(String(e));
    }
  }

  async function testMysql() {
    setMysqlTest("testing");
    setMysqlError(null);
    try {
      const config = {
        host: mysqlHost,
        port: Number(mysqlPort),
        username: mysqlUser,
        database: null,
      };
      await invoke("test_mysql_connection", { config, password: mysqlPassword });
      await invoke("connect_mysql", { config, password: mysqlPassword });
      await invoke("set_setting", { key: "mysql_host", value: mysqlHost });
      await invoke("set_setting", { key: "mysql_port", value: mysqlPort });
      await invoke("set_setting", { key: "mysql_username", value: mysqlUser });
      await invoke("store_credential", { account: "mysql_password", secret: mysqlPassword });
      setMysqlTest("ok");
    } catch (e) {
      setMysqlTest("error");
      setMysqlError(String(e));
    }
  }

  async function finish() {
    await invoke("set_setting", { key: "setup_completed", value: "true" });
    onComplete();
  }

  function StatusIcon({ state }: { state: TestState }) {
    if (state === "ok") return <CheckCircle2 className="size-4 text-green-500" />;
    if (state === "error") return <XCircle className="size-4 text-destructive" />;
    return null;
  }

  return (
    <div className="flex h-screen items-center justify-center bg-background p-6">
      <div className="w-full max-w-lg space-y-6 rounded-lg border border-border bg-card p-6">
        <div>
          <h1 className="text-xl font-semibold">Willkommen bei M2Manager</h1>
          <p className="text-sm text-muted-foreground">
            Ein paar Angaben, bevor es losgeht ({stepIndex + 1}/{STEPS.length}:{" "}
            {STEP_LABELS[step]})
          </p>
        </div>

        <div className="flex gap-1">
          {STEPS.map((s, i) => (
            <div
              key={s}
              className={`h-1 flex-1 rounded-full ${
                i <= stepIndex ? "bg-primary" : "bg-muted"
              }`}
            />
          ))}
        </div>

        {step === "client" && (
          <div className="space-y-3">
            <p className="text-sm text-muted-foreground">
              Der Ordner deines Metin2-Game-Clients (dort liegt{" "}
              <code>granny2.dll</code>) wird für den 3D-Modell-Viewer
              benötigt.
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
            <div className="flex items-center gap-2">
              <Button
                variant="outline"
                onClick={testClientPath}
                disabled={!clientPath || clientTest === "testing"}
              >
                {clientTest === "testing" ? "Prüfe…" : "Testen"}
              </Button>
              <StatusIcon state={clientTest} />
            </div>
            {clientError && <p className="text-sm text-destructive">{clientError}</p>}
          </div>
        )}

        {step === "ssh" && (
          <div className="space-y-2">
            <p className="text-sm text-muted-foreground">
              Zugangsdaten für den SSH-Zugriff auf deinen Gameserver (Server
              starten/stoppen, Logs, Quest-Reload).
            </p>
            <input
              value={sshHost}
              onChange={(e) => setSshHost(e.target.value)}
              placeholder="Host / IP"
              className="w-full rounded-md border border-border bg-background px-2 py-1 text-sm"
            />
            <input
              value={sshPort}
              onChange={(e) => setSshPort(e.target.value)}
              placeholder="Port (Standard 22)"
              className="w-full rounded-md border border-border bg-background px-2 py-1 text-sm"
            />
            <input
              value={sshUser}
              onChange={(e) => setSshUser(e.target.value)}
              placeholder="Benutzername"
              className="w-full rounded-md border border-border bg-background px-2 py-1 text-sm"
            />
            <input
              type="password"
              value={sshPassword}
              onChange={(e) => setSshPassword(e.target.value)}
              placeholder="Passwort"
              className="w-full rounded-md border border-border bg-background px-2 py-1 text-sm"
            />
            <div className="flex items-center gap-2">
              <Button
                variant="outline"
                onClick={testSsh}
                disabled={!sshHost || !sshUser || sshTest === "testing"}
              >
                {sshTest === "testing" ? "Verbinde…" : "Testen"}
              </Button>
              <StatusIcon state={sshTest} />
            </div>
            {sshError && <p className="text-sm text-destructive">{sshError}</p>}
          </div>
        )}

        {step === "mysql" && (
          <div className="space-y-2">
            <p className="text-sm text-muted-foreground">
              Zugangsdaten für die MySQL-Datenbank (Shop-Editor,
              Datenbank-Explorer).
            </p>
            <input
              value={mysqlHost}
              onChange={(e) => setMysqlHost(e.target.value)}
              placeholder="Host / IP"
              className="w-full rounded-md border border-border bg-background px-2 py-1 text-sm"
            />
            <input
              value={mysqlPort}
              onChange={(e) => setMysqlPort(e.target.value)}
              placeholder="Port (Standard 3306)"
              className="w-full rounded-md border border-border bg-background px-2 py-1 text-sm"
            />
            <input
              value={mysqlUser}
              onChange={(e) => setMysqlUser(e.target.value)}
              placeholder="Benutzername"
              className="w-full rounded-md border border-border bg-background px-2 py-1 text-sm"
            />
            <input
              type="password"
              value={mysqlPassword}
              onChange={(e) => setMysqlPassword(e.target.value)}
              placeholder="Passwort"
              className="w-full rounded-md border border-border bg-background px-2 py-1 text-sm"
            />
            <div className="flex items-center gap-2">
              <Button
                variant="outline"
                onClick={testMysql}
                disabled={!mysqlHost || !mysqlUser || mysqlTest === "testing"}
              >
                {mysqlTest === "testing" ? "Verbinde…" : "Testen"}
              </Button>
              <StatusIcon state={mysqlTest} />
            </div>
            {mysqlError && <p className="text-sm text-destructive">{mysqlError}</p>}
          </div>
        )}

        <div className="flex justify-between pt-2">
          <Button
            variant="ghost"
            onClick={() => setStepIndex((i) => Math.max(0, i - 1))}
            disabled={stepIndex === 0}
          >
            Zurück
          </Button>
          {stepIndex < STEPS.length - 1 ? (
            <Button onClick={() => setStepIndex((i) => i + 1)}>Weiter</Button>
          ) : (
            <Button onClick={finish}>Fertig</Button>
          )}
        </div>
      </div>
    </div>
  );
}
