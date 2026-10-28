import { create } from "zustand";
import { invoke } from "@tauri-apps/api/core";
import {
  connectMysql,
  loadStoredConnection,
  saveMysql,
  saveSsh,
  type MysqlValues,
  type SshValues,
} from "@/lib/connection";

export interface ConnectionProfile {
  id: number;
  name: string;
  ssh_host: string;
  ssh_port: string;
  ssh_username: string;
  ssh_auth_mode: "password" | "key";
  ssh_key_path: string | null;
  mysql_host: string;
  mysql_port: string;
  mysql_username: string;
}

// Gemeinsamer Stand der Server-Profile für Startseite und Einstellungen -
// beide sind gleichzeitig gemountet, ein Wechsel auf der einen Seite muss auf
// der anderen sofort sichtbar sein. `revision` zählt bei jedem Wechsel der
// aktiven Verbindung hoch, damit z.B. das Dashboard seinen Status neu prüft.
interface ConnectionProfilesState {
  profiles: ConnectionProfile[];
  activeId: number | null;
  loaded: boolean;
  revision: number;
  reload: () => Promise<void>;
  activate: (id: number) => Promise<void>;
  create: (name: string, ssh: SshValues, mysql: MysqlValues) => Promise<void>;
  remove: (id: number) => Promise<void>;
  syncActive: () => Promise<void>;
}

export const useConnectionProfiles = create<ConnectionProfilesState>((set, get) => ({
  profiles: [],
  activeId: null,
  loaded: false,
  revision: 0,

  reload: async () => {
    const [profiles, activeId] = await Promise.all([
      invoke<ConnectionProfile[]>("list_connection_profiles"),
      invoke<number | null>("get_active_connection_profile_id"),
    ]);
    set({ profiles, activeId, loaded: true });
  },

  activate: async (id) => {
    await invoke("activate_connection_profile", { id });
    // SSH liest seine Einstellungen bei jedem Befehl frisch, der MySQL-Pool
    // dagegen bleibt bis zum expliziten Neuverbinden am alten Server.
    const { mysql } = await loadStoredConnection();
    if (mysql.host && mysql.username && mysql.password) {
      await connectMysql(mysql).catch(() => {});
    }
    await get().reload();
    set((s) => ({ revision: s.revision + 1 }));
  },

  create: async (name, ssh, mysql) => {
    await saveSsh(ssh);
    await saveMysql(mysql);
    await invoke("save_connection_profile", { name });
    // Verbindungsfehler hier nicht hart abbrechen lassen: das Profil ist
    // gespeichert, die Startseite zeigt danach den echten SSH-/MySQL-Status.
    await connectMysql(mysql).catch(() => {});
    await get().reload();
    set((s) => ({ revision: s.revision + 1 }));
  },

  remove: async (id) => {
    await invoke("delete_connection_profile", { id });
    await get().reload();
  },

  // Nach einer Änderung an SSH/MySQL in den Einstellungen das aktive Profil
  // mitziehen, sonst würde ein späteres "Aktivieren" die alten Werte zurückholen.
  syncActive: async () => {
    const { profiles, activeId } = get();
    const active = profiles.find((p) => p.id === activeId);
    if (active) await invoke("save_connection_profile", { name: active.name });
    await get().reload();
    set((s) => ({ revision: s.revision + 1 }));
  },
}));
