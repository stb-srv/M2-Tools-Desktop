import { Activity, DatabaseBackup, Map as MapIcon, PackageOpen, type LucideIcon } from "lucide-react";
import { SettingsCard, TextSetting, type TextSettingDef } from "./shared";

// Alle Pfade auf dem Gameserver, nach Einsatzzweck gruppiert. Jedes
// Server-Profil hat eigene Werte (siehe PER_SERVER_KEYS im Backend), ein
// Profilwechsel lädt die Felder neu.
const GROUPS: { title: string; description: string; icon: LucideIcon; fields: TextSettingDef[] }[] = [
  {
    title: "Drop-Dateien",
    icon: PackageOpen,
    description: "Werden per SFTP geladen und gespeichert, mit Backup vor jedem Überschreiben.",
    fields: [
      {
        key: "mob_drop_file_path",
        label: "mob_drop_item.txt",
        defaultValue: "/usr/home/game/share/mob_drop_item.txt",
        hint: "Mob Drop Editor und Drop-Generator.",
      },
      {
        key: "common_drop_file_path",
        label: "common_drop_item.txt",
        defaultValue: "/usr/home/game/share/common_drop_item.txt",
        hint: "Level-Bracket-Drops pro Mob-Rang (Drop-Generator).",
      },
      {
        key: "etc_drop_file_path",
        label: "etc_drop_item.txt",
        defaultValue: "/usr/home/game/share/etc_drop_item.txt",
        hint: "Item→Prozent-Tabelle, unabhängig vom Mob (Drop-Generator).",
      },
      {
        key: "drop_item_group_file_path",
        label: "drop_item_group.txt",
        defaultValue: "/usr/home/game/share/drop_item_group.txt",
        hint: "Unabhängig würfelnde Item-Pools pro Mob (Drop-Generator).",
      },
      {
        key: "special_item_group_file_path",
        label: "special_item_group.txt",
        defaultValue: "/usr/home/game/share/special_item_group.txt",
        hint: "Kisten-Editor und Drop-Generator.",
      },
      {
        key: "cube_file_path",
        label: "cube.txt",
        defaultValue: "/usr/home/game/share/cube.txt",
        hint: "Cube-Editor.",
      },
    ],
  },
  {
    title: "Quests & Welt",
    icon: MapIcon,
    description: "Quest Builder, Regen-Datei-Editor und Locale-Verwaltung.",
    fields: [
      {
        key: "quest_dir",
        label: "Quest-Ordner",
        defaultValue: "/usr/home/game/share/quest",
        hint: (
          <>
            Enthält <code>quest_list</code> und die Quest-Unterordner. „Quests reloaden" kompiliert
            ihn über <code>make.py</code> neu.
          </>
        ),
      },
      {
        key: "regen_base_dir",
        label: "Basisordner für Regen-Dateien",
        defaultValue: "/usr/home/game/share",
        hint: (
          <>
            Die Regen-Pfade der Dungeon-Etagen sind relativ dazu, z.B.{" "}
            <code>data/dungeon/dt_short/deviltower3_regen.txt</code>.
          </>
        ),
      },
      {
        key: "locale_file_path",
        label: "translate.lua",
        defaultValue: "/usr/home/game/share/translate.lua",
        hint: "Textbausteine für Quest-Dialoge (Locale-Verwaltung).",
      },
    ],
  },
  {
    title: "Server-Überwachung",
    icon: Activity,
    description: "Was die Startseite als Prozesse und Festplattenbelegung anzeigt.",
    fields: [
      {
        key: "server_process_names",
        label: "Prozessnamen",
        defaultValue: "game,db",
        placeholder: "game,db",
        allowEmpty: true,
        hint: "Kommagetrennt, Teilstring-Vergleich. Leer lassen zeigt alle laufenden Prozesse.",
      },
      {
        key: "server_disk_path",
        label: "Pfad für die Festplattenbelegung",
        defaultValue: "/usr/home/game",
        hint: (
          <>
            Welcher Mountpunkt (<code>df -Pk</code>) angezeigt wird, z.B. die Partition mit den
            Server-Daten.
          </>
        ),
      },
    ],
  },
  {
    title: "Datenbank-Backups",
    icon: DatabaseBackup,
    description: "Für die Seite Backups → Datenbank.",
    fields: [
      {
        key: "db_backup_dir",
        label: "Backup-Ordner auf dem Server",
        defaultValue: "/usr/home/game/m2manager_db_backups",
        hint: (
          <>
            Hier legt <code>mysqldump</code> die Sicherungen ab.
          </>
        ),
      },
      {
        key: "db_backup_databases",
        label: "Zu sichernde Datenbanken",
        defaultValue: "account common player log",
        hint: (
          <>
            Leerzeichen-getrennt, wird an <code>mysqldump --databases</code> übergeben.
          </>
        ),
      },
    ],
  },
];

export function ServerPathsTab() {
  return (
    <div className="space-y-4">
      {GROUPS.map((group) => (
        <SettingsCard key={group.title} title={group.title} description={group.description} icon={group.icon}>
          {group.fields.map((field) => (
            <TextSetting key={field.key} def={field} />
          ))}
        </SettingsCard>
      ))}
    </div>
  );
}
