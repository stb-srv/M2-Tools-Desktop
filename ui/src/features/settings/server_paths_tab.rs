//! Alle Pfade auf dem Gameserver, nach Einsatzzweck gruppiert. Jedes
//! Server-Profil hat eigene Werte (siehe PER_SERVER_KEYS im Backend), ein
//! Profilwechsel lädt die Felder neu.

use super::shared::{SettingsCard, TextSetting, TextSettingDef};
use leptos::prelude::*;

struct Group {
    title: &'static str,
    description: &'static str,
    icon: &'static str,
    fields: &'static [TextSettingDef],
}

const fn f(key: &'static str, label: &'static str, default_value: &'static str, hint: &'static str) -> TextSettingDef {
    TextSettingDef::new(key, label, default_value).hint(hint)
}

const GROUPS: &[Group] = &[
    Group {
        title: "Drop-Dateien",
        icon: "PackageOpen",
        description: "Werden per SFTP geladen und gespeichert, mit Backup vor jedem Überschreiben.",
        fields: &[
            f("mob_drop_file_path", "mob_drop_item.txt", "/usr/home/game/share/mob_drop_item.txt", "Mob Drop Editor und Drop-Generator."),
            f(
                "common_drop_file_path",
                "common_drop_item.txt",
                "/usr/home/game/share/common_drop_item.txt",
                "Level-Bracket-Drops pro Mob-Rang (Drop-Generator).",
            ),
            f(
                "etc_drop_file_path",
                "etc_drop_item.txt",
                "/usr/home/game/share/etc_drop_item.txt",
                "Item→Prozent-Tabelle, unabhängig vom Mob (Drop-Generator).",
            ),
            f(
                "drop_item_group_file_path",
                "drop_item_group.txt",
                "/usr/home/game/share/drop_item_group.txt",
                "Unabhängig würfelnde Item-Pools pro Mob (Drop-Generator).",
            ),
            f(
                "special_item_group_file_path",
                "special_item_group.txt",
                "/usr/home/game/share/special_item_group.txt",
                "Kisten-Editor und Drop-Generator.",
            ),
            f("cube_file_path", "cube.txt", "/usr/home/game/share/cube.txt", "Cube-Editor."),
        ],
    },
    Group {
        title: "Quests & Welt",
        icon: "Map",
        description: "Quest Builder, Regen-Datei-Editor und Locale-Verwaltung.",
        fields: &[
            f(
                "quest_dir",
                "Quest-Ordner",
                "/usr/home/game/share/quest",
                "Enthält <code>quest_list</code> und die Quest-Unterordner. „Quests reloaden\" kompiliert ihn über <code>make.py</code> neu.",
            ),
            f(
                "regen_base_dir",
                "Basisordner für Regen-Dateien",
                "/usr/home/game/share",
                "Die Regen-Pfade der Dungeon-Etagen sind relativ dazu, z.B. <code>data/dungeon/dt_short/deviltower3_regen.txt</code>.",
            ),
            f(
                "locale_file_path",
                "translate.lua",
                "/usr/home/game/share/translate.lua",
                "Textbausteine für Quest-Dialoge (Locale-Verwaltung).",
            ),
        ],
    },
    Group {
        title: "Server-Überwachung",
        icon: "Activity",
        description: "Was die Startseite als Prozesse und Festplattenbelegung anzeigt.",
        fields: &[
            TextSettingDef {
                placeholder: Some("game,db"),
                allow_empty: true,
                ..f(
                    "server_process_names",
                    "Prozessnamen",
                    "game,db",
                    "Kommagetrennt, Teilstring-Vergleich. Leer lassen zeigt alle laufenden Prozesse.",
                )
            },
            f(
                "server_disk_path",
                "Pfad für die Festplattenbelegung",
                "/usr/home/game",
                "Welcher Mountpunkt (<code>df -Pk</code>) angezeigt wird, z.B. die Partition mit den Server-Daten.",
            ),
        ],
    },
    Group {
        title: "Datenbank-Backups",
        icon: "DatabaseBackup",
        description: "Für die Seite Backups → Datenbank.",
        fields: &[
            f(
                "db_backup_dir",
                "Backup-Ordner auf dem Server",
                "/usr/home/game/m2manager_db_backups",
                "Hier legt <code>mysqldump</code> die Sicherungen ab.",
            ),
            f(
                "db_backup_databases",
                "Zu sichernde Datenbanken",
                "account common player log",
                "Leerzeichen-getrennt, wird an <code>mysqldump --databases</code> übergeben.",
            ),
        ],
    },
];

#[component]
pub fn ServerPathsTab() -> impl IntoView {
    view! {
        <div class="space-y-4">
            {GROUPS
                .iter()
                .map(|g| view! {
                    <SettingsCard title=g.title.to_string() description=g.description.to_string() icon=g.icon>
                        {g.fields.iter().map(|d| view! { <TextSetting def=*d/> }).collect_view()}
                    </SettingsCard>
                })
                .collect_view()}
        </div>
    }
}
