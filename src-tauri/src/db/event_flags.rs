use serde::Serialize;
use sqlx::{MySqlPool, Row};

// Server-Events (Doppel-Drop, Doppel-EXP, saisonale Sonder-Drops, ...) -
// verified against the real server source, not assumed:
//
// - The mechanism is a generic `map<string,int>` on the game server's
//   `CQuestManager` (`GetEventFlag`/`SetEventFlag`, `questmanager.cpp`) -
//   there is no fixed list of valid flag names anywhere in the source, any
//   string works.
// - Persisted in the SAME `quest` table used for per-player quest progress
//   (`player.quest`), distinguished by `dwPID = 0` (global) vs a real
//   player ID. Columns verified live: `dwPID int unsigned`, `szName
//   varchar(32)` (matches `EVENT_FLAG_NAME_MAX_LEN` in the real packet
//   struct), `szState varchar(64)` (unused for event flags, always ''),
//   `lValue int`.
// - **Only loaded into the DB server's memory once, at its own boot**
//   (`ClientManager.cpp`'s `LoadEventFlag()`), then pushed live to every
//   connected game-server process on change. A direct SQL write from this
//   app changes the persisted value but has **zero effect on a running
//   server** until the whole server (db+game+login) is restarted - same
//   restart requirement as `refine_proto`. The *only* way to change a flag
//   live without restarting is the in-game GM command (`eventflag <name>
//   <value>`), which updates memory AND persists to this same table in one
//   step - offered as a copyable convenience alongside the DB-only edit.
// - Two distinct value semantics exist, both verified from source, and
//   mixing them up would show a confusing default:
//   - "Rate" flags (`mob_item`, `mob_exp`, ...): read via a SEPARATE
//     `CHARACTER_MANAGER` field with its OWN default (100 = normal),
//     independent of whether a DB row exists at all. An absent row means
//     "still 100%", not "0%".
//   - Seasonal drop flags (`valentine_drop`, `halloween_drop`, ...): read
//     via `GetDropPerKillPct(min, default, deltaPercent, flagName)`
//     (`item_manager.cpp:645`), which returns `0` (event fully OFF) when
//     the flag is entirely unset - an absent row here really does mean
//     "off", the opposite convention from the rate flags.

#[derive(Debug, Clone, Serialize)]
pub struct EventFlagRow {
    pub name: String,
    pub value: i32,
}

pub async fn list_event_flags(pool: &MySqlPool) -> Result<Vec<EventFlagRow>, String> {
    let rows = sqlx::query("SELECT szName, lValue FROM player.quest WHERE dwPID = 0 ORDER BY szName")
        .fetch_all(pool)
        .await
        .map_err(|e| e.to_string())?;
    rows.into_iter()
        .map(|row| {
            Ok(EventFlagRow {
                name: row.try_get("szName").map_err(|e| e.to_string())?,
                value: row.try_get("lValue").map_err(|e| e.to_string())?,
            })
        })
        .collect()
}

fn validate_name(name: &str) -> Result<(), String> {
    if name.trim().is_empty() {
        return Err("Flag-Name darf nicht leer sein.".to_string());
    }
    // EVENT_FLAG_NAME_MAX_LEN (common/length.h) = 32, matching the real
    // packet struct / szName column width - a longer name would just get
    // silently truncated by the real server, so reject it upfront instead.
    if name.len() > 32 {
        return Err(format!("Flag-Name ist zu lang ({} Zeichen, maximal 32).", name.len()));
    }
    Ok(())
}

/// Upserts a global event flag row (`dwPID = 0`). Only changes what's
/// persisted in the DB - has no effect on any already-running server
/// process, see module doc.
pub async fn set_event_flag(pool: &MySqlPool, name: &str, value: i32) -> Result<(), String> {
    validate_name(name)?;
    sqlx::query("REPLACE INTO player.quest (dwPID, szName, szState, lValue) VALUES (0, ?, '', ?)")
        .bind(name)
        .bind(value)
        .execute(pool)
        .await
        .map_err(|e| e.to_string())?;
    Ok(())
}

/// Removes a global event flag row entirely - for the "rate" family this
/// means "fall back to the server's own hardcoded default (100%)"; for the
/// seasonal-drop family it means "fully off". Both are the real server's
/// own definition of "unset", not something this app invents.
pub async fn delete_event_flag(pool: &MySqlPool, name: &str) -> Result<(), String> {
    sqlx::query("DELETE FROM player.quest WHERE dwPID = 0 AND szName = ?")
        .bind(name)
        .execute(pool)
        .await
        .map_err(|e| e.to_string())?;
    Ok(())
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

    #[tokio::test]
    async fn sets_lists_and_deletes_a_real_throwaway_flag() {
        let pool = real_pool().await;
        let name = format!("m2mgr_test_{}", chrono::Local::now().timestamp() % 100000);

        set_event_flag(&pool, &name, 200).await.expect("set failed");
        let flags = list_event_flags(&pool).await.expect("list failed");
        let found = flags.iter().find(|f| f.name == name).expect("flag not in list");
        assert_eq!(found.value, 200);

        // REPLACE INTO must overwrite, not duplicate.
        set_event_flag(&pool, &name, 300).await.expect("update failed");
        let flags = list_event_flags(&pool).await.expect("list failed");
        assert_eq!(flags.iter().filter(|f| f.name == name).count(), 1);
        assert_eq!(flags.iter().find(|f| f.name == name).unwrap().value, 300);

        delete_event_flag(&pool, &name).await.expect("delete failed");
        let flags = list_event_flags(&pool).await.expect("list failed");
        assert!(!flags.iter().any(|f| f.name == name));
    }

    #[test]
    fn rejects_names_longer_than_the_real_column_width() {
        let too_long = "a".repeat(33);
        assert!(validate_name(&too_long).is_err());
        assert!(validate_name("mob_item").is_ok());
    }
}
