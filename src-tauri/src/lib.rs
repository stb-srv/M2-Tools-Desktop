// MSVC's linker prints informational "creating library/object file" notes
// when building the cdylib's import lib - harmless, just noisy.
#![allow(linker_messages)]

mod backups;
mod commands;
mod credentials;
mod db;
mod gr2;
mod icons;
mod imageconv;
mod import_history;
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
mod resources;
mod webhook;
mod settings;
mod ssh;
mod state;
mod textures;

use state::AppState;
use tauri::Manager;

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_opener::init())
        .plugin(tauri_plugin_dialog::init())
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
            commands::read_local_text_file,
            commands::parse_mob_drop_text,
            commands::write_local_mob_drop_file,
            commands::convert_image_to_tga,
            commands::preview_image_file,
            commands::run_server_command,
            commands::test_mysql_connection,
            commands::store_credential,
            commands::get_credential,
            commands::delete_credential,
            commands::load_gr2_model,
            commands::connect_mysql,
            commands::is_mysql_connected,
            commands::get_database_stats,
            commands::list_databases,
            commands::list_tables,
            commands::get_table_columns,
            commands::get_table_rows,
            commands::search_table_rows,
            commands::list_shops,
            commands::get_shop_items,
            commands::search_items,
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
            commands::regenerate_item_proto,
            commands::deploy_item_proto,
            commands::scan_module,
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
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
