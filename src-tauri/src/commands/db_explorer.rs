//! DB-Explorer: generisches Datenbank/Tabellen-Browsing plus generisches
//! Zeilen-CRUD (Basis für Mob-Proto-Editor und Account-/Player-Verwaltung -
//! die Primärschlüssel-Spalte wird vom Frontend aus `get_table_columns`'
//! `is_primary_key`-Flag automatisch erkannt statt hier geraten zu werden).

use crate::db::explorer::{self, ColumnInfo, TableInfo, TableRows};
use crate::state::AppState;
use tauri::State;

use super::support::require_pool;

#[tauri::command]
pub async fn list_databases(state: State<'_, AppState>) -> Result<Vec<String>, String> {
    let pool = require_pool(&state).await?;
    explorer::list_databases(&pool).await
}

#[tauri::command]
pub async fn list_tables(
    state: State<'_, AppState>,
    database: String,
) -> Result<Vec<TableInfo>, String> {
    let pool = require_pool(&state).await?;
    explorer::list_tables(&pool, &database).await
}

#[tauri::command]
pub async fn get_table_columns(
    state: State<'_, AppState>,
    database: String,
    table: String,
) -> Result<Vec<ColumnInfo>, String> {
    let pool = require_pool(&state).await?;
    explorer::get_columns(&pool, &database, &table).await
}

#[tauri::command]
pub async fn get_table_rows(
    state: State<'_, AppState>,
    database: String,
    table: String,
    limit: i64,
    offset: i64,
) -> Result<TableRows, String> {
    let pool = require_pool(&state).await?;
    explorer::get_rows(&pool, &database, &table, limit, offset).await
}

#[tauri::command]
pub async fn search_table_rows(
    state: State<'_, AppState>,
    database: String,
    table: String,
    column: String,
    query: String,
) -> Result<TableRows, String> {
    let pool = require_pool(&state).await?;
    explorer::search_rows(&pool, &database, &table, &column, &query, 200).await
}

#[tauri::command]
pub async fn get_table_row(
    state: State<'_, AppState>,
    database: String,
    table: String,
    pk_column: String,
    pk_value: String,
) -> Result<Option<TableRows>, String> {
    let pool = require_pool(&state).await?;
    explorer::get_row_by_pk(&pool, &database, &table, &pk_column, &pk_value).await
}

#[tauri::command]
pub async fn update_table_row(
    state: State<'_, AppState>,
    database: String,
    table: String,
    pk_column: String,
    pk_value: String,
    changes: Vec<(String, Option<String>)>,
) -> Result<(), String> {
    let pool = require_pool(&state).await?;
    explorer::update_row(&pool, &database, &table, &pk_column, &pk_value, &changes).await
}

#[tauri::command]
pub async fn insert_table_row(
    state: State<'_, AppState>,
    database: String,
    table: String,
    values: Vec<(String, Option<String>)>,
) -> Result<(), String> {
    let pool = require_pool(&state).await?;
    explorer::insert_row(&pool, &database, &table, &values).await
}

#[tauri::command]
pub async fn delete_table_row(
    state: State<'_, AppState>,
    database: String,
    table: String,
    pk_column: String,
    pk_value: String,
) -> Result<(), String> {
    let pool = require_pool(&state).await?;
    explorer::delete_row(&pool, &database, &table, &pk_column, &pk_value).await
}
