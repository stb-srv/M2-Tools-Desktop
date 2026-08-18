use serde::Serialize;
use sqlx::{MySqlPool, Row};

// account.account schema verified live against the real dev DB
// (information_schema.columns, 2026-08-07): `login`/`password` are NOT NULL
// with no useful default, every other column already has a sensible default
// (`status`='OK', `empire`=0, various `*_expire` datetimes, ...) - so a new
// account only ever needs to supply login/password[/empire], everything
// else can be left to MySQL's own column defaults.
//
// Passwords are a MySQL `PASSWORD()` hash (verified live: real accounts'
// `password` column is exactly 41 chars, `*` + 40 hex chars - MySQL's own
// legacy password hash format), NOT plaintext and NOT any custom cipher -
// confirmed against the real server source (`source/game/src/input_auth.cpp`/
// `db.cpp`: login does `SELECT PASSWORD('<input>'), password ... WHERE
// login=...` then `strcmp`s the two). This is one-way - there is no way to
// "read" an existing password back out, only ever set a new one via the
// same `PASSWORD(?)` SQL function so the server's own comparison keeps
// working. (Live-observed anomaly, not acted on here: one real test account
// had a 60-char `$2a$10$...` bcrypt hash instead - almost certainly created
// by some other, non-server tool - such an account cannot log in via the
// normal path since the server only ever compares against `PASSWORD()`
// output. Not this module's concern to fix, just worth knowing if a
// "can't log in" report ever comes up.)

#[derive(Debug, Clone, Serialize)]
pub struct AccountSummary {
    pub id: i32,
    pub login: String,
    pub email: String,
    pub status: String,
    pub empire: i8,
    pub create_time: String,
    pub last_play: Option<String>,
}

const SELECT_COLUMNS: &str = "id, login, email, status, empire, \
     DATE_FORMAT(create_time, '%Y-%m-%d %H:%i') AS create_time, \
     DATE_FORMAT(last_play, '%Y-%m-%d %H:%i') AS last_play";

fn row_to_summary(row: &sqlx::mysql::MySqlRow) -> Result<AccountSummary, sqlx::Error> {
    Ok(AccountSummary {
        id: row.try_get("id")?,
        login: row.try_get("login")?,
        email: row.try_get("email")?,
        status: row.try_get("status")?,
        empire: row.try_get("empire")?,
        create_time: row.try_get("create_time")?,
        last_play: row.try_get("last_play")?,
    })
}

pub async fn count_accounts(pool: &MySqlPool, search: &str) -> Result<i64, String> {
    let pattern = format!("%{search}%");
    sqlx::query_scalar("SELECT COUNT(*) FROM account.account WHERE login LIKE ?")
        .bind(pattern)
        .fetch_one(pool)
        .await
        .map_err(|e| e.to_string())
}

/// Paginated, optionally filtered by a `login LIKE %search%` match - an
/// empty `search` lists everything. Deliberately never selects `password`
/// (not even the hash) - nothing in the UI needs it, and there's no reason
/// to ever send it to the frontend at all.
pub async fn list_accounts(pool: &MySqlPool, search: &str, limit: i64, offset: i64) -> Result<Vec<AccountSummary>, String> {
    let pattern = format!("%{search}%");
    let sql = format!(
        "SELECT {SELECT_COLUMNS} FROM account.account WHERE login LIKE ? ORDER BY id DESC LIMIT ? OFFSET ?"
    );
    let rows = sqlx::query(&sql)
        .bind(pattern)
        .bind(limit)
        .bind(offset)
        .fetch_all(pool)
        .await
        .map_err(|e| e.to_string())?;
    rows.iter().map(row_to_summary).collect::<Result<Vec<_>, _>>().map_err(|e| e.to_string())
}

pub async fn login_exists(pool: &MySqlPool, login: &str) -> Result<bool, String> {
    let count: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM account.account WHERE login = ?")
        .bind(login)
        .fetch_one(pool)
        .await
        .map_err(|e| e.to_string())?;
    Ok(count > 0)
}

/// Creates a new account. `empire` is left at the column default (0 =
/// "not yet chosen", verified live) when `None` - this server type lets the
/// player pick their empire at first login, not at account creation.
pub async fn create_account(pool: &MySqlPool, login: &str, password: &str, empire: Option<i8>) -> Result<(), String> {
    if login_exists(pool, login).await? {
        return Err(format!("Login '{login}' ist bereits vergeben."));
    }
    match empire {
        Some(empire) => {
            sqlx::query("INSERT INTO account.account (login, password, empire) VALUES (?, PASSWORD(?), ?)")
                .bind(login)
                .bind(password)
                .bind(empire)
                .execute(pool)
                .await
        }
        None => {
            sqlx::query("INSERT INTO account.account (login, password) VALUES (?, PASSWORD(?))")
                .bind(login)
                .bind(password)
                .execute(pool)
                .await
        }
    }
    .map_err(|e| e.to_string())?;
    Ok(())
}

/// Sets a new password for an existing account, hashed the same way the
/// real login check expects (`PASSWORD()`) - there is no way to "read" the
/// old one back out, only ever replace it (see module doc).
pub async fn reset_password(pool: &MySqlPool, id: i32, new_password: &str) -> Result<(), String> {
    let result = sqlx::query("UPDATE account.account SET password = PASSWORD(?) WHERE id = ?")
        .bind(new_password)
        .bind(id)
        .execute(pool)
        .await
        .map_err(|e| e.to_string())?;
    if result.rows_affected() == 0 {
        return Err(format!("Account #{id} nicht gefunden."));
    }
    Ok(())
}

/// Setzt `account.account.status` auf einen beliebigen String. Der
/// Login-Server vergleicht beim Anmelden nur `status == "OK"` (verifiziert:
/// `game-src/source/game/src/db.cpp:367`, `input_db.cpp:121-130`) - jeder
/// andere Wert sperrt das Konto, UND wird wörtlich als Fehlermeldung am
/// Login-Screen angezeigt. Es gibt keine feste Werte-Liste im Quellcode
/// (kein Enum, kein "BLOCK"/"BANNED") und keine serverseitige
/// Zeitsteuerung - das übernimmt `bans.rs` rein lokal in M2Manager.
pub async fn set_status(pool: &MySqlPool, id: i32, status: &str) -> Result<(), String> {
    let result = sqlx::query("UPDATE account.account SET status = ? WHERE id = ?")
        .bind(status)
        .bind(id)
        .execute(pool)
        .await
        .map_err(|e| e.to_string())?;
    if result.rows_affected() == 0 {
        return Err(format!("Account #{id} nicht gefunden."));
    }
    Ok(())
}

/// Yang ist `player.player.gold` (verifiziert: `char.h:342`
/// `long long gold;`, Accessoren `char.h:913-914`), pro Charakter, nicht pro
/// Account. Additiv statt Lesen+Schreiben vom Client aus - atomar und ohne
/// Rundungs-/Race-Risiko bei einem Geldwert.
pub async fn adjust_player_gold(pool: &MySqlPool, player_id: i32, delta: i64) -> Result<i64, String> {
    let result = sqlx::query("UPDATE player.player SET gold = gold + ? WHERE id = ?")
        .bind(delta)
        .bind(player_id)
        .execute(pool)
        .await
        .map_err(|e| e.to_string())?;
    if result.rows_affected() == 0 {
        return Err(format!("Spieler #{player_id} nicht gefunden."));
    }
    let new_value: i64 = sqlx::query_scalar("SELECT gold FROM player.player WHERE id = ?")
        .bind(player_id)
        .fetch_one(pool)
        .await
        .map_err(|e| e.to_string())?;
    Ok(new_value)
}

/// Additive Anpassung einer beliebigen numerischen Konto-Spalte (z.B. eine
/// unverifizierte Zusatzwährung wie "Drachenmünzen"/"Drachenmarken", falls
/// eine solche Spalte auf diesem Core existiert - im Server-Quellcode nicht
/// gefunden, siehe Recherche-Notiz). `column` MUSS vorher vom Aufrufer gegen
/// eine frisch per `get_table_columns` geholte, echte Spaltenliste geprüft
/// worden sein (siehe `commands::adjust_account_numeric_column`) - ein
/// Spaltenname lässt sich nicht parametrisiert binden, ungeprüftes
/// Interpolieren wäre SQL-Injection.
pub async fn adjust_account_numeric_column(pool: &MySqlPool, account_id: i32, column: &str, delta: i64) -> Result<i64, String> {
    let sql = format!("UPDATE account.account SET `{column}` = `{column}` + ? WHERE id = ?");
    let result = sqlx::query(&sql)
        .bind(delta)
        .bind(account_id)
        .execute(pool)
        .await
        .map_err(|e| e.to_string())?;
    if result.rows_affected() == 0 {
        return Err(format!("Account #{account_id} nicht gefunden."));
    }
    let select = format!("SELECT `{column}` FROM account.account WHERE id = ?");
    let new_value: i64 = sqlx::query_scalar(&select)
        .bind(account_id)
        .fetch_one(pool)
        .await
        .map_err(|e| e.to_string())?;
    Ok(new_value)
}

#[cfg(test)]
mod tests {
    use super::*;

    async fn real_pool() -> MySqlPool {
        let conn = rusqlite::Connection::open(
            r"C:\Users\DevSteven\AppData\Roaming\com.m2manager.app\m2manager.sqlite",
        )
        .expect("open settings db");
        let get = |key: &str| -> Option<String> {
            conn.query_row("SELECT value FROM paths WHERE key = ?1", [key], |r| r.get(0)).ok()
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

    // Full round trip against the real dev DB: create a throwaway account,
    // confirm it's listed/searchable, reset its password, then clean up -
    // this is the only way to be sure the PASSWORD()-hashing INSERT/UPDATE
    // actually work against this server's real schema, not just that they
    // compile.
    #[tokio::test]
    async fn creates_lists_and_resets_password_for_a_real_throwaway_account() {
        let pool = real_pool().await;
        let login = format!("m2mgr_test_{}", chrono::Local::now().timestamp());

        assert!(!login_exists(&pool, &login).await.unwrap());
        create_account(&pool, &login, "hunter2", None).await.expect("create failed");
        assert!(login_exists(&pool, &login).await.unwrap());

        let found = list_accounts(&pool, &login, 10, 0).await.expect("list failed");
        assert_eq!(found.len(), 1);
        assert_eq!(found[0].login, login);
        assert_eq!(found[0].status, "OK");
        assert_eq!(found[0].empire, 0);
        let id = found[0].id;

        reset_password(&pool, id, "newpass123").await.expect("reset failed");

        // Duplicate create must be rejected with a clear message, not a raw
        // SQL error.
        let dup = create_account(&pool, &login, "irrelevant", None).await;
        assert!(dup.is_err());
        assert!(dup.unwrap_err().contains("bereits vergeben"));

        sqlx::query("DELETE FROM account.account WHERE login = ?")
            .bind(&login)
            .execute(&pool)
            .await
            .expect("cleanup failed");
    }
}
