//! Kleine Restgruppe ohne eigenes größeres Feature: GR2-Modell laden,
//! Bild-Konvertierung/-Vorschau (TGA Converter), Item-/NPC-Icons und
//! -Modelle lokal auflösen, generischer lokaler Text-Export, und die
//! globalen Server-Event-Flags (siehe db/event_flags.rs).

use crate::db::event_flags;
use crate::gr2::{self, ModelInfo};
use crate::icons;
use crate::settings;
use crate::state::AppState;
use tauri::State;

use super::support::{client_path_setting, require_pool};

#[tauri::command]
pub fn export_text_file(path: String, content: String) -> Result<(), String> {
    std::fs::write(&path, content).map_err(|e| format!("Datei konnte nicht geschrieben werden: {e}"))
}

#[tauri::command]
pub fn convert_image_to_tga(source_path: String, dest_path: String) -> Result<(), String> {
    crate::imageconv::convert_to_tga(&source_path, std::path::Path::new(&dest_path))
}

#[tauri::command]
pub fn preview_image_file(path: String) -> Result<String, String> {
    crate::imageconv::preview_as_data_url(&path)
}

#[tauri::command]
pub fn load_gr2_model(granny_dll_path: String, gr2_path: String) -> Result<ModelInfo, String> {
    gr2::parse(&granny_dll_path, &gr2_path)
}

#[tauri::command]
pub fn check_client_path(path: String) -> bool {
    gr2::find_granny_dll(&path).is_some()
}

#[tauri::command]
pub fn get_item_icon(
    state: State<'_, AppState>,
    vnum: u32,
) -> Result<Option<String>, String> {
    let client_path = {
        let conn = state.settings_db.lock().map_err(|e| e.to_string())?;
        settings::get_path(&conn, "client_path")?
    }
    .ok_or_else(|| "Kein Client-Pfad konfiguriert.".to_string())?;
    icons::load_item_icon_data_url(&client_path, vnum)
}

#[tauri::command]
pub fn list_item_icon_files(state: State<'_, AppState>) -> Result<Vec<icons::IconFile>, String> {
    let client_path = client_path_setting(&state)?;
    Ok(icons::list_item_icon_files(&client_path))
}

#[tauri::command]
pub fn load_icon_file(absolute_path: String) -> Result<String, String> {
    icons::load_icon_file_data_url(&absolute_path)
}

#[tauri::command]
pub fn locate_npc_model(
    state: State<'_, AppState>,
    npc_vnum: i32,
    folder: Option<String>,
) -> Result<(String, String), String> {
    let (client_path, npclist_override) = {
        let conn = state.settings_db.lock().map_err(|e| e.to_string())?;
        (
            settings::get_path(&conn, "client_path")?,
            settings::get_path(&conn, "npclist_path")?,
        )
    };
    let client_path = client_path.ok_or_else(|| {
        "Kein Client-Pfad konfiguriert. Bitte in den Einstellungen setzen.".to_string()
    })?;

    let dll = gr2::find_granny_dll(&client_path)
        .ok_or_else(|| format!("granny2.dll nicht gefunden unter {client_path}"))?;

    // npclist.txt is the client's own mapping and covers shop NPCs, whose
    // mob_proto.folder is typically empty - fall back to the DB value only if
    // the client list has no entry.
    let from_list = gr2::find_npclist(&client_path, npclist_override.as_deref())
        .and_then(|list| gr2::lookup_npc_folder(&list, npc_vnum));

    let resolved = from_list
        .or(folder)
        .filter(|f| !f.is_empty())
        .ok_or_else(|| {
            format!(
                "Kein Modell-Ordner für NPC {npc_vnum} gefunden. \
                 npclist.txt wurde nicht gefunden oder enthält keinen Eintrag - \
                 Pfad ggf. in den Einstellungen setzen."
            )
        })?;

    let model = gr2::find_npc_model(&client_path, &resolved).ok_or_else(|| {
        format!("Kein .gr2-Modell für '{resolved}' im Client-Ordner gefunden")
    })?;
    Ok((dll, model))
}

// ---- Server-Events (globale Event-Flags, siehe db/event_flags.rs) ----

#[tauri::command]
pub async fn list_event_flags(state: State<'_, AppState>) -> Result<Vec<event_flags::EventFlagRow>, String> {
    let pool = require_pool(&state).await?;
    event_flags::list_event_flags(&pool).await
}

#[tauri::command]
pub async fn set_event_flag(state: State<'_, AppState>, name: String, value: i32) -> Result<(), String> {
    let pool = require_pool(&state).await?;
    event_flags::set_event_flag(&pool, &name, value).await
}

#[tauri::command]
pub async fn delete_event_flag(state: State<'_, AppState>, name: String) -> Result<(), String> {
    let pool = require_pool(&state).await?;
    event_flags::delete_event_flag(&pool, &name).await
}
