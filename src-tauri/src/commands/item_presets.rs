use crate::db::item::ItemProtoInput;
use crate::item_presets;
use crate::state::AppState;
use tauri::State;

#[tauri::command]
pub async fn save_item_preset(
    state: State<'_, AppState>,
    name: String,
    item: ItemProtoInput,
) -> Result<i64, String> {
    let name = name.trim();
    if name.is_empty() {
        return Err("Vorlagenname darf nicht leer sein.".to_string());
    }
    let conn = state.settings_db.lock().map_err(|e| e.to_string())?;
    item_presets::save(&conn, name, &item)
}

#[tauri::command]
pub async fn list_item_presets(state: State<'_, AppState>) -> Result<Vec<item_presets::ItemPreset>, String> {
    let conn = state.settings_db.lock().map_err(|e| e.to_string())?;
    item_presets::list(&conn)
}

#[tauri::command]
pub async fn delete_item_preset(state: State<'_, AppState>, id: i64) -> Result<(), String> {
    let conn = state.settings_db.lock().map_err(|e| e.to_string())?;
    item_presets::delete(&conn, id)
}
