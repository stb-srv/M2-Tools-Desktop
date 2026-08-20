//! Account-Verwaltung.
//!
//! Eigene, gezielte Commands statt der generischen DB-Explorer-Zeilen-CRUD -
//! Passwörter müssen über MySQLs eigene PASSWORD()-Funktion gesetzt werden,
//! nicht als Klartext-Spaltenwert (siehe db/account.rs), das kann der
//! generische Insert/Update-Pfad nicht leisten.
//!
//! Zeitgesteuerte Sperren (siehe bans.rs): serverseitig gibt es dafür keinen
//! Mechanismus - `status` ist ein freier String, den der Login-Server nur
//! gegen "OK" vergleicht und sonst wörtlich als Fehlermeldung zeigt (siehe
//! db/account.rs::set_status). Die Zeitsteuerung selbst ist rein lokal in
//! M2Manager (SQLite `account_bans`) und greift nur, solange/wann immer die
//! App läuft - kein Server-Cron.

use crate::bans;
use crate::db::account;
use crate::db::explorer;
use crate::state::AppState;
use tauri::State;

use super::support::require_pool;

#[tauri::command]
pub async fn list_accounts(
    state: State<'_, AppState>,
    search: String,
    limit: i64,
    offset: i64,
) -> Result<Vec<account::AccountSummary>, String> {
    let pool = require_pool(&state).await?;
    account::list_accounts(&pool, &search, limit, offset).await
}

#[tauri::command]
pub async fn count_accounts(state: State<'_, AppState>, search: String) -> Result<i64, String> {
    let pool = require_pool(&state).await?;
    account::count_accounts(&pool, &search).await
}

#[tauri::command]
pub async fn create_account(
    state: State<'_, AppState>,
    login: String,
    password: String,
    empire: Option<i8>,
) -> Result<(), String> {
    let pool = require_pool(&state).await?;
    account::create_account(&pool, &login, &password, empire).await
}

#[tauri::command]
pub async fn reset_account_password(
    state: State<'_, AppState>,
    id: i32,
    new_password: String,
) -> Result<(), String> {
    let pool = require_pool(&state).await?;
    account::reset_password(&pool, id, &new_password).await
}

/// Prüft die reale Spaltenbreite von `account.account.status` (nicht
/// geraten) und lehnt eine zu lange Sperr-Nachricht vorher ab, statt sie
/// stillschweigend von MySQL abschneiden zu lassen.
async fn validate_status_length(pool: &sqlx::MySqlPool, value: &str) -> Result<(), String> {
    let columns = explorer::get_columns(pool, "account", "account").await?;
    let Some(status_col) = columns.iter().find(|c| c.name == "status") else {
        return Ok(());
    };
    if let Some(max) = status_col
        .data_type
        .split('(')
        .nth(1)
        .and_then(|s| s.split(')').next())
        .and_then(|s| s.parse::<usize>().ok())
    {
        if value.chars().count() > max {
            return Err(format!(
                "Nachricht ist zu lang ({} Zeichen, die Spalte erlaubt maximal {}).",
                value.chars().count(),
                max
            ));
        }
    }
    Ok(())
}

#[tauri::command]
pub async fn ban_account(
    state: State<'_, AppState>,
    account_id: i32,
    login: String,
    message: String,
    days: Option<i64>,
) -> Result<(), String> {
    if message.trim().is_empty() {
        return Err("Sperr-Nachricht darf nicht leer sein.".to_string());
    }
    let pool = require_pool(&state).await?;
    validate_status_length(&pool, &message).await?;
    account::set_status(&pool, account_id, &message).await?;
    let conn = state.settings_db.lock().map_err(|e| e.to_string())?;
    bans::create_ban(&conn, account_id as i64, &login, &message, days)?;
    Ok(())
}

#[tauri::command]
pub async fn unban_account(state: State<'_, AppState>, account_id: i32, ban_id: Option<i64>) -> Result<(), String> {
    let pool = require_pool(&state).await?;
    account::set_status(&pool, account_id, "OK").await?;
    if let Some(ban_id) = ban_id {
        let conn = state.settings_db.lock().map_err(|e| e.to_string())?;
        bans::deactivate_ban(&conn, ban_id)?;
    }
    Ok(())
}

#[tauri::command]
pub fn list_account_bans(state: State<'_, AppState>) -> Result<Vec<bans::BanRecord>, String> {
    let conn = state.settings_db.lock().map_err(|e| e.to_string())?;
    bans::list_bans(&conn)
}

/// Setzt jede fällige Sperre automatisch zurück - aufgerufen beim Öffnen des
/// Account-Managers (siehe AccountManager.tsx), nicht per Hintergrunddienst.
#[tauri::command]
pub async fn process_due_bans(state: State<'_, AppState>) -> Result<u32, String> {
    let pool = require_pool(&state).await?;
    let due = {
        let conn = state.settings_db.lock().map_err(|e| e.to_string())?;
        bans::due_bans(&conn)?
    };
    let count = due.len() as u32;
    for record in due {
        account::set_status(&pool, record.account_id as i32, "OK").await?;
        let conn = state.settings_db.lock().map_err(|e| e.to_string())?;
        bans::deactivate_ban(&conn, record.id)?;
    }
    Ok(count)
}

// ---- Guthaben anpassen (Yang / unverifizierte Konto-Zusatzwährung) ----

#[tauri::command]
pub async fn adjust_player_gold(state: State<'_, AppState>, player_id: i32, delta: i64) -> Result<i64, String> {
    let pool = require_pool(&state).await?;
    account::adjust_player_gold(&pool, player_id, delta).await
}

/// `column` wird hier - nicht erst in db/account.rs - gegen eine frisch
/// geholte, echte Spaltenliste von `account.account` geprüft (Name UND
/// numerischer Typ), bevor sie in SQL interpoliert wird.
#[tauri::command]
pub async fn adjust_account_numeric_column(
    state: State<'_, AppState>,
    account_id: i32,
    column: String,
    delta: i64,
) -> Result<i64, String> {
    let pool = require_pool(&state).await?;
    let columns = explorer::get_columns(&pool, "account", "account").await?;
    const NUMERIC_TYPE_PREFIXES: &[&str] = &["int", "tinyint", "smallint", "mediumint", "bigint", "decimal", "float", "double"];
    let is_valid = columns.iter().any(|c| {
        c.name == column
            && !c.is_primary_key
            && NUMERIC_TYPE_PREFIXES.iter().any(|p| c.data_type.to_lowercase().starts_with(p))
    });
    if !is_valid {
        return Err(format!("Spalte '{column}' existiert nicht oder ist nicht numerisch."));
    }
    account::adjust_account_numeric_column(&pool, account_id, &column, delta).await
}
