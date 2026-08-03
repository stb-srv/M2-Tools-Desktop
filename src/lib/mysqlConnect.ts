import { invoke } from "@tauri-apps/api/core";

/**
 * The MySQL pool lives only in the Rust process's memory (`AppState`), so it
 * is gone after every app restart even though host/user/password are
 * persisted (SQLite settings + Windows Credential Manager). Without this,
 * every DB-backed page silently shows "not connected" until the user
 * revisits Connections and re-runs "Verbindung testen & Speichern".
 * Called once at app startup (see App.tsx) so pages never have to know
 * whether this is a fresh process or not.
 */
export async function autoConnectMysql(): Promise<boolean> {
  try {
    const already = await invoke<boolean>("is_mysql_connected");
    if (already) return true;

    const [host, port, username] = await Promise.all([
      invoke<string | null>("get_setting", { key: "mysql_host" }),
      invoke<string | null>("get_setting", { key: "mysql_port" }),
      invoke<string | null>("get_setting", { key: "mysql_username" }),
    ]);
    if (!host || !username) return false;

    const password = await invoke<string>("get_credential", {
      account: "mysql_password",
    }).catch(() => null);
    if (!password) return false;

    await invoke("connect_mysql", {
      config: { host, port: Number(port ?? 3306), username, database: null },
      password,
    });
    return true;
  } catch {
    return false;
  }
}
