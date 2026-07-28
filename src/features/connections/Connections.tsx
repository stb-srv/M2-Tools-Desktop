import { useEffect, useState } from "react";
import { useTranslation } from "react-i18next";
import { invoke } from "@tauri-apps/api/core";
import { open } from "@tauri-apps/plugin-dialog";
import { Button } from "@/components/ui/button";
import { PasswordInput } from "@/components/ui/password-input";
import { CheckCircle2, XCircle, FolderOpen } from "lucide-react";

type TestState = "idle" | "testing" | "ok" | "error";
type SshAuthMode = "password" | "key";

export function Connections() {
  const { t } = useTranslation();

  const [sshHost, setSshHost] = useState("");
  const [sshPort, setSshPort] = useState("22");
  const [sshUser, setSshUser] = useState("");
  const [sshAuthMode, setSshAuthMode] = useState<SshAuthMode>("password");
  const [sshPassword, setSshPassword] = useState("");
  const [sshKeyPath, setSshKeyPath] = useState("");
  const [sshKeyPassphrase, setSshKeyPassphrase] = useState("");
  const [sshTest, setSshTest] = useState<TestState>("idle");
  const [sshError, setSshError] = useState<string | null>(null);

  const [mysqlHost, setMysqlHost] = useState("");
  const [mysqlPort, setMysqlPort] = useState("3306");
  const [mysqlUser, setMysqlUser] = useState("");
  const [mysqlPassword, setMysqlPassword] = useState("");
  const [mysqlTest, setMysqlTest] = useState<TestState>("idle");
  const [mysqlError, setMysqlError] = useState<string | null>(null);

  useEffect(() => {
    (async () => {
      const get = (key: string) =>
        invoke<string | null>("get_setting", { key }).catch(() => null);

      setSshHost((await get("ssh_host")) ?? "");
      setSshPort((await get("ssh_port")) ?? "22");
      setSshUser((await get("ssh_username")) ?? "");
      setSshAuthMode(((await get("ssh_auth_mode")) as SshAuthMode) ?? "password");
      setSshKeyPath((await get("ssh_key_path")) ?? "");

      setMysqlHost((await get("mysql_host")) ?? "");
      setMysqlPort((await get("mysql_port")) ?? "3306");
      setMysqlUser((await get("mysql_username")) ?? "");

      // Secrets stay in the Windows Credential Manager; prefill so the user
      // doesn't have to retype them just to change a host or port.
      const sshSecret = await invoke<string>("get_credential", {
        account: "ssh_password",
      }).catch(() => null);
      if (sshSecret) setSshPassword(sshSecret);

      const keyPass = await invoke<string>("get_credential", {
        account: "ssh_key_passphrase",
      }).catch(() => null);
      if (keyPass) setSshKeyPassphrase(keyPass);

      const mysqlSecret = await invoke<string>("get_credential", {
        account: "mysql_password",
      }).catch(() => null);
      if (mysqlSecret) setMysqlPassword(mysqlSecret);
    })();
  }, []);

  async function pickSshKey() {
    const selected = await open({
      multiple: false,
      title: "Privaten SSH-Schlüssel auswählen",
    });
    if (typeof selected === "string") setSshKeyPath(selected);
  }

  async function saveAndTestSsh() {
    setSshTest("testing");
    setSshError(null);
    try {
      const auth =
        sshAuthMode === "password"
          ? { type: "password", password: sshPassword }
          : {
              type: "private_key",
              path: sshKeyPath,
              passphrase: sshKeyPassphrase || null,
            };
      await invoke("test_ssh_connection", {
        config: { host: sshHost, port: Number(sshPort), username: sshUser },
        auth,
      });
      await invoke("set_setting", { key: "ssh_host", value: sshHost });
      await invoke("set_setting", { key: "ssh_port", value: sshPort });
      await invoke("set_setting", { key: "ssh_username", value: sshUser });
      await invoke("set_setting", { key: "ssh_auth_mode", value: sshAuthMode });
      if (sshAuthMode === "password") {
        await invoke("store_credential", {
          account: "ssh_password",
          secret: sshPassword,
        });
      } else {
        await invoke("set_setting", { key: "ssh_key_path", value: sshKeyPath });
        if (sshKeyPassphrase) {
          await invoke("store_credential", {
            account: "ssh_key_passphrase",
            secret: sshKeyPassphrase,
          });
        }
      }
      setSshTest("ok");
    } catch (e) {
      setSshTest("error");
      setSshError(String(e));
    }
  }

  async function saveAndTestMysql() {
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
      await invoke("store_credential", {
        account: "mysql_password",
        secret: mysqlPassword,
      });
      setMysqlTest("ok");
    } catch (e) {
      setMysqlTest("error");
      setMysqlError(String(e));
    }
  }

  return (
    <div className="max-w-2xl space-y-8">
      <h1 className="text-2xl font-semibold">{t("connections.title")}</h1>

      <section className="space-y-2 rounded-lg border border-border bg-card p-4">
        <h2 className="font-medium">{t("connections.ssh")}</h2>
        <p className="text-xs text-muted-foreground">
          Für Server-Steuerung (Starten/Stoppen, Logs, Quest-Reload).
        </p>
        <div className="grid grid-cols-3 gap-2">
          <Field
            label={t("connections.host")}
            value={sshHost}
            onChange={setSshHost}
            className="col-span-2"
          />
          <Field label={t("connections.port")} value={sshPort} onChange={setSshPort} />
        </div>
        <Field
          label={t("connections.username")}
          value={sshUser}
          onChange={setSshUser}
        />

        <div className="flex overflow-hidden rounded-md border border-border text-sm">
          <button
            onClick={() => setSshAuthMode("password")}
            className={`flex-1 px-3 py-1 ${sshAuthMode === "password" ? "bg-primary text-primary-foreground" : ""}`}
          >
            {t("connections.password")}
          </button>
          <button
            onClick={() => setSshAuthMode("key")}
            className={`flex-1 px-3 py-1 ${sshAuthMode === "key" ? "bg-primary text-primary-foreground" : ""}`}
          >
            SSH-Key
          </button>
        </div>

        {sshAuthMode === "password" ? (
          <PasswordInput
            value={sshPassword}
            onChange={setSshPassword}
            placeholder={t("connections.password")}
          />
        ) : (
          <>
            <div className="flex items-center gap-2">
              <Button variant="outline" onClick={pickSshKey} className="shrink-0">
                <FolderOpen className="size-4" />
                {t("connections.privateKey")}
              </Button>
              <span className="truncate text-sm text-muted-foreground">
                {sshKeyPath || "Keine Datei ausgewählt"}
              </span>
            </div>
            <PasswordInput
              value={sshKeyPassphrase}
              onChange={setSshKeyPassphrase}
              placeholder="Passphrase (falls vorhanden)"
            />
          </>
        )}

        <div className="flex items-center gap-2 pt-1">
          <Button
            onClick={saveAndTestSsh}
            disabled={!sshHost || !sshUser || sshTest === "testing"}
          >
            {sshTest === "testing" ? "Verbinde…" : `${t("connections.testConnection")} & ${t("connections.save")}`}
          </Button>
          <StatusIcon state={sshTest} />
        </div>
        {sshError && <p className="text-sm text-destructive">{sshError}</p>}
      </section>

      <section className="space-y-2 rounded-lg border border-border bg-card p-4">
        <h2 className="font-medium">{t("connections.mysql")}</h2>
        <p className="text-xs text-muted-foreground">
          Für Shop-Editor, Datenbank-Explorer und Dashboard-Statistiken.
        </p>
        <div className="grid grid-cols-3 gap-2">
          <Field
            label={t("connections.host")}
            value={mysqlHost}
            onChange={setMysqlHost}
            className="col-span-2"
          />
          <Field
            label={t("connections.port")}
            value={mysqlPort}
            onChange={setMysqlPort}
          />
        </div>
        <Field
          label={t("connections.username")}
          value={mysqlUser}
          onChange={setMysqlUser}
        />
        <PasswordInput
          value={mysqlPassword}
          onChange={setMysqlPassword}
          placeholder={t("connections.password")}
        />
        <div className="flex items-center gap-2 pt-1">
          <Button
            onClick={saveAndTestMysql}
            disabled={!mysqlHost || !mysqlUser || mysqlTest === "testing"}
          >
            {mysqlTest === "testing" ? "Verbinde…" : `${t("connections.testConnection")} & ${t("connections.save")}`}
          </Button>
          <StatusIcon state={mysqlTest} />
        </div>
        {mysqlError && <p className="text-sm text-destructive">{mysqlError}</p>}
      </section>
    </div>
  );
}

function Field({
  label,
  value,
  onChange,
  className,
}: {
  label: string;
  value: string;
  onChange: (value: string) => void;
  className?: string;
}) {
  return (
    <div className={`space-y-1 ${className ?? ""}`}>
      <label className="text-xs font-medium text-muted-foreground">{label}</label>
      <input
        value={value}
        onChange={(e) => onChange(e.target.value)}
        className="w-full rounded-md border border-border bg-background px-2 py-1 text-sm"
      />
    </div>
  );
}

function StatusIcon({ state }: { state: TestState }) {
  if (state === "ok") return <CheckCircle2 className="size-4 text-green-600" />;
  if (state === "error") return <XCircle className="size-4 text-destructive" />;
  return null;
}
