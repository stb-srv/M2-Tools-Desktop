//! Gilden-Verwaltung. Search/browse/edit reuse the generic DB-Explorer
//! commands directly from the frontend (`search_table_rows`,
//! `GenericRowEditor`) - only the disband cascade needs a dedicated command.

use crate::db::guild;
use crate::state::AppState;
use tauri::State;

use super::support::require_pool;

#[tauri::command]
pub async fn disband_guild(state: State<'_, AppState>, database: String, guild_id: i64) -> Result<(), String> {
    let pool = require_pool(&state).await?;
    guild::disband_guild(&pool, &database, guild_id).await
}
