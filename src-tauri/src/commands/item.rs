//! Item Editor: item_proto CRUD, Icon/Modell-Einspielen + Repack,
//! Item-Beschreibung (itemdesc.txt).

use crate::db::item::{self, ItemProtoFull, ItemProtoInput};
use crate::itemdesc;
use crate::packtools;
use crate::state::AppState;
use tauri::{AppHandle, State};

use super::support::{item_editor_setting, require_pool};

#[tauri::command]
pub async fn item_vnum_exists(state: State<'_, AppState>, vnum: u32) -> Result<bool, String> {
    let pool = require_pool(&state).await?;
    item::vnum_exists(&pool, vnum).await
}

#[tauri::command]
pub async fn next_free_item_vnum(
    state: State<'_, AppState>,
    range_start: u32,
) -> Result<u32, String> {
    let pool = require_pool(&state).await?;
    item::next_free_vnum(&pool, range_start).await
}

#[tauri::command]
pub async fn get_item_proto(
    state: State<'_, AppState>,
    vnum: u32,
) -> Result<Option<ItemProtoFull>, String> {
    let pool = require_pool(&state).await?;
    item::get_item_proto(&pool, vnum).await
}

#[tauri::command]
pub async fn create_item_proto(
    state: State<'_, AppState>,
    item: ItemProtoInput,
) -> Result<(), String> {
    let pool = require_pool(&state).await?;
    item::create_item_proto(&pool, &item).await
}

#[tauri::command]
pub async fn update_item_proto(
    state: State<'_, AppState>,
    item: ItemProtoInput,
) -> Result<(), String> {
    let pool = require_pool(&state).await?;
    item::update_item_proto(&pool, &item).await
}

#[tauri::command]
pub async fn delete_item_proto(state: State<'_, AppState>, vnum: u32) -> Result<(), String> {
    let pool = require_pool(&state).await?;
    item::delete_item_proto(&pool, vnum).await
}

/// Checks every path the item-creation pipeline depends on *before* anything
/// destructive (including the DB insert) runs, so a misconfigured setting
/// fails loudly upfront instead of leaving a half-created item behind.
#[tauri::command]
pub fn validate_item_editor_setup(
    state: State<'_, AppState>,
    require_icon_tool: bool,
) -> Result<(), String> {
    let mut problems = Vec::new();

    match item_editor_setting(&state, "client_path") {
        Ok(path) if !std::path::Path::new(&path).is_dir() => {
            problems.push(format!("Client-Pfad existiert nicht: {path}"));
        }
        Err(e) => problems.push(e),
        _ => {}
    }

    if require_icon_tool {
        match item_editor_setting(&state, "eterpack_tool_path") {
            Ok(path) if !std::path::Path::new(&path).is_file() => {
                problems.push(format!("EterPackConsoleLz4.exe nicht gefunden unter: {path}"));
            }
            Err(e) => problems.push(e),
            _ => {}
        }
    }

    match item_editor_setting(&state, "mysql2proto_dir") {
        Ok(dir) => {
            let exe = std::path::Path::new(&dir).join("Mysql2Proto.exe");
            if !exe.is_file() {
                problems.push(format!("Mysql2Proto.exe nicht gefunden unter: {}", exe.display()));
            }
        }
        Err(e) => problems.push(e),
    }

    if problems.is_empty() {
        Ok(())
    } else {
        Err(problems.join("\n"))
    }
}

#[tauri::command]
pub fn write_item_icon(
    state: State<'_, AppState>,
    vnum: u32,
    source_path: String,
) -> Result<String, String> {
    let client_path = item_editor_setting(&state, "client_path")?;
    let path = packtools::write_icon_tga(&client_path, vnum, &source_path)?;
    Ok(path.display().to_string())
}

#[tauri::command]
pub async fn pack_item_icons(app: AppHandle, state: State<'_, AppState>) -> Result<(), String> {
    let tool_path = item_editor_setting(&state, "eterpack_tool_path")?;
    packtools::run_eterpack_pack(&app, &tool_path, "icon").await
}

#[tauri::command]
pub fn write_item_model(
    state: State<'_, AppState>,
    vnum: u32,
    source_vnum: u32,
) -> Result<String, String> {
    let client_path = item_editor_setting(&state, "client_path")?;
    let path = packtools::copy_weapon_model(&client_path, vnum, source_vnum)?;
    Ok(path.display().to_string())
}

#[tauri::command]
pub async fn pack_item_models(app: AppHandle, state: State<'_, AppState>) -> Result<(), String> {
    let tool_path = item_editor_setting(&state, "eterpack_tool_path")?;
    packtools::run_eterpack_pack(&app, &tool_path, "item").await
}

#[tauri::command]
pub fn write_item_list_entry(
    state: State<'_, AppState>,
    vnum: u32,
    item_type: u32,
    icon_rel_path: String,
    model_rel_path: Option<String>,
) -> Result<Vec<String>, String> {
    let client_path = item_editor_setting(&state, "client_path")?;
    packtools::upsert_item_list_entries(
        &client_path,
        vnum,
        item_type,
        &icon_rel_path,
        model_rel_path.as_deref(),
    )
}

// ---- Item-Beschreibung (locale/<lang>/itemdesc.txt - client-seitig, keine
// item_proto-Spalte) ----

#[tauri::command]
pub fn get_item_desc(state: State<'_, AppState>, vnum: u32) -> Result<Option<itemdesc::ItemDescEntry>, String> {
    let client_path = item_editor_setting(&state, "client_path")?;
    itemdesc::read_entry(&client_path, vnum)
}

#[tauri::command]
pub fn write_item_desc(
    state: State<'_, AppState>,
    vnum: u32,
    description: String,
    summary: String,
) -> Result<(), String> {
    let client_path = item_editor_setting(&state, "client_path")?;
    itemdesc::write_entry(&client_path, vnum, &description, &summary)
}

#[tauri::command]
pub async fn regenerate_item_proto(
    app: AppHandle,
    state: State<'_, AppState>,
) -> Result<String, String> {
    let tool_dir = item_editor_setting(&state, "mysql2proto_dir")?;
    let generated = packtools::run_mysql2proto(&app, &tool_dir).await?;
    Ok(generated.display().to_string())
}

#[tauri::command]
pub fn deploy_item_proto(
    state: State<'_, AppState>,
    generated_proto_path: String,
) -> Result<Vec<String>, String> {
    let client_path = item_editor_setting(&state, "client_path")?;
    packtools::replace_client_item_proto(&client_path, &generated_proto_path)
}
