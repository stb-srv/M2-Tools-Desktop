import { invoke } from "@tauri-apps/api/core";

// Einheitliche Lese-/Schreib-/Test-Logik für die SSH- und MySQL-Verbindung.
// Vorher stand derselbe Code dreifach im Setup-Assistenten, in den
// Einstellungen und (neu) im "Neues Profil"-Dialog. Alle Werte landen in den
// kanonischen Settings-Keys + Windows Credential Manager, die jeder SSH-/DB-
// Befehl im Backend bei jedem Aufruf frisch liest.

export type SshAuthMode = "password" | "key";

export interface SshValues {
  host: string;
  port: string;
  username: string;
  authMode: SshAuthMode;
  password: string;
  keyPath: string;
  keyPassphrase: string;
}

export interface MysqlValues {
  host: string;
  port: string;
  username: string;
  password: string;
}

export const EMPTY_SSH: SshValues = {
  host: "",
  port: "22",
  username: "",
  authMode: "password",
  password: "",
  keyPath: "",
  keyPassphrase: "",
};

export const EMPTY_MYSQL: MysqlValues = { host: "", port: "3306", username: "", password: "" };

function sshAuth(v: SshValues) {
  return v.authMode === "password"
    ? { type: "password", password: v.password }
    : { type: "private_key", path: v.keyPath, passphrase: v.keyPassphrase || null };
}

function mysqlConfig(v: MysqlValues) {
  return { host: v.host, port: Number(v.port || "3306"), username: v.username, database: null };
}

export function sshComplete(v: SshValues): boolean {
  return !!v.host.trim() && !!v.username.trim() && (v.authMode === "password" || !!v.keyPath);
}

export function mysqlComplete(v: MysqlValues): boolean {
  return !!v.host.trim() && !!v.username.trim();
}

/** Liest die aktuell aktive Verbindung (inkl. gespeicherter Passwörter). */
export async function loadStoredConnection(): Promise<{ ssh: SshValues; mysql: MysqlValues }> {
  const get = (key: string) => invoke<string | null>("get_setting", { key }).catch(() => null);
  const secret = (account: string) => invoke<string>("get_credential", { account }).catch(() => null);

  const ssh: SshValues = {
    host: (await get("ssh_host")) ?? "",
    port: (await get("ssh_port")) ?? "22",
    username: (await get("ssh_username")) ?? "",
    authMode: ((await get("ssh_auth_mode")) as SshAuthMode | null) ?? "password",
    password: (await secret("ssh_password")) ?? "",
    keyPath: (await get("ssh_key_path")) ?? "",
    keyPassphrase: (await secret("ssh_key_passphrase")) ?? "",
  };
  const mysql: MysqlValues = {
    host: (await get("mysql_host")) ?? "",
    port: (await get("mysql_port")) ?? "3306",
    username: (await get("mysql_username")) ?? "",
    password: (await secret("mysql_password")) ?? "",
  };
  return { ssh, mysql };
}

export async function testSsh(v: SshValues): Promise<void> {
  await invoke("test_ssh_connection", {
    config: { host: v.host, port: Number(v.port || "22"), username: v.username },
    auth: sshAuth(v),
  });
}

export async function testMysql(v: MysqlValues): Promise<void> {
  await invoke("test_mysql_connection", { config: mysqlConfig(v), password: v.password });
}

/** Schreibt die SSH-Werte als aktive Verbindung (ohne Test). */
export async function saveSsh(v: SshValues): Promise<void> {
  await invoke("set_setting", { key: "ssh_host", value: v.host.trim() });
  await invoke("set_setting", { key: "ssh_port", value: v.port.trim() || "22" });
  await invoke("set_setting", { key: "ssh_username", value: v.username.trim() });
  await invoke("set_setting", { key: "ssh_auth_mode", value: v.authMode });
  if (v.authMode === "password") {
    await invoke("store_credential", { account: "ssh_password", secret: v.password });
  } else {
    await invoke("set_setting", { key: "ssh_key_path", value: v.keyPath });
    if (v.keyPassphrase) {
      await invoke("store_credential", { account: "ssh_key_passphrase", secret: v.keyPassphrase });
    } else {
      // Sonst bliebe die Passphrase eines vorher aktiven Servers hängen und
      // würde ins neue Profil mitkopiert.
      await invoke("delete_credential", { account: "ssh_key_passphrase" }).catch(() => {});
    }
  }
}

/** Schreibt die MySQL-Werte als aktive Verbindung (ohne Test). */
export async function saveMysql(v: MysqlValues): Promise<void> {
  await invoke("set_setting", { key: "mysql_host", value: v.host.trim() });
  await invoke("set_setting", { key: "mysql_port", value: v.port.trim() || "3306" });
  await invoke("set_setting", { key: "mysql_username", value: v.username.trim() });
  await invoke("store_credential", { account: "mysql_password", secret: v.password });
}

/** Baut den MySQL-Pool im Backend neu auf (der bleibt sonst am alten Server hängen). */
export async function connectMysql(v: MysqlValues): Promise<void> {
  await invoke("connect_mysql", { config: mysqlConfig(v), password: v.password });
}
