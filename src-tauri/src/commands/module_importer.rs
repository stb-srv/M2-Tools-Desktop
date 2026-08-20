//! Modul-Importer (beliebige Ausrüstungs-Pakete: Waffen + Rüstung), plus
//! Verlauf & Rückgängig-machen.

use crate::db::item;
use crate::import_history;
use crate::modulescan::{self, ScannedModule};
use crate::msm;
use crate::packtools;
use crate::refine;
use crate::state::AppState;
use tauri::{AppHandle, State};

use super::support::{item_editor_setting, require_pool};

#[tauri::command]
pub fn scan_module(path: String) -> Result<ScannedModule, String> {
    modulescan::scan_module(std::path::Path::new(&path))
}

/// Scan for the icon-only importer (accessories with no 3D model at all -
/// shoes, necklaces, shields, earrings, bracelets, ...): just every image
/// file under the folder, no weapon/armor detection.
#[tauri::command]
pub fn scan_icon_folder(path: String) -> Result<Vec<String>, String> {
    modulescan::scan_icon_folder(std::path::Path::new(&path))
}

#[tauri::command]
pub fn import_weapon_model(
    state: State<'_, AppState>,
    module_name: String,
    vnum: u32,
    source_abs: String,
    texture_sources: Vec<String>,
) -> Result<(String, String), String> {
    let client_path = item_editor_setting(&state, "client_path")?;
    let (dest, virtual_path) = packtools::import_custom_weapon_model(
        &client_path,
        &module_name,
        vnum,
        std::path::Path::new(&source_abs),
        &texture_sources,
    )?;
    Ok((dest.display().to_string(), virtual_path))
}

/// Allocates a single fresh `value3`/`ShapeIndex` for a whole armor piece -
/// call **once per item**, before looping `import_armor_model` over that
/// item's selected races. It must not be re-derived per race: the same
/// numeric index has to end up in every involved race's `.msm` (each
/// pointing at that race's own model) so the one `item_proto` row renders
/// consistently everywhere it's equippable; calling this per-race would
/// hand out a different, incompatible index each time since every previous
/// `.msm` write raises the observed maximum.
#[tauri::command]
pub async fn next_free_shape_index(state: State<'_, AppState>) -> Result<u32, String> {
    let client_path = item_editor_setting(&state, "client_path")?;
    let pool = require_pool(&state).await?;
    let max_db_value3 = item::max_armor_value3(&pool).await?;
    Ok(msm::next_free_shape_index(&client_path, max_db_value3))
}

/// Imports a female armor body model and wires it into that race's `.msm`
/// under the given (already-allocated, see `next_free_shape_index`)
/// `shape_index` - see `msm.rs`'s module doc for why there is no male
/// equivalent.
#[tauri::command]
pub fn import_armor_model(
    state: State<'_, AppState>,
    module_name: String,
    race: String,
    source_abs: String,
    texture_sources: Vec<String>,
    shape_index: u32,
) -> Result<(), String> {
    let client_path = item_editor_setting(&state, "client_path")?;
    let (_, model_rel, skin_rel) = packtools::import_custom_armor_model(
        &client_path,
        &module_name,
        &race,
        std::path::Path::new(&source_abs),
        &texture_sources,
    )?;
    msm::add_shape_data(&client_path, &race, shape_index, &model_rel, &skin_rel)
}

/// Repacks `<client>/pack/<folder_name>.epk` - a generic counterpart to
/// `pack_item_icons`/`pack_item_models`/`pack_item_effects` for the
/// module importer's other pack folders (e.g. `pc_warrior`, `pc_sura`).
#[tauri::command]
pub async fn pack_folder(app: AppHandle, state: State<'_, AppState>, folder_name: String) -> Result<(), String> {
    let tool_path = item_editor_setting(&state, "eterpack_tool_path")?;
    packtools::run_eterpack_pack(&app, &tool_path, &folder_name).await
}

#[tauri::command]
pub fn import_effect_bundle(
    state: State<'_, AppState>,
    module_name: String,
    source_dir: String,
) -> Result<Vec<String>, String> {
    let client_path = item_editor_setting(&state, "client_path")?;
    let copied =
        packtools::import_effect_bundle(&client_path, &module_name, std::path::Path::new(&source_dir))?;
    Ok(copied.into_iter().map(|p| p.display().to_string()).collect())
}

#[tauri::command]
pub async fn pack_item_effects(app: AppHandle, state: State<'_, AppState>) -> Result<(), String> {
    let tool_path = item_editor_setting(&state, "eterpack_tool_path")?;
    packtools::run_eterpack_pack(&app, &tool_path, "effect").await
}

// ---- Modul-Importer: Verlauf & Rückgängig-machen ----

#[tauri::command]
pub fn record_import_batch(
    state: State<'_, AppState>,
    module_name: String,
    item_type: i32,
    vnums: Vec<u32>,
    had_effects: bool,
) -> Result<i64, String> {
    let conn = state.settings_db.lock().map_err(|e| e.to_string())?;
    import_history::record_batch(&conn, &module_name, item_type, &vnums, had_effects)
}

#[tauri::command]
pub fn list_import_batches(state: State<'_, AppState>) -> Result<Vec<import_history::ImportBatch>, String> {
    let conn = state.settings_db.lock().map_err(|e| e.to_string())?;
    import_history::list_batches(&conn)
}

/// Fully undoes a single imported item: removes its `item_proto` row, its
/// `item_list.txt` entry (in every locale) and its icon file. Does *not*
/// touch the copied `.gr2`/effect files under `pack/.../custom/<module>/` -
/// those can be shared by other vnums imported from the same module run
/// (e.g. re-running an import overwrites the same destination path), so
/// deleting them per-vnum would risk breaking a sibling item. Leaving them
/// behind is harmless: without a DB row or `item_list.txt` entry, nothing
/// references them anymore.
async fn teardown_item(pool: &sqlx::MySqlPool, client_path: &str, vnum: u32) -> Result<(), String> {
    // Fetched *before* the delete below so the item's own refine linkage
    // is still readable - used afterward to also drop its recipe if this
    // was the last item using it (e.g. undoing a Module-Importer refine
    // chain, see refine.rs's module doc). A recipe still shared by other
    // items (`delete_recipe` errors on that) is deliberately left alone;
    // the error is swallowed since that's the expected/correct outcome,
    // not a failure of this teardown.
    let refine_set: Option<u16> = sqlx::query("SELECT refine_set FROM player.item_proto WHERE vnum = ?")
        .bind(vnum)
        .fetch_optional(pool)
        .await
        .map_err(|e| e.to_string())?
        .map(|row| sqlx::Row::try_get(&row, "refine_set").unwrap_or_default());

    item::delete_item_proto(pool, vnum).await?;
    packtools::remove_item_list_entries(client_path, vnum)?;
    packtools::delete_item_icon(client_path, vnum)?;

    if let Some(refine_set) = refine_set {
        if refine_set != 0 {
            let _ = refine::delete_recipe(pool, refine_set as i32).await;
        }
    }
    Ok(())
}

/// Rolls back one item created earlier in an import batch that then failed
/// (e.g. the repack step erroring out after several items were already
/// created). Deliberately skips `pack_item_icons`/`regenerate_item_proto` -
/// unlike `remove_single_item`, this runs *from inside* a failure handler,
/// so repacking again here would only add another risky operation on top
/// of whatever already went wrong; the next successful import's own repack
/// step naturally cleans up the icon.epk/item_proto inconsistency this
/// leaves behind in the meantime (harmless: nothing references vnum
/// without a DB row or item_list.txt entry, both removed here).
#[tauri::command]
pub async fn rollback_created_item(state: State<'_, AppState>, vnum: u32) -> Result<(), String> {
    let pool = require_pool(&state).await?;
    let client_path = item_editor_setting(&state, "client_path")?;
    teardown_item(&pool, &client_path, vnum).await
}

#[tauri::command]
pub async fn remove_single_item(app: AppHandle, state: State<'_, AppState>, vnum: u32) -> Result<(), String> {
    let pool = require_pool(&state).await?;
    let client_path = item_editor_setting(&state, "client_path")?;

    teardown_item(&pool, &client_path, vnum).await?;

    let tool_path = item_editor_setting(&state, "eterpack_tool_path")?;
    packtools::run_eterpack_pack(&app, &tool_path, "icon").await?;
    let generated = packtools::run_mysql2proto(&app, &item_editor_setting(&state, "mysql2proto_dir")?).await?;
    packtools::replace_client_item_proto(&client_path, &generated.display().to_string())?;
    Ok(())
}

#[tauri::command]
pub async fn undo_import_batch(app: AppHandle, state: State<'_, AppState>, id: i64) -> Result<(), String> {
    let batch = {
        let conn = state.settings_db.lock().map_err(|e| e.to_string())?;
        import_history::get_batch(&conn, id)?
            .ok_or_else(|| format!("Import-Batch {id} nicht gefunden (bereits entfernt?)"))?
    };

    let pool = require_pool(&state).await?;
    let client_path = item_editor_setting(&state, "client_path")?;

    for vnum in &batch.vnums {
        teardown_item(&pool, &client_path, *vnum).await?;
    }

    let tool_path = item_editor_setting(&state, "eterpack_tool_path")?;
    packtools::run_eterpack_pack(&app, &tool_path, "icon").await?;
    let generated = packtools::run_mysql2proto(&app, &item_editor_setting(&state, "mysql2proto_dir")?).await?;
    packtools::replace_client_item_proto(&client_path, &generated.display().to_string())?;

    let conn = state.settings_db.lock().map_err(|e| e.to_string())?;
    import_history::delete_batch_record(&conn, id)?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    async fn real_pool() -> sqlx::MySqlPool {
        let conn = rusqlite::Connection::open(
            r"C:\Users\DevSteven\AppData\Roaming\com.m2manager.app\m2manager.sqlite",
        )
        .expect("open settings db");
        let get = |key: &str| -> Option<String> {
            conn.query_row("SELECT value FROM paths WHERE key = ?1", [key], |r| r.get(0))
                .ok()
        };
        let host = get("mysql_host").expect("mysql_host not configured on this machine");
        let port: u16 = get("mysql_port").unwrap_or_else(|| "3306".into()).parse().unwrap();
        let user = get("mysql_username").expect("mysql_username not configured");
        let password = keyring::Entry::new("m2manager", "mysql_password")
            .unwrap()
            .get_password()
            .expect("mysql_password credential not stored");
        let url = format!("mysql://{user}:{password}@{host}:{port}/player");
        sqlx::mysql::MySqlPoolOptions::new()
            .max_connections(1)
            .connect(&url)
            .await
            .expect("connect to dev DB")
    }

    // Regression test for the Module Importer's refine-chain feature
    // (2026-08-06): undoing an item that owned its own private (not
    // shared) refine recipe must also drop that now-unused recipe, not
    // just the item - otherwise every rolled-back or undone chain leaves
    // orphaned refine_proto rows behind forever. A recipe still used by
    // *another* item must survive the same teardown untouched.
    #[tokio::test]
    async fn teardown_item_drops_its_own_unshared_recipe_but_not_a_shared_one() {
        let pool = real_pool().await;
        let client_path = r"C:\Users\DevSteven\Desktop\Client";

        // High throwaway vnums, well outside any real item range on this
        // server (verified in earlier sessions to top out well under
        // 900000) - safe test fixture that can't collide with real data.
        let owner_vnum = 900001u32;
        let sharer_vnum = 900002u32;
        for v in [owner_vnum, sharer_vnum] {
            let _ = item::delete_item_proto(&pool, v).await;
        }

        let recipe_id = refine::save_recipe(&pool, None, 1000, 90, &[]).await.expect("create recipe");

        let mut item = crate::db::item::ItemProtoInput {
            vnum: owner_vnum,
            vnum_range: 0,
            name: "test_teardown_owner".into(),
            locale_name: "TeardownTestOwner".into(),
            r#type: 1,
            subtype: 0,
            weight: 0,
            size: 1,
            antiflag: 0,
            flag: 0,
            wearflag: 0,
            immuneflag: 0,
            gold: 0,
            shop_buy_price: 0,
            refined_vnum: 0,
            refine_set: recipe_id as u16,
            magic_pct: 0,
            limittype0: 0,
            limitvalue0: 0,
            limittype1: 0,
            limitvalue1: 0,
            applytype0: 0,
            applyvalue0: 0,
            applytype1: 0,
            applyvalue1: 0,
            applytype2: 0,
            applyvalue2: 0,
            applytype3: 0,
            applyvalue3: 0,
            value0: 0,
            value1: 0,
            value2: 0,
            value3: 0,
            value4: 0,
            value5: 0,
            socket0: 0,
            socket1: 0,
            socket2: 0,
            socket3: 0,
            socket4: 0,
            socket5: 0,
            specular: 0,
            socket_pct: 0,
            addon_type: 0,
        };
        item::create_item_proto(&pool, &item).await.expect("create owner item");
        item.vnum = sharer_vnum;
        item.name = "test_teardown_sharer".into();
        item::create_item_proto(&pool, &item).await.expect("create sharer item");

        teardown_item(&pool, client_path, owner_vnum).await.expect("teardown owner");

        assert!(
            !item::vnum_exists(&pool, owner_vnum).await.unwrap(),
            "owner item must be gone"
        );
        assert!(
            item::vnum_exists(&pool, sharer_vnum).await.unwrap(),
            "sharer item must be untouched"
        );
        let recipe_after = refine::get_recipe(&pool, recipe_id).await.unwrap();
        assert!(
            recipe_after.is_some(),
            "recipe must survive - still used by the sharer item"
        );

        teardown_item(&pool, client_path, sharer_vnum).await.expect("teardown sharer");
        assert!(!item::vnum_exists(&pool, sharer_vnum).await.unwrap());
        let recipe_gone = refine::get_recipe(&pool, recipe_id).await.unwrap();
        assert!(
            recipe_gone.is_none(),
            "recipe must be dropped once the last item using it is gone"
        );
    }
}
