//! Mob Drop Editor (`mob_drop_item.txt`) plus its local-file syntax-check
//! variant, and the sibling Drop-Generator files (`common_drop_item.txt`,
//! `etc_drop_item.txt`, `drop_item_group.txt`) - grouped together here
//! rather than split further since they share the same parse/serialize
//! round-trip-check pattern and each individually would be a handful of
//! lines.

use super::support::stored_ssh_auth;
use crate::db::item::{self, ItemBrief};
use crate::drop_item_group;
use crate::etc_drop;
use crate::mobdrop;
use crate::packtools;
use crate::settings;
use crate::ssh;
use crate::state::AppState;
use tauri::State;

use super::support::require_pool;

fn mob_drop_file_path(state: &State<'_, AppState>) -> Result<String, String> {
    let conn = state.settings_db.lock().map_err(|e| e.to_string())?;
    Ok(settings::get_path(&conn, "mob_drop_file_path")?
        .filter(|v| !v.is_empty())
        .unwrap_or_else(|| "/usr/home/game/share/mob_drop_item.txt".to_string()))
}

#[tauri::command]
pub async fn read_mob_drop_file(
    state: State<'_, AppState>,
) -> Result<Vec<mobdrop::MobDropGroup>, String> {
    let (config, auth) = stored_ssh_auth(&state)?;
    let path = mob_drop_file_path(&state)?;
    let content = ssh::read_remote_file(&config, &auth, &path).await?;
    mobdrop::parse(&content)
}

#[tauri::command]
pub async fn write_mob_drop_file(
    state: State<'_, AppState>,
    groups: Vec<mobdrop::MobDropGroup>,
) -> Result<Option<String>, String> {
    let (config, auth) = stored_ssh_auth(&state)?;
    let path = mob_drop_file_path(&state)?;
    let content = mobdrop::serialize(&groups);
    // Round-trip-check what we're about to write before touching the
    // server - refuse to upload something we couldn't parse back ourselves.
    mobdrop::parse(&content)?;
    ssh::write_remote_file_with_backup(&config, &auth, &path, &content).await
}

#[tauri::command]
pub fn sanitize_mob_drop_group_name(name: String) -> String {
    mobdrop::sanitize_group_name(&name)
}

// ---- Drop-Generator: common_drop_item.txt / etc_drop_item.txt /
// drop_item_group.txt (siehe common_drop.rs/etc_drop.rs/drop_item_group.rs
// für die verifizierten Datei-Formate) - gleiches SFTP-Lade/Speicher-Muster
// wie cube.txt/special_item_group.txt: Rundreise-Sanity-Check vor jedem
// Hochladen, Backup vor jedem Überschreiben (`write_remote_file_with_backup`).

fn common_drop_file_path(state: &State<'_, AppState>) -> Result<String, String> {
    let conn = state.settings_db.lock().map_err(|e| e.to_string())?;
    Ok(settings::get_path(&conn, "common_drop_file_path")?
        .filter(|v| !v.is_empty())
        .unwrap_or_else(|| "/usr/home/game/share/common_drop_item.txt".to_string()))
}

// Live gegen den echten Dev-Server verifiziert (2026-08-18, nach einem
// realen Ladefehler): entgegen der ursprünglichen, aus dem generischen
// game-src-Checkout abgeleiteten Annahme (24-Tab-Felder-pro-Zeile,
// Rang-Level-Brackets - `ReadCommonDropItemFile` in
// `item_manager_read_tables.cpp`) benutzt dieser Fork für
// `common_drop_item.txt` tatsächlich **dieselbe Group/Mob/Type-Grammatik
// wie `mob_drop_item.txt`** (echter Datei-Inhalt: `Group\tMetinStein1\n{\n
// \tMob\t8001\n\tType\tdrop\n\t1\t19\t1\t100\n}`) - der generische
// Quellcode-Checkout entspricht an dieser Stelle offenbar nicht dem, was auf
// diesem Server tatsächlich läuft (individuelle Fork-Anpassung). Wiederverwendet
// deshalb direkt `mobdrop::parse`/`serialize` (identischer `MobDropGroup`-Typ)
// statt eines eigenen, nachweislich falschen Parsers.
#[tauri::command]
pub async fn read_common_drop_file(state: State<'_, AppState>) -> Result<Vec<mobdrop::MobDropGroup>, String> {
    let (config, auth) = stored_ssh_auth(&state)?;
    let path = common_drop_file_path(&state)?;
    let content = ssh::read_remote_file(&config, &auth, &path).await?;
    mobdrop::parse(&content)
}

#[tauri::command]
pub async fn write_common_drop_file(
    state: State<'_, AppState>,
    groups: Vec<mobdrop::MobDropGroup>,
) -> Result<Option<String>, String> {
    let (config, auth) = stored_ssh_auth(&state)?;
    let path = common_drop_file_path(&state)?;
    let content = mobdrop::serialize(&groups);
    mobdrop::parse(&content)?;
    ssh::write_remote_file_with_backup(&config, &auth, &path, &content).await
}

fn etc_drop_file_path(state: &State<'_, AppState>) -> Result<String, String> {
    let conn = state.settings_db.lock().map_err(|e| e.to_string())?;
    Ok(settings::get_path(&conn, "etc_drop_file_path")?
        .filter(|v| !v.is_empty())
        .unwrap_or_else(|| "/usr/home/game/share/etc_drop_item.txt".to_string()))
}

#[tauri::command]
pub async fn read_etc_drop_file(state: State<'_, AppState>) -> Result<Vec<etc_drop::EtcDropEntry>, String> {
    let (config, auth) = stored_ssh_auth(&state)?;
    let path = etc_drop_file_path(&state)?;
    let content = ssh::read_remote_file(&config, &auth, &path).await?;
    etc_drop::parse(&content)
}

#[tauri::command]
pub async fn write_etc_drop_file(
    state: State<'_, AppState>,
    entries: Vec<etc_drop::EtcDropEntry>,
) -> Result<Option<String>, String> {
    let (config, auth) = stored_ssh_auth(&state)?;
    let path = etc_drop_file_path(&state)?;
    let content = etc_drop::serialize(&entries);
    etc_drop::parse(&content)?;
    ssh::write_remote_file_with_backup(&config, &auth, &path, &content).await
}

fn drop_item_group_file_path(state: &State<'_, AppState>) -> Result<String, String> {
    let conn = state.settings_db.lock().map_err(|e| e.to_string())?;
    Ok(settings::get_path(&conn, "drop_item_group_file_path")?
        .filter(|v| !v.is_empty())
        .unwrap_or_else(|| "/usr/home/game/share/drop_item_group.txt".to_string()))
}

#[tauri::command]
pub async fn read_drop_item_group_file(
    state: State<'_, AppState>,
) -> Result<Vec<drop_item_group::DropItemGroup>, String> {
    let (config, auth) = stored_ssh_auth(&state)?;
    let path = drop_item_group_file_path(&state)?;
    let content = ssh::read_remote_file(&config, &auth, &path).await?;
    drop_item_group::parse(&content)
}

#[tauri::command]
pub async fn write_drop_item_group_file(
    state: State<'_, AppState>,
    groups: Vec<drop_item_group::DropItemGroup>,
) -> Result<Option<String>, String> {
    let (config, auth) = stored_ssh_auth(&state)?;
    let path = drop_item_group_file_path(&state)?;
    let content = drop_item_group::serialize(&groups);
    drop_item_group::parse(&content)?;
    ssh::write_remote_file_with_backup(&config, &auth, &path, &content).await
}

/// Löst die per `EntityBrowser` gewählte vnum in den echten internen
/// `item_proto.name` auf - für Etc-Drops, siehe `etc_drop.rs`.
#[tauri::command]
pub async fn get_item_internal_name(state: State<'_, AppState>, vnum: u32) -> Result<String, String> {
    let pool = require_pool(&state).await?;
    item::get_item_internal_name(&pool, vnum).await
}

/// Rückwärtssuche für die Anzeige bestehender Etc-Drop-Einträge - siehe
/// `etc_drop.rs`.
#[tauri::command]
pub async fn find_item_by_internal_name(state: State<'_, AppState>, name: String) -> Result<Option<ItemBrief>, String> {
    let pool = require_pool(&state).await?;
    item::find_item_by_internal_name(&pool, &name).await
}

// ---- Mob Drop Editor: local file variant (syntax check/repair tab) ----

#[tauri::command]
pub fn read_local_text_file(path: String) -> Result<String, String> {
    std::fs::read_to_string(&path).map_err(|e| format!("Datei konnte nicht gelesen werden: {e}"))
}

#[tauri::command]
pub fn parse_mob_drop_text(content: String) -> Result<Vec<mobdrop::MobDropGroup>, String> {
    mobdrop::parse(&content)
}

#[tauri::command]
pub fn write_local_mob_drop_file(
    path: String,
    groups: Vec<mobdrop::MobDropGroup>,
) -> Result<Option<String>, String> {
    let content = mobdrop::serialize(&groups);
    // Same sanity check as the server write path - never write something we
    // couldn't parse back ourselves.
    mobdrop::parse(&content)?;
    let backup = packtools::backup_file(std::path::Path::new(&path))?;
    std::fs::write(&path, content).map_err(|e| format!("Datei konnte nicht geschrieben werden: {e}"))?;
    Ok(backup.map(|p| p.display().to_string()))
}
