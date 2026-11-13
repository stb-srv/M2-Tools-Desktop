use super::shared::{PathSetting, PathSettingDef, SettingsCard};
use crate::features::shared::ClientUnpackPanel;
use crate::tauri::DialogFilter;
use leptos::prelude::*;

fn def(key: &'static str, label: &'static str, dialog_title: &'static str) -> PathSettingDef {
    PathSettingDef {
        key,
        label,
        hint: None,
        directory: false,
        dialog_title,
        filters: vec![],
        empty_text: None,
        clearable: false,
    }
}

#[component]
pub fn ClientTab() -> impl IntoView {
    view! {
        <div class="space-y-4">
            <SettingsCard title="Spiel-Client".to_string() icon="Gamepad2">
                <PathSetting def=PathSettingDef {
                    directory: true,
                    hint: Some("Ordner des Metin2-Clients (enthält <code>granny2.dll</code>) - für 3D-Modelle, Item-Icons, Icon-Browser und Item-Importer."),
                    ..def("client_path", "Client-Ordner", "Metin2-Client-Ordner auswählen")
                }/>
                <PathSetting def=PathSettingDef {
                    filters: vec![DialogFilter::new("Textdatei", &["txt"])],
                    empty_text: Some("Automatisch suchen"),
                    clearable: true,
                    hint: Some("Normalerweise nicht nötig: M2Manager sucht zuerst unter <code>root\\npclist.txt</code> und sonst im ganzen Client-Ordner. Nur setzen, wenn die Datei umbenannt wurde oder außerhalb des Clients liegt."),
                    ..def("npclist_path", "NPC-Liste (npclist.txt)", "npclist.txt auswählen")
                }/>
            </SettingsCard>

            <SettingsCard
                title="Client-Dateien entpacken".to_string()
                icon="PackageOpen"
                description="Icons, Modelle und Texte liegen in einem frischen Client nur gepackt vor. Hier entpackst du sie, damit M2Manager sie anzeigen kann.".to_string()
            >
                <ClientUnpackPanel client_path=Signal::derive(String::new)/>
            </SettingsCard>

            <SettingsCard
                title="Externe Werkzeuge".to_string()
                icon="Wrench"
                description="Werden vom Item Editor beim Anlegen und Bearbeiten von Items aufgerufen.".to_string()
            >
                <PathSetting def=PathSettingDef {
                    filters: vec![DialogFilter::new("Programm", &["exe"])],
                    hint: Some("Packt <code>icon.epk</code> neu, nachdem ein Item-Icon hinzugefügt wurde. Liegt normalerweise unter <code>Client\\pack\\EterPackConsoleLz4.exe</code>."),
                    ..def("eterpack_tool_path", "EterPackConsoleLz4.exe", "EterPackConsoleLz4.exe auswählen")
                }/>
                <PathSetting def=PathSettingDef {
                    directory: true,
                    hint: Some("Ordner mit <code>Mysql2Proto.exe</code> und <code>Mysql2Proto.json</code> - erzeugt aus der Datenbank eine neue <code>item_proto</code> für den Client."),
                    ..def("mysql2proto_dir", "Mysql2Proto-Ordner", "Mysql2Proto-Ordner auswählen")
                }/>
            </SettingsCard>
        </div>
    }
}
