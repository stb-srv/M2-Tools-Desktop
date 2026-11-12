//! Mob Drop Editor (`mob_drop_item.txt`) plus its local-file syntax-check
//! variant, and the sibling Drop-Generator files (`common_drop_item.txt`,
//! `etc_drop_item.txt`, `drop_item_group.txt`) - grouped together here
//! rather than split further since they share the same parse/serialize
//! round-trip-check pattern and each individually would be a handful of
//! lines.

use super::support::stored_ssh_auth;
use crate::common_drop;
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
use serde::Serialize;

fn mob_drop_file_path(state: &State<'_, AppState>) -> Result<String, String> {
    let conn = state.settings_db.lock().map_err(|e| e.to_string())?;
    Ok(settings::get_path(&conn, "mob_drop_file_path")?
        .filter(|v| !v.is_empty())
        .unwrap_or_else(|| "/usr/home/game/share/mob_drop_item.txt".to_string()))
}

// Bundles the parsed groups with the numbering diagnostic (see
// `mobdrop::check_numbering`) so the frontend gets both from one round-trip
// instead of needing the raw file content a second time just to re-run the
// check itself.
#[derive(Debug, Clone, Serialize)]
pub struct MobDropLoadResult {
    pub groups: Vec<mobdrop::MobDropGroup>,
    pub numbering_issues: Vec<mobdrop::NumberingIssue>,
}

#[tauri::command]
pub async fn read_mob_drop_file(state: State<'_, AppState>) -> Result<MobDropLoadResult, String> {
    let (config, auth) = stored_ssh_auth(&state)?;
    let path = mob_drop_file_path(&state)?;
    let content = ssh::read_remote_file(&config, &auth, &path).await?;
    let groups = mobdrop::parse(&content)?;
    let numbering_issues = mobdrop::check_numbering(&content);
    Ok(MobDropLoadResult { groups, numbering_issues })
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

// Klassisches Rang-Format (4 Blöcke x 6 Felder), siehe `common_drop.rs`.
// Unlesbare Zeilen kommen als `Invalid` zurück statt den ganzen Ladevorgang
// abzubrechen; Speichern verweigert sie (`common_drop::serialize`).
#[tauri::command]
pub async fn read_common_drop_file(state: State<'_, AppState>) -> Result<common_drop::CommonDropFile, String> {
    let (config, auth) = stored_ssh_auth(&state)?;
    let path = common_drop_file_path(&state)?;
    let content = ssh::read_remote_file(&config, &auth, &path).await?;
    Ok(common_drop::parse(&content))
}

#[tauri::command]
pub async fn write_common_drop_file(
    state: State<'_, AppState>,
    file: common_drop::CommonDropFile,
) -> Result<Option<String>, String> {
    let blocking: Vec<String> = common_drop::validate(&file)
        .into_iter()
        .filter(|i| i.level == common_drop::IssueLevel::Error)
        .map(|i| format!("Zeile {}: {}", i.line_index + 1, i.message))
        .collect();
    if !blocking.is_empty() {
        return Err(format!("Nicht gespeichert, die Datei hat noch Fehler:\n{}", blocking.join("\n")));
    }
    let content = common_drop::serialize(&file)?;
    // Rundreise-Prüfung: nur hochladen, was wir selbst identisch zurücklesen.
    let reparsed = common_drop::parse(&content);
    if reparsed.lines.len() != file.lines.len()
        || reparsed.lines.iter().any(|l| matches!(l, common_drop::CommonDropLine::Invalid { .. }))
    {
        return Err("Interner Fehler: die erzeugte Datei ließ sich nicht wieder einlesen - nichts gespeichert.".into());
    }
    let (config, auth) = stored_ssh_auth(&state)?;
    let path = common_drop_file_path(&state)?;
    ssh::write_remote_file_with_backup(&config, &auth, &path, &content).await
}

/// Prüfungen ohne Datenbank (Level, Chance, gesperrte VNUMs, Feldanzahl).
#[tauri::command]
pub fn validate_common_drop(file: common_drop::CommonDropFile) -> Vec<common_drop::CommonDropIssue> {
    common_drop::validate(&file)
}

/// Eine einzelne, von Hand korrigierte Rohzeile neu lesen.
#[tauri::command]
pub fn parse_common_drop_line(line: String) -> common_drop::CommonDropLine {
    let mut file = common_drop::parse(&line);
    file.lines.pop().unwrap_or(common_drop::CommonDropLine::Text { text: String::new() })
}

#[derive(Debug, Clone, Serialize)]
pub struct CommonDropItemCheck {
    pub vnum: u32,
    /// Gibt es ein Item mit dieser VNUM (auch über `vnum_range`)?
    pub exists: bool,
    pub locale_name: Option<String>,
    /// Der Server versucht das VNUM-Feld zuerst als internen Item-Namen.
    /// Heißt ein anderes Item genau so, nimmt er dieses.
    pub name_collision: Option<ItemBrief>,
}

#[tauri::command]
pub async fn check_common_drop_items(
    state: State<'_, AppState>,
    vnums: Vec<u32>,
) -> Result<Vec<CommonDropItemCheck>, String> {
    let pool = require_pool(&state).await?;
    let mut out = Vec::with_capacity(vnums.len());
    for vnum in vnums {
        let brief = item::get_item_brief(&pool, vnum).await?;
        let exists = brief.is_some() || item::vnum_exists(&pool, vnum).await?;
        let name_collision = item::find_item_by_internal_name(&pool, &vnum.to_string())
            .await?
            .filter(|other| other.vnum != vnum);
        out.push(CommonDropItemCheck { vnum, exists, locale_name: brief.map(|b| b.locale_name), name_collision });
    }
    Ok(out)
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
pub fn parse_mob_drop_text(content: String) -> Result<MobDropLoadResult, String> {
    let groups = mobdrop::parse(&content)?;
    let numbering_issues = mobdrop::check_numbering(&content);
    Ok(MobDropLoadResult { groups, numbering_issues })
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
