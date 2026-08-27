use crate::connection_profiles;
use crate::state::AppState;
use tauri::State;

#[tauri::command]
pub async fn save_connection_profile(state: State<'_, AppState>, name: String) -> Result<i64, String> {
    let name = name.trim();
    if name.is_empty() {
        return Err("Profilname darf nicht leer sein.".to_string());
    }
    let conn = state.settings_db.lock().map_err(|e| e.to_string())?;
    connection_profiles::save_current_as_profile(&conn, name)
}

#[tauri::command]
pub async fn list_connection_profiles(
    state: State<'_, AppState>,
) -> Result<Vec<connection_profiles::ConnectionProfile>, String> {
    let conn = state.settings_db.lock().map_err(|e| e.to_string())?;
    connection_profiles::list(&conn)
}

#[tauri::command]
pub async fn activate_connection_profile(state: State<'_, AppState>, id: i64) -> Result<(), String> {
    let conn = state.settings_db.lock().map_err(|e| e.to_string())?;
    connection_profiles::activate(&conn, id)
}

#[tauri::command]
pub async fn delete_connection_profile(state: State<'_, AppState>, id: i64) -> Result<(), String> {
    let conn = state.settings_db.lock().map_err(|e| e.to_string())?;
    connection_profiles::delete(&conn, id)
}
