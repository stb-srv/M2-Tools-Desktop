// MSVC's linker prints informational "creating library/object file" notes
// when building the cdylib's import lib - harmless, just noisy.
#![allow(linker_messages)]

mod backups;
mod bans;
mod broadcast;
mod build_deploy;
mod commands;
mod credentials;
mod cube;
mod db;
mod drop_item_group;
mod etc_drop;
mod gr2;
mod icons;
mod imageconv;
mod import_history;
mod itemdesc;
mod locale;
mod mapdata;
mod mobdrop;
mod modulescan;
mod msm;
mod packtools;
mod quest;
mod refine;
mod regen;
mod db_backup;
mod deploy_history;
mod resources;
mod webhook;
mod settings;
mod special_item_group;
mod ssh;
mod state;
mod system_installs;
mod system_patch;
mod system_scan;
mod textures;
mod weather;

use state::AppState;
use tauri::Manager;

// Der Lese-Token für die privaten GitHub-Release-Assets wird nur zur
// Build-Zeit eingebettet (`option_env!`, bewusst NICHT `env!` - `env!` würde
// jeden lokalen `cargo check`/Dev-Build ohne gesetzte Variable hart brechen).
// Ohne gesetzten `GH_RELEASE_TOKEN` (z.B. bei einem normalen lokalen
// Dev-Build) bleibt der Header einfach weg - eine Update-Prüfung schlägt
// dann erwartungsgemäß fehl (401/404 auf das private Repo), die App stürzt
// dabei aber nicht ab. Nur der offizielle CI-Release-Build
// (.github/workflows/release.yml) setzt diese Variable wirklich.
fn build_updater_plugin<R: tauri::Runtime>() -> tauri::plugin::TauriPlugin<R, tauri_plugin_updater::Config> {
    let mut builder = tauri_plugin_updater::Builder::new();
    if let Some(token) = option_env!("GH_RELEASE_TOKEN") {
        builder = builder
            .header("Authorization", format!("Bearer {token}"))
            .expect("GH_RELEASE_TOKEN must be a valid HTTP header value");
    }
    builder.build()
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_opener::init())
        .plugin(tauri_plugin_dialog::init())
        .plugin(build_updater_plugin())
        .plugin(tauri_plugin_process::init())
        .setup(|app| {
            let app_data_dir = app
                .path()
                .app_data_dir()
                .expect("app data dir should be resolvable");
            let settings_db =
                settings::init_db(&app_data_dir).expect("failed to initialize settings database");
            app.manage(AppState {
                mysql_pool: Default::default(),
                settings_db: std::sync::Mutex::new(settings_db),
            });
            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            commands::test_ssh_connection,
            commands::test_stored_ssh,
            commands::read_mob_drop_file,
            commands::write_mob_drop_file,
            commands::sanitize_mob_drop_group_name,
            commands::read_special_item_group_file,
            commands::write_special_item_group_file,
            commands::sanitize_special_item_group_name,
            commands::read_cube_file,
            commands::write_cube_file,
            commands::read_local_text_file,
            commands::parse_mob_drop_text,
            commands::write_local_mob_drop_file,
            commands::export_text_file,
            commands::convert_image_to_tga,
            commands::preview_image_file,
            commands::run_server_command,
            commands::list_build_targets,
            commands::sync_build_source,
            commands::run_source_build,
            commands::run_deploy,
            commands::run_rollback,
            commands::list_deploy_history,
            commands::test_mysql_connection,
            commands::store_credential,
            commands::get_credential,
            commands::delete_credential,
            commands::load_gr2_model,
            commands::connect_mysql,
            commands::is_mysql_connected,
            commands::get_database_stats,
            commands::list_event_flags,
            commands::set_event_flag,
            commands::delete_event_flag,
            commands::list_accounts,
            commands::count_accounts,
            commands::create_account,
            commands::read_common_drop_file,
            commands::write_common_drop_file,
            commands::read_etc_drop_file,
            commands::write_etc_drop_file,
            commands::read_drop_item_group_file,
            commands::write_drop_item_group_file,
            commands::get_item_internal_name,
            commands::find_item_by_internal_name,
            commands::reset_account_password,
            commands::ban_account,
            commands::unban_account,
            commands::list_account_bans,
            commands::process_due_bans,
            commands::adjust_player_gold,
            commands::adjust_account_numeric_column,
            commands::list_databases,
            commands::list_tables,
            commands::get_table_columns,
            commands::get_table_rows,
            commands::search_table_rows,
            commands::list_shops,
            commands::get_shop_items,
            commands::search_items,
            commands::browse_items,
            commands::browse_item_proto,
            commands::update_shop_item_count,
            commands::add_shop_item,
            commands::remove_shop_item,
            commands::sync_shop_stack_sizes,
            commands::delete_shop,
            commands::rename_shop,
            commands::create_shop,
            commands::get_setting,
            commands::set_setting,
            commands::check_client_path,
            commands::locate_npc_model,
            commands::get_shop_default_max,
            commands::set_shop_default_max,
            commands::get_item_icon,
            commands::validate_item_editor_setup,
            commands::item_vnum_exists,
            commands::next_free_item_vnum,
            commands::get_item_proto,
            commands::create_item_proto,
            commands::update_item_proto,
            commands::delete_item_proto,
            commands::write_item_icon,
            commands::pack_item_icons,
            commands::write_item_model,
            commands::pack_item_models,
            commands::write_item_list_entry,
            commands::get_item_desc,
            commands::write_item_desc,
            commands::regenerate_item_proto,
            commands::deploy_item_proto,
            commands::scan_module,
            commands::scan_icon_folder,
            commands::import_weapon_model,
            commands::next_free_shape_index,
            commands::import_armor_model,
            commands::pack_folder,
            commands::import_effect_bundle,
            commands::pack_item_effects,
            commands::record_import_batch,
            commands::list_import_batches,
            commands::remove_single_item,
            commands::rollback_created_item,
            commands::undo_import_batch,
            commands::get_refine_chain,
            commands::get_refine_recipe,
            commands::save_refine_recipe,
            commands::delete_refine_recipe,
            commands::set_item_refine_link,
            commands::find_refine_shop_sources,
            commands::search_mobs,
            commands::browse_mobs,
            commands::list_quest_files,
            commands::read_quest_file,
            commands::write_quest_file,
            commands::create_quest_file,
            commands::delete_quest_file,
            commands::sanitize_quest_identifier,
            commands::list_remote_dir,
            commands::restore_remote_backup,
            commands::diff_remote_backup,
            commands::search_quest_files,
            commands::get_server_resource_usage,
            commands::get_server_overview,
            commands::notify_webhook_message,
            commands::send_test_webhook,
            commands::create_database_backup,
            commands::restore_database_backup,
            commands::delete_database_backup,
            commands::list_item_icon_files,
            commands::load_icon_file,
            commands::get_table_row,
            commands::update_table_row,
            commands::insert_table_row,
            commands::delete_table_row,
            commands::read_regen_file,
            commands::write_regen_file,
            commands::list_client_map_folders,
            commands::get_regen_map_image,
            commands::list_locale_namespaces,
            commands::read_locale_namespace,
            commands::write_locale_namespace,
            commands::create_locale_namespace,
            commands::sanitize_locale_namespace,
            commands::scan_system_package,
            commands::find_system_target,
            commands::find_system_targets_batch,
            commands::read_system_target_file,
            commands::resolve_system_insertion,
            commands::apply_system_install,
            commands::list_system_installs,
            commands::undo_system_install,
            commands::list_broadcast_messages,
            commands::create_broadcast_message,
            commands::update_broadcast_message,
            commands::set_broadcast_message_enabled,
            commands::delete_broadcast_message,
            commands::get_weather_state,
            commands::set_weather_state,
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
