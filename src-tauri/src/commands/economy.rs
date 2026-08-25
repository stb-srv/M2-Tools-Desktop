//! Wirtschafts-Dashboard: read-only DB aggregates, no writes. See
//! `db/economy.rs` for the verified column sourcing.

use crate::db::economy::{self, EconomyStats, TopGoldHolder};
use crate::state::AppState;
use tauri::State;

use super::support::require_pool;

#[tauri::command]
pub async fn get_economy_stats(state: State<'_, AppState>) -> Result<EconomyStats, String> {
    let pool = require_pool(&state).await?;
    economy::get_economy_stats(&pool).await
}

#[tauri::command]
pub async fn get_top_gold_holders(state: State<'_, AppState>, limit: i64) -> Result<Vec<TopGoldHolder>, String> {
    let pool = require_pool(&state).await?;
    economy::get_top_gold_holders(&pool, limit).await
}
