mod commands;
mod credentials;
mod db;
mod gr2;
mod settings;
mod ssh;
mod state;

use state::AppState;
use tauri::Manager;

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_opener::init())
        .plugin(tauri_plugin_dialog::init())
        .manage(AppState::default())
        .setup(|app| {
            let app_data_dir = app
                .path()
                .app_data_dir()
                .expect("app data dir should be resolvable");
            settings::init_db(&app_data_dir).expect("failed to initialize settings database");
            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            commands::test_ssh_connection,
            commands::test_mysql_connection,
            commands::store_credential,
            commands::get_credential,
            commands::delete_credential,
            commands::load_gr2_model,
            commands::connect_mysql,
            commands::list_shops,
            commands::get_shop_items,
            commands::search_items,
            commands::update_shop_item_count,
            commands::add_shop_item,
            commands::remove_shop_item,
            commands::delete_shop,
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
