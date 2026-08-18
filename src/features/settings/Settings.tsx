import { useEffect, useState } from "react";
import { useTranslation } from "react-i18next";
import { invoke } from "@tauri-apps/api/core";
import { open } from "@tauri-apps/plugin-dialog";
import { getVersion } from "@tauri-apps/api/app";
import { check, type Update } from "@tauri-apps/plugin-updater";
import { relaunch } from "@tauri-apps/plugin-process";
import { useThemeStore, type Theme } from "@/store/theme";
import { useUpdateStore } from "@/store/updateStore";
import { Button } from "@/components/ui/button";
import { PasswordInput } from "@/components/ui/password-input";
import { cn } from "@/lib/utils";
import { FolderOpen, X, CheckCircle2, XCircle, HelpCircle } from "lucide-react";
import { openManual } from "@/lib/manual";

const THEMES: Theme[] = ["light", "dark", "system"];
const LANGUAGES = [
  { code: "de", label: "Deutsch" },
  { code: "en", label: "English" },
];

type TestState = "idle" | "testing" | "ok" | "error";
type SshAuthMode = "password" | "key";

type SettingsTab = "general" | "updates" | "server" | "notifications" | "client";
const SETTINGS_TABS: { id: SettingsTab; label: string }[] = [
  { id: "general", label: "Allgemein" },
  { id: "updates", label: "Updates" },
  { id: "server", label: "Server" },
  { id: "notifications", label: "Benachrichtigungen" },
  { id: "client", label: "Client" },
];

export function Settings() {
  const { t, i18n } = useTranslation();
  const { theme, setTheme } = useThemeStore();
  const [tab, setTab] = useState<SettingsTab>("general");

  const [clientPath, setClientPath] = useState("");
  const [binarySrcPath, setBinarySrcPath] = useState("");
  const [npclistPath, setNpclistPath] = useState("");
  const [eterpackPath, setEterpackPath] = useState("");
  const [mysql2protoDir, setMysql2protoDir] = useState("");
  const [vnumRangeStart, setVnumRangeStart] = useState("500000");
  const [mobDropFilePath, setMobDropFilePath] = useState("");
  const [commonDropFilePath, setCommonDropFilePath] = useState("");
  const [etcDropFilePath, setEtcDropFilePath] = useState("");
  const [dropItemGroupFilePath, setDropItemGroupFilePath] = useState("");
  const [specialItemGroupFilePath, setSpecialItemGroupFilePath] = useState("");
  const [cubeFilePath, setCubeFilePath] = useState("");
  const [questDir, setQuestDir] = useState("");
  const [regenBaseDir, setRegenBaseDir] = useState("");
  const [localeFilePath, setLocaleFilePath] = useState("");
  const [serverProcessNames, setServerProcessNames] = useState("");
  const [serverDiskPath, setServerDiskPath] = useState("");
  const [dbBackupDir, setDbBackupDir] = useState("");
  const [dbBackupDatabases, setDbBackupDatabases] = useState("");
  const [webhookUrl, setWebhookUrl] = useState("");
  const [webhookTestState, setWebhookTestState] = useState<TestState>("idle");
  const [webhookTestError, setWebhookTestError] = useState<string | null>(null);
  const [saved, setSaved] = useState<string | null>(null);

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
    invoke<string | null>("get_setting", { key: "client_path" })
      .then((v) => setClientPath(v ?? ""))
      .catch(() => {});
    invoke<string | null>("get_setting", { key: "binary_src_path" })
      .then((v) => setBinarySrcPath(v ?? ""))
      .catch(() => {});
    invoke<string | null>("get_setting", { key: "npclist_path" })
      .then((v) => setNpclistPath(v ?? ""))
      .catch(() => {});
    invoke<string | null>("get_setting", { key: "eterpack_tool_path" })
      .then((v) => setEterpackPath(v ?? ""))
      .catch(() => {});
    invoke<string | null>("get_setting", { key: "mysql2proto_dir" })
      .then((v) => setMysql2protoDir(v ?? ""))
      .catch(() => {});
    invoke<string | null>("get_setting", { key: "item_vnum_range_start" })
      .then((v) => setVnumRangeStart(v ?? "500000"))
      .catch(() => {});
    invoke<string | null>("get_setting", { key: "mob_drop_file_path" })
      .then((v) => setMobDropFilePath(v ?? "/usr/home/game/share/mob_drop_item.txt"))
      .catch(() => {});
    invoke<string | null>("get_setting", { key: "common_drop_file_path" })
      .then((v) => setCommonDropFilePath(v ?? "/usr/home/game/share/common_drop_item.txt"))
      .catch(() => {});
    invoke<string | null>("get_setting", { key: "etc_drop_file_path" })
      .then((v) => setEtcDropFilePath(v ?? "/usr/home/game/share/etc_drop_item.txt"))
      .catch(() => {});
    invoke<string | null>("get_setting", { key: "drop_item_group_file_path" })
      .then((v) => setDropItemGroupFilePath(v ?? "/usr/home/game/share/drop_item_group.txt"))
      .catch(() => {});
    invoke<string | null>("get_setting", { key: "special_item_group_file_path" })
      .then((v) => setSpecialItemGroupFilePath(v ?? "/usr/home/game/share/special_item_group.txt"))
      .catch(() => {});
    invoke<string | null>("get_setting", { key: "cube_file_path" })
      .then((v) => setCubeFilePath(v ?? "/usr/home/game/share/cube.txt"))
      .catch(() => {});
    invoke<string | null>("get_setting", { key: "quest_dir" })
      .then((v) => setQuestDir(v ?? "/usr/home/game/share/quest"))
      .catch(() => {});
    invoke<string | null>("get_setting", { key: "regen_base_dir" })
      .then((v) => setRegenBaseDir(v ?? "/usr/home/game/share"))
      .catch(() => {});
    invoke<string | null>("get_setting", { key: "locale_file_path" })
      .then((v) => setLocaleFilePath(v ?? "/usr/home/game/share/translate.lua"))
      .catch(() => {});
    invoke<string | null>("get_setting", { key: "server_process_names" })
      .then((v) => setServerProcessNames(v ?? "game,db"))
      .catch(() => {});
    invoke<string | null>("get_setting", { key: "server_disk_path" })
      .then((v) => setServerDiskPath(v ?? "/usr/home/game"))
      .catch(() => {});
    invoke<string | null>("get_setting", { key: "db_backup_dir" })
      .then((v) => setDbBackupDir(v ?? "/usr/home/game/m2manager_db_backups"))
      .catch(() => {});
    invoke<string | null>("get_setting", { key: "db_backup_databases" })
      .then((v) => setDbBackupDatabases(v ?? "account common player log website"))
      .catch(() => {});
    invoke<string | null>("get_setting", { key: "webhook_url" })
      .then((v) => setWebhookUrl(v ?? ""))
      .catch(() => {});
  }, []);

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

  async function save(key: string, value: string, label: string) {
    await invoke("set_setting", { key, value });
    setSaved(label);
    setTimeout(() => setSaved(null), 2000);
  }

  async function pickClientPath() {
    const selected = await open({ directory: true, title: "Metin2-Client-Ordner auswählen" });
    if (typeof selected === "string") {
      setClientPath(selected);
      await save("client_path", selected, "Client-Pfad gespeichert");
    }
  }

  async function pickBinarySrcPath() {
    const selected = await open({ directory: true, title: "Lokalen Client-Quellcode-Ordner auswählen" });
    if (typeof selected === "string") {
      setBinarySrcPath(selected);
      await save("binary_src_path", selected, "Client-Quellcode-Pfad gespeichert");
    }
  }

  async function pickNpclist() {
    const selected = await open({
      multiple: false,
      title: "npclist.txt auswählen",
      filters: [{ name: "Textdatei", extensions: ["txt"] }],
    });
    if (typeof selected === "string") {
      setNpclistPath(selected);
      await save("npclist_path", selected, "NPC-Liste gespeichert");
    }
  }

  async function clearNpclist() {
    setNpclistPath("");
    await save("npclist_path", "", "NPC-Liste zurückgesetzt");
  }

  async function pickEterpack() {
    const selected = await open({
      multiple: false,
      title: "EterPackConsoleLz4.exe auswählen",
      filters: [{ name: "Programm", extensions: ["exe"] }],
    });
    if (typeof selected === "string") {
      setEterpackPath(selected);
      await save("eterpack_tool_path", selected, "EterPackConsoleLz4-Pfad gespeichert");
    }
  }

  async function pickMysql2Proto() {
    const selected = await open({ directory: true, title: "Mysql2Proto-Ordner auswählen" });
    if (typeof selected === "string") {
      setMysql2protoDir(selected);
      await save("mysql2proto_dir", selected, "Mysql2Proto-Ordner gespeichert");
    }
  }

  async function saveVnumRangeStart(value: string) {
    setVnumRangeStart(value);
    if (/^\d+$/.test(value)) {
      await save("item_vnum_range_start", value, "VNUM-Bereich gespeichert");
    }
  }

  async function commitMobDropFilePath() {
    if (mobDropFilePath.trim()) {
      await save("mob_drop_file_path", mobDropFilePath.trim(), "Mob-Drop-Dateipfad gespeichert");
    }
  }

  async function commitCommonDropFilePath() {
    if (commonDropFilePath.trim()) {
      await save("common_drop_file_path", commonDropFilePath.trim(), "Common-Drop-Dateipfad gespeichert");
    }
  }

  async function commitEtcDropFilePath() {
    if (etcDropFilePath.trim()) {
      await save("etc_drop_file_path", etcDropFilePath.trim(), "Etc-Drop-Dateipfad gespeichert");
    }
  }

  async function commitDropItemGroupFilePath() {
    if (dropItemGroupFilePath.trim()) {
      await save("drop_item_group_file_path", dropItemGroupFilePath.trim(), "Zufalls-Gruppen-Dateipfad gespeichert");
    }
  }

  async function commitSpecialItemGroupFilePath() {
    if (specialItemGroupFilePath.trim()) {
      await save("special_item_group_file_path", specialItemGroupFilePath.trim(), "Item-Gruppen-Dateipfad gespeichert");
    }
  }

  async function commitCubeFilePath() {
    if (cubeFilePath.trim()) {
      await save("cube_file_path", cubeFilePath.trim(), "Cube-Dateipfad gespeichert");
    }
  }

  async function commitQuestDir() {
    if (questDir.trim()) {
      await save("quest_dir", questDir.trim(), "Quest-Ordner gespeichert");
    }
  }

  async function commitRegenBaseDir() {
    if (regenBaseDir.trim()) {
      await save("regen_base_dir", regenBaseDir.trim(), "Regen-Basisordner gespeichert");
    }
  }

  async function commitLocaleFilePath() {
    if (localeFilePath.trim()) {
      await save("locale_file_path", localeFilePath.trim(), "Locale-Dateipfad gespeichert");
    }
  }

  async function commitServerProcessNames() {
    await save(
      "server_process_names",
      serverProcessNames.trim(),
      "Prozessnamen gespeichert",
    );
  }

  async function commitServerDiskPath() {
    if (serverDiskPath.trim()) {
      await save("server_disk_path", serverDiskPath.trim(), "Festplatten-Pfad gespeichert");
    }
  }

  async function commitDbBackupDir() {
    if (dbBackupDir.trim()) {
      await save("db_backup_dir", dbBackupDir.trim(), "Backup-Ordner gespeichert");
    }
  }

  async function commitDbBackupDatabases() {
    if (dbBackupDatabases.trim()) {
      await save(
        "db_backup_databases",
        dbBackupDatabases.trim(),
        "Datenbank-Liste gespeichert",
      );
    }
  }

  async function commitWebhookUrl() {
    await save("webhook_url", webhookUrl.trim(), "Webhook-URL gespeichert");
  }

  async function testWebhook() {
    setWebhookTestState("testing");
    setWebhookTestError(null);
    try {
      await commitWebhookUrl();
      await invoke("send_test_webhook");
      setWebhookTestState("ok");
    } catch (e) {
      setWebhookTestState("error");
      setWebhookTestError(String(e));
    }
  }

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

  const updateAvailable = useUpdateStore((s) => s.available);

  return (
    <div className="max-w-2xl space-y-6">
      <div className="flex items-center gap-2">
        <h1 className="text-2xl font-semibold">{t("settings.title")}</h1>
        <Button variant="ghost" size="icon-sm" title="Hilfe zu diesem Modul" onClick={() => openManual("settings")}>
          <HelpCircle className="size-4" />
        </Button>
      </div>

      <div className="flex flex-wrap gap-1 border-b border-border">
        {SETTINGS_TABS.map((tabDef) => (
          <button
            key={tabDef.id}
            onClick={() => setTab(tabDef.id)}
            className={cn(
              "flex items-center gap-1.5 rounded-t-md px-3 py-2 text-sm font-medium transition-colors",
              tab === tabDef.id
                ? "border-b-2 border-primary text-foreground"
                : "text-muted-foreground hover:text-foreground",
            )}
          >
            {tabDef.label}
            {tabDef.id === "updates" && updateAvailable && (
              <span className="size-1.5 shrink-0 rounded-full bg-blue-500" />
            )}
          </button>
        ))}
      </div>

      {/* ---------------- Allgemein ---------------- */}
      <div className={cn("space-y-4", tab !== "general" && "hidden")}>
        <h2 className="text-base font-semibold">Allgemein</h2>

        <section className="space-y-2">
          <h3 className="text-sm font-medium text-muted-foreground">{t("settings.theme")}</h3>
          <div className="flex gap-2">
            {THEMES.map((value) => (
              <Button
                key={value}
                variant={theme === value ? "default" : "outline"}
                onClick={() => setTheme(value)}
              >
                {t(`settings.${value}`)}
              </Button>
            ))}
          </div>
        </section>

        <section className="space-y-2">
          <h3 className="text-sm font-medium text-muted-foreground">{t("settings.language")}</h3>
          <div className="flex gap-2">
            {LANGUAGES.map((lang) => (
              <Button
                key={lang.code}
                variant={i18n.language === lang.code ? "default" : "outline"}
                onClick={() => i18n.changeLanguage(lang.code)}
                className={cn(i18n.language === lang.code && "font-semibold")}
              >
                {lang.label}
              </Button>
            ))}
          </div>
        </section>
      </div>

      {/* ---------------- Updates ---------------- */}
      <div className={cn(tab !== "updates" && "hidden")}>
        <UpdateSection />
      </div>

      {/* ---------------- Server ---------------- */}
      <div className={cn("space-y-4", tab !== "server" && "hidden")}>
        <h2 className="text-base font-semibold">Server</h2>

        <section className="space-y-2 rounded-lg border border-border bg-card p-4">
          <h3 className="font-medium">{t("connections.ssh")}</h3>
          <p className="text-xs text-muted-foreground">
            Für Server-Steuerung (Starten/Stoppen, Logs, Quest-Reload) und den Mob Drop Editor
            (SFTP).
          </p>
          <div className="grid grid-cols-3 gap-2">
            <ConnField
              label={t("connections.host")}
              value={sshHost}
              onChange={setSshHost}
              className="col-span-2"
            />
            <ConnField label={t("connections.port")} value={sshPort} onChange={setSshPort} />
          </div>
          <ConnField label={t("connections.username")} value={sshUser} onChange={setSshUser} />

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
              {sshTest === "testing"
                ? "Verbinde…"
                : `${t("connections.testConnection")} & ${t("connections.save")}`}
            </Button>
            <ConnStatusIcon state={sshTest} />
          </div>
          {sshError && <p className="text-sm text-destructive">{sshError}</p>}
        </section>

        <section className="space-y-2 rounded-lg border border-border bg-card p-4">
          <h3 className="font-medium">{t("connections.mysql")}</h3>
          <p className="text-xs text-muted-foreground">
            Für Shop-Editor, Item Editor, Datenbank-Explorer und Dashboard-Statistiken.
          </p>
          <div className="grid grid-cols-3 gap-2">
            <ConnField
              label={t("connections.host")}
              value={mysqlHost}
              onChange={setMysqlHost}
              className="col-span-2"
            />
            <ConnField label={t("connections.port")} value={mysqlPort} onChange={setMysqlPort} />
          </div>
          <ConnField label={t("connections.username")} value={mysqlUser} onChange={setMysqlUser} />
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
              {mysqlTest === "testing"
                ? "Verbinde…"
                : `${t("connections.testConnection")} & ${t("connections.save")}`}
            </Button>
            <ConnStatusIcon state={mysqlTest} />
          </div>
          {mysqlError && <p className="text-sm text-destructive">{mysqlError}</p>}
        </section>

        <section className="space-y-1">
          <div className="flex items-center gap-2">
            <Button variant="outline" onClick={pickMysql2Proto} className="shrink-0">
              <FolderOpen className="size-4" />
              Mysql2Proto-Ordner
            </Button>
            <span className="truncate text-sm text-muted-foreground">
              {mysql2protoDir || "Nicht gesetzt"}
            </span>
          </div>
          <p className="text-xs text-muted-foreground">
            Ordner mit <code>Mysql2Proto.exe</code> und <code>Mysql2Proto.json</code> — erzeugt aus
            der Datenbank eine neue <code>item_proto</code>-Datei für den Client.
          </p>
        </section>

        <section className="space-y-1">
          <label className="flex items-center gap-2 text-sm">
            VNUM-Bereich für neue Items (Start)
            <input
              value={vnumRangeStart}
              onChange={(e) => saveVnumRangeStart(e.target.value)}
              className="w-28 rounded-md border border-border bg-background px-2 py-1 text-sm"
            />
          </label>
          <p className="text-xs text-muted-foreground">
            Der Item Editor schlägt ab dieser vnum die nächste freie Nummer vor.
          </p>
        </section>

        <section className="space-y-1">
          <label className="flex items-center gap-2 text-sm">
            Pfad der Mob-Drop-Datei auf dem Server
            <input
              value={mobDropFilePath}
              onChange={(e) => setMobDropFilePath(e.target.value)}
              onBlur={commitMobDropFilePath}
              className="w-80 rounded-md border border-border bg-background px-2 py-1 text-sm"
            />
          </label>
          <p className="text-xs text-muted-foreground">
            Wird per SFTP über die obige SSH-Verbindung geladen/gespeichert (Mob Drop Editor / Drop-Generator).
          </p>
        </section>

        <section className="space-y-1">
          <label className="flex items-center gap-2 text-sm">
            Pfad der Common-Drop-Datei auf dem Server
            <input
              value={commonDropFilePath}
              onChange={(e) => setCommonDropFilePath(e.target.value)}
              onBlur={commitCommonDropFilePath}
              className="w-80 rounded-md border border-border bg-background px-2 py-1 text-sm"
            />
          </label>
          <p className="text-xs text-muted-foreground">
            Level-Bracket-Drops pro Mob-Rang, wird per SFTP geladen/gespeichert (Drop-Generator).
          </p>
        </section>

        <section className="space-y-1">
          <label className="flex items-center gap-2 text-sm">
            Pfad der Etc-Drop-Datei auf dem Server
            <input
              value={etcDropFilePath}
              onChange={(e) => setEtcDropFilePath(e.target.value)}
              onBlur={commitEtcDropFilePath}
              className="w-80 rounded-md border border-border bg-background px-2 py-1 text-sm"
            />
          </label>
          <p className="text-xs text-muted-foreground">
            Item→Prozent-Tabelle (mob-unabhängig), wird per SFTP geladen/gespeichert (Drop-Generator).
          </p>
        </section>

        <section className="space-y-1">
          <label className="flex items-center gap-2 text-sm">
            Pfad der Zufalls-Gruppen-Datei (drop_item_group.txt) auf dem Server
            <input
              value={dropItemGroupFilePath}
              onChange={(e) => setDropItemGroupFilePath(e.target.value)}
              onBlur={commitDropItemGroupFilePath}
              className="w-80 rounded-md border border-border bg-background px-2 py-1 text-sm"
            />
          </label>
          <p className="text-xs text-muted-foreground">
            Unabhängig würfelnde Item-Pools pro Mob, wird per SFTP geladen/gespeichert (Drop-Generator).
          </p>
        </section>

        <section className="space-y-1">
          <label className="flex items-center gap-2 text-sm">
            Pfad der Item-Gruppen-Datei (special_item_group.txt) auf dem Server
            <input
              value={specialItemGroupFilePath}
              onChange={(e) => setSpecialItemGroupFilePath(e.target.value)}
              onBlur={commitSpecialItemGroupFilePath}
              className="w-80 rounded-md border border-border bg-background px-2 py-1 text-sm"
            />
          </label>
          <p className="text-xs text-muted-foreground">
            Wird per SFTP über die obige SSH-Verbindung geladen/gespeichert (Kisten-Editor / Drop-Generator).
          </p>
        </section>

        <section className="space-y-1">
          <label className="flex items-center gap-2 text-sm">
            Pfad der Cube-Datei (cube.txt) auf dem Server
            <input
              value={cubeFilePath}
              onChange={(e) => setCubeFilePath(e.target.value)}
              onBlur={commitCubeFilePath}
              className="w-80 rounded-md border border-border bg-background px-2 py-1 text-sm"
            />
          </label>
          <p className="text-xs text-muted-foreground">
            Wird per SFTP über die obige SSH-Verbindung geladen/gespeichert (Cube-Editor).
          </p>
        </section>

        <section className="space-y-1">
          <label className="flex items-center gap-2 text-sm">
            Quest-Ordner auf dem Server
            <input
              value={questDir}
              onChange={(e) => setQuestDir(e.target.value)}
              onBlur={commitQuestDir}
              className="w-80 rounded-md border border-border bg-background px-2 py-1 text-sm"
            />
          </label>
          <p className="text-xs text-muted-foreground">
            Enthält <code>quest_list</code> und die Quest-Unterordner (Quest Builder). „Quests
            reloaden" kompiliert diesen Ordner über <code>make.py</code> auf dem Server neu.
          </p>
        </section>

        <section className="space-y-1">
          <label className="flex items-center gap-2 text-sm">
            Basisordner für Regen-Dateien
            <input
              value={regenBaseDir}
              onChange={(e) => setRegenBaseDir(e.target.value)}
              onBlur={commitRegenBaseDir}
              className="w-80 rounded-md border border-border bg-background px-2 py-1 text-sm"
            />
          </label>
          <p className="text-xs text-muted-foreground">
            Ausgangsordner für den Regen-Datei-Editor (Monster-Spawns) - die Pfade in den
            Dungeon-Etagen des Quest Builders sind relativ dazu, z.B.{" "}
            <code>data/dungeon/dt_short/deviltower3_regen.txt</code>.
          </p>
        </section>

        <section className="space-y-1">
          <label className="flex items-center gap-2 text-sm">
            Pfad der Locale-Datei (translate.lua)
            <input
              value={localeFilePath}
              onChange={(e) => setLocaleFilePath(e.target.value)}
              onBlur={commitLocaleFilePath}
              className="w-80 rounded-md border border-border bg-background px-2 py-1 text-sm"
            />
          </label>
          <p className="text-xs text-muted-foreground">
            Wird von der Locale-String-Verwaltung genutzt (Textbausteine für Quest-Dialoge).
          </p>
        </section>

        <section className="space-y-1">
          <label className="flex items-center gap-2 text-sm">
            Prozessnamen für Ressourcen-Monitoring
            <input
              value={serverProcessNames}
              onChange={(e) => setServerProcessNames(e.target.value)}
              onBlur={commitServerProcessNames}
              placeholder="game,db"
              className="w-48 rounded-md border border-border bg-background px-2 py-1 text-sm"
            />
          </label>
          <p className="text-xs text-muted-foreground">
            Kommagetrennt, Teilstring-Vergleich (Groß-/Kleinschreibung egal). Nicht gegen einen
            echten Server verifiziert, welche Prozessnamen dieser Core konkret nutzt - bei Bedarf
            anpassen. Leer lassen zeigt alle laufenden Prozesse im Dashboard.
          </p>
        </section>

        <section className="space-y-1">
          <label className="flex items-center gap-2 text-sm">
            Pfad für Festplatten-Auslastung im Dashboard
            <input
              value={serverDiskPath}
              onChange={(e) => setServerDiskPath(e.target.value)}
              onBlur={commitServerDiskPath}
              className="w-80 rounded-md border border-border bg-background px-2 py-1 text-sm"
            />
          </label>
          <p className="text-xs text-muted-foreground">
            Welches Dateisystem/welcher Mountpunkt (<code>df -Pk</code>) im Dashboard als
            Festplattenbelegung angezeigt wird - z.B. die Partition, auf der die Server-Daten
            liegen.
          </p>
        </section>

        <section className="space-y-1">
          <label className="flex items-center gap-2 text-sm">
            Ordner für Datenbank-Backups (auf dem Server)
            <input
              value={dbBackupDir}
              onChange={(e) => setDbBackupDir(e.target.value)}
              onBlur={commitDbBackupDir}
              className="w-80 rounded-md border border-border bg-background px-2 py-1 text-sm"
            />
          </label>
          <p className="text-xs text-muted-foreground">
            Wo <code>mysqldump</code>-Backups abgelegt werden (Datenbank-Backups-Seite).
          </p>
        </section>

        <section className="space-y-1">
          <label className="flex items-center gap-2 text-sm">
            Zu sichernde Datenbanken
            <input
              value={dbBackupDatabases}
              onChange={(e) => setDbBackupDatabases(e.target.value)}
              onBlur={commitDbBackupDatabases}
              className="w-80 rounded-md border border-border bg-background px-2 py-1 text-sm"
            />
          </label>
          <p className="text-xs text-muted-foreground">
            Leerzeichen-getrennt, wird an <code>mysqldump --databases</code> übergeben.
          </p>
        </section>
      </div>

      {/* ---------------- Benachrichtigungen ---------------- */}
      <div className={cn("space-y-4", tab !== "notifications" && "hidden")}>
        <h2 className="text-base font-semibold">Benachrichtigungen</h2>
        <section className="space-y-1">
          <label className="flex items-center gap-2 text-sm">
            Webhook-URL (Discord o.ä.)
            <input
              value={webhookUrl}
              onChange={(e) => setWebhookUrl(e.target.value)}
              onBlur={commitWebhookUrl}
              placeholder="https://discord.com/api/webhooks/…"
              className="w-96 rounded-md border border-border bg-background px-2 py-1 text-sm"
            />
          </label>
          <p className="text-xs text-muted-foreground">
            Wird bei fehlgeschlagenen Datenbank-Backups und (solange die App geöffnet ist) beim
            Verschwinden der überwachten Server-Prozesse benachrichtigt. Leer lassen deaktiviert
            Benachrichtigungen.
          </p>
          <div className="flex items-center gap-2 pt-1">
            <Button
              variant="outline"
              onClick={testWebhook}
              disabled={!webhookUrl.trim() || webhookTestState === "testing"}
            >
              {webhookTestState === "testing" ? "Sende…" : "Testnachricht senden"}
            </Button>
            <ConnStatusIcon state={webhookTestState} />
          </div>
          {webhookTestError && <p className="text-sm text-destructive">{webhookTestError}</p>}
        </section>
      </div>

      {/* ---------------- Client ---------------- */}
      <div className={cn("space-y-4", tab !== "client" && "hidden")}>
        <h2 className="text-base font-semibold">Client</h2>

        <section className="space-y-1">
          <div className="flex items-center gap-2">
            <Button variant="outline" onClick={pickClientPath} className="shrink-0">
              <FolderOpen className="size-4" />
              {t("settings.clientPath")}
            </Button>
            <span className="truncate text-sm text-muted-foreground">
              {clientPath || "Nicht gesetzt"}
            </span>
          </div>
          <p className="text-xs text-muted-foreground">
            Ordner des Metin2-Clients (enthält <code>granny2.dll</code>) – wird für 3D-Modelle und
            Item-Icons benötigt.
          </p>
        </section>

        <section className="space-y-1">
          <div className="flex items-center gap-2">
            <Button variant="outline" onClick={pickBinarySrcPath} className="shrink-0">
              <FolderOpen className="size-4" />
              Client-Quellcode-Ordner
            </Button>
            <span className="truncate text-sm text-muted-foreground">
              {binarySrcPath || "Nicht gesetzt"}
            </span>
          </div>
          <p className="text-xs text-muted-foreground">
            Lokaler Checkout des Client-Quellcodes (z.B. <code>binary-src</code>) – nur für den
            System-Installer nötig, wenn ein System Client-C++-Dateien patcht. Das Kompilieren des
            Clients bleibt danach manuell (kein Client-Build-Werkzeug in M2Manager).
          </p>
        </section>

        <section className="space-y-1">
          <div className="flex items-center gap-2">
            <Button variant="outline" onClick={pickNpclist} className="shrink-0">
              <FolderOpen className="size-4" />
              NPC-Liste (npclist.txt)
            </Button>
            <span className="truncate text-sm text-muted-foreground">
              {npclistPath || "Automatisch suchen"}
            </span>
            {npclistPath && (
              <Button variant="ghost" size="icon-sm" onClick={clearNpclist}>
                <X className="size-3.5" />
              </Button>
            )}
          </div>
          <p className="text-xs text-muted-foreground">
            Ordnet NPC-VNUMs den Modell-Ordnern zu. Normalerweise nicht nötig: M2Manager sucht
            zuerst unter <code>root\npclist.txt</code> und durchsucht sonst den kompletten
            Client-Ordner. Nur setzen, falls die Datei umbenannt wurde oder außerhalb des Clients
            liegt.
          </p>
        </section>

        <section className="space-y-1">
          <div className="flex items-center gap-2">
            <Button variant="outline" onClick={pickEterpack} className="shrink-0">
              <FolderOpen className="size-4" />
              EterPackConsoleLz4.exe
            </Button>
            <span className="truncate text-sm text-muted-foreground">
              {eterpackPath || "Nicht gesetzt"}
            </span>
          </div>
          <p className="text-xs text-muted-foreground">
            Packt <code>icon.epk</code> neu, nachdem ein Item-Icon hinzugefügt wurde. Liegt
            normalerweise unter <code>Client\pack\EterPackConsoleLz4.exe</code>.
          </p>
        </section>
      </div>

      {saved && <p className="text-xs text-green-600">{saved}</p>}
    </div>
  );
}

function ConnField({
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

function ConnStatusIcon({ state }: { state: TestState }) {
  if (state === "ok") return <CheckCircle2 className="size-4 text-green-600" />;
  if (state === "error") return <XCircle className="size-4 text-destructive" />;
  return null;
}

type UpdateStatus = "idle" | "checking" | "upToDate" | "error" | "available";

// Nutzt die Plugin-API direkt (check()/Update.downloadAndInstall()) statt
// eigener Tauri-Commands - der Authorization-Header fürs private Repo wird
// bereits einmalig beim Plugin-Setup gesetzt (siehe build_updater_plugin()
// in lib.rs), gilt automatisch für jede Anfrage dieses Plugins.
function UpdateSection() {
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
