//! Broadcast-System und Tag/Nacht & Schnee.
//!
//! Beide teilen dasselbe Prinzip: lokaler SQLite-Zustand (broadcast.rs/
//! weather.rs), Deploy generiert eine Quest-Datei und schaltet sie live per
//! bereits vorhandenem `create_quest_file`/`write_quest_file` +
//! `run_server_command` scharf - kein Server-Neustart nötig.

use crate::broadcast;
use crate::state::AppState;
use crate::weather;
use tauri::State;

#[tauri::command]
pub fn list_broadcast_messages(state: State<'_, AppState>) -> Result<Vec<broadcast::BroadcastMessage>, String> {
    let conn = state.settings_db.lock().map_err(|e| e.to_string())?;
    broadcast::list_messages(&conn)
}

#[tauri::command]
pub fn create_broadcast_message(
    state: State<'_, AppState>,
    text: String,
    interval_minutes: i64,
) -> Result<i64, String> {
    let conn = state.settings_db.lock().map_err(|e| e.to_string())?;
    broadcast::create_message(&conn, &text, interval_minutes)
}

#[tauri::command]
pub fn update_broadcast_message(
    state: State<'_, AppState>,
    id: i64,
    text: String,
    interval_minutes: i64,
) -> Result<(), String> {
    let conn = state.settings_db.lock().map_err(|e| e.to_string())?;
    broadcast::update_message(&conn, id, &text, interval_minutes)
}

#[tauri::command]
pub fn set_broadcast_message_enabled(state: State<'_, AppState>, id: i64, enabled: bool) -> Result<(), String> {
    let conn = state.settings_db.lock().map_err(|e| e.to_string())?;
    broadcast::set_enabled(&conn, id, enabled)
}

#[tauri::command]
pub fn delete_broadcast_message(state: State<'_, AppState>, id: i64) -> Result<(), String> {
    let conn = state.settings_db.lock().map_err(|e| e.to_string())?;
    broadcast::delete_message(&conn, id)
}

// ---- Tag/Nacht & Schnee ----

#[tauri::command]
pub fn get_weather_state(state: State<'_, AppState>) -> Result<weather::WeatherState, String> {
    let conn = state.settings_db.lock().map_err(|e| e.to_string())?;
    weather::get_state(&conn)
}

#[tauri::command]
pub fn set_weather_state(
    state: State<'_, AppState>,
    night_enabled: bool,
    snow_enabled: bool,
) -> Result<weather::WeatherState, String> {
    let conn = state.settings_db.lock().map_err(|e| e.to_string())?;
    weather::set_state(&conn, night_enabled, snow_enabled)
}
