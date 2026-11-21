use crate::known_hosts::{self, KnownHost};

#[tauri::command]
pub async fn list_known_hosts() -> Result<Vec<KnownHost>, String> {
    Ok(known_hosts::list())
}

#[tauri::command]
pub async fn forget_known_host(host: String, port: u16) -> Result<bool, String> {
    Ok(known_hosts::forget(&host, port))
}
