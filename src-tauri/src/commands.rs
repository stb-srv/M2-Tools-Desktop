use crate::credentials;
use crate::db::mysql::{self, MysqlConfig};
use crate::ssh::{self, SshConfig};

#[tauri::command]
pub async fn test_ssh_connection(config: SshConfig, password: String) -> Result<(), String> {
    ssh::test_connection(&config, &password).await
}

#[tauri::command]
pub async fn test_mysql_connection(config: MysqlConfig, password: String) -> Result<(), String> {
    mysql::test_connection(&config, &password).await
}

#[tauri::command]
pub fn store_credential(account: String, secret: String) -> Result<(), String> {
    credentials::store_secret(&account, &secret)
}

#[tauri::command]
pub fn get_credential(account: String) -> Result<String, String> {
    credentials::get_secret(&account)
}

#[tauri::command]
pub fn delete_credential(account: String) -> Result<(), String> {
    credentials::delete_secret(&account)
}
