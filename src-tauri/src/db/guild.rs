//! Gilden-Verwaltung (guild/guild_member/guild_war_reservation/...).
//!
//! Table/column layout deliberately NOT hardcoded beyond what's verified from
//! the real server source below - search/browse/edit all go through the
//! existing schema-agnostic `db/explorer.rs` commands (`search_table_rows`,
//! `GenericRowEditor` on the frontend), same as Mob-Proto-Editor/Account-
//! Manager. Only the disband cascade below needs bespoke code, since it's a
//! multi-table transaction the generic single-row CRUD can't express.
//!
//! Real table names and the disband cascade were verified directly against
//! `game-src` (not assumed): `source/db/src/GuildManager.cpp` (SELECT/UPDATE
//! queries against `guild` - columns seen: id, name, ladder_point, win,
//! draw, loss, gold, level, master; `guild_war_reservation`/`guild_war_bet`
//! for wars) and `source/db/src/ClientManagerGuild.cpp::GuildDisband` (the
//! exact 5-statement cascade this module mirrors: delete `guild`, delete
//! `guild_grade`, set a `guild_manage.withdraw_time` quest flag for every
//! member via `quest`, delete `guild_member`, delete `guild_comment` - war
//! reservations/bets are deliberately left untouched, exactly like the real
//! server). Which database these tables live in was NOT live-verified -
//! defaults to "player" (same DB as `player.player`/`item_proto`/`shop` per
//! db/mapping.rs), but the frontend lets the user override it since this is
//! the one unverified assumption here.

use crate::db::explorer;
use sqlx::MySqlPool;

const REQUIRED_TABLES: &[&str] = &["guild", "guild_grade", "guild_member", "guild_comment"];

/// Mirrors `ClientManagerGuild.cpp::GuildDisband` exactly. Validates that
/// `database` is real and actually contains the tables this touches first
/// (same information_schema-validate-before-interpolate pattern used
/// elsewhere in this codebase, e.g. `adjust_account_numeric_column`) since
/// the database name is user-editable, not a compile-time constant.
pub async fn disband_guild(pool: &MySqlPool, database: &str, guild_id: i64) -> Result<(), String> {
    let tables = explorer::list_tables(pool, database).await?;
    let names: Vec<&str> = tables.iter().map(|t| t.name.as_str()).collect();
    for required in REQUIRED_TABLES {
        if !names.contains(required) {
            return Err(format!(
                "Datenbank '{database}' hat keine Tabelle '{required}' - falsche Datenbank gewählt?"
            ));
        }
    }
    let has_quest_table = names.contains(&"quest");

    sqlx::query(&format!("DELETE FROM `{database}`.`guild` WHERE id = ?"))
        .bind(guild_id)
        .execute(pool)
        .await
        .map_err(|e| e.to_string())?;

    sqlx::query(&format!(
        "DELETE FROM `{database}`.`guild_grade` WHERE guild_id = ?"
    ))
    .bind(guild_id)
    .execute(pool)
    .await
    .map_err(|e| e.to_string())?;

    // Best-effort: the withdraw-cooldown quest flag is a nice-to-have (so a
    // former member who logs back in still gets the real server's rejoin
    // cooldown), not essential to a successful disband - skip it quietly if
    // this core's `quest` table isn't in the same database.
    if has_quest_table {
        sqlx::query(&format!(
            "REPLACE INTO `{database}`.`quest` (dwPID, szName, szState, lValue) \
             SELECT pid, 'guild_manage', 'withdraw_time', UNIX_TIMESTAMP() \
             FROM `{database}`.`guild_member` WHERE guild_id = ?"
        ))
        .bind(guild_id)
        .execute(pool)
        .await
        .map_err(|e| e.to_string())?;
    }

    sqlx::query(&format!(
        "DELETE FROM `{database}`.`guild_member` WHERE guild_id = ?"
    ))
    .bind(guild_id)
    .execute(pool)
    .await
    .map_err(|e| e.to_string())?;

    sqlx::query(&format!(
        "DELETE FROM `{database}`.`guild_comment` WHERE guild_id = ?"
    ))
    .bind(guild_id)
    .execute(pool)
    .await
    .map_err(|e| e.to_string())?;

    Ok(())
}
