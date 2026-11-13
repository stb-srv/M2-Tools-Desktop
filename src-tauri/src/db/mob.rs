//! Kleine Lese-Abfragen auf `player.mob_proto` für die Drop-Werkzeuge.

use serde::Serialize;
use sqlx::{MySqlPool, Row};
use std::collections::HashSet;

fn decode_name(bytes: &[u8]) -> String {
    let trimmed = bytes.split(|&b| b == 0).next().unwrap_or(bytes);
    let (text, _, _) = encoding_rs::WINDOWS_1252.decode(trimmed);
    text.trim().to_string()
}

/// Mob-Ränge in der Reihenfolge des Servers (`EMobRank`).
pub const MOB_RANKS: [&str; 6] = ["PAWN", "S_PAWN", "KNIGHT", "S_KNIGHT", "BOSS", "KING"];

/// `rank` steht je nach Datenbank als Zahl (0-5) oder als Name in der
/// Spalte. Beides wird auf den Namen abgebildet.
pub fn normalize_rank(raw: &str) -> Option<String> {
    let t = raw.trim();
    if let Ok(n) = t.parse::<usize>() {
        return MOB_RANKS.get(n).map(|s| s.to_string());
    }
    let upper = t.to_ascii_uppercase();
    MOB_RANKS.contains(&upper.as_str()).then_some(upper)
}

/// Index 0-3 für die 4 Blöcke von common_drop_item.txt, sonst `None`
/// (BOSS/KING haben keine Common-Drops).
pub fn common_drop_block(rank: &str) -> Option<usize> {
    MOB_RANKS.iter().position(|r| *r == rank).filter(|i| *i < 4)
}

#[derive(Debug, Clone, Serialize)]
pub struct MobBrief {
    pub vnum: u32,
    pub name: String,
    pub rank: Option<String>,
    pub level: Option<i64>,
}

pub async fn get_mob_brief(pool: &MySqlPool, vnum: u32) -> Result<Option<MobBrief>, String> {
    let row = sqlx::query(
        "SELECT vnum, locale_name, CAST(`rank` AS CHAR) AS rank_text, CAST(`level` AS SIGNED) AS lvl \
         FROM player.mob_proto WHERE vnum = ? LIMIT 1",
    )
    .bind(vnum)
    .fetch_optional(pool)
    .await
    .map_err(|e| e.to_string())?;
    Ok(row.map(|row| {
        let name_raw: Vec<u8> = row.try_get("locale_name").unwrap_or_default();
        let rank_text: Option<String> = row
            .try_get::<Option<String>, _>("rank_text")
            .ok()
            .flatten()
            .or_else(|| row.try_get::<Option<Vec<u8>>, _>("rank_text").ok().flatten().map(|b| decode_name(&b)));
        MobBrief {
            vnum: row.try_get("vnum").unwrap_or(vnum),
            name: decode_name(&name_raw),
            rank: rank_text.as_deref().and_then(normalize_rank),
            level: row.try_get::<Option<i64>, _>("lvl").ok().flatten(),
        }
    }))
}

pub async fn existing_mob_vnums(pool: &MySqlPool, vnums: &[u32]) -> Result<HashSet<u32>, String> {
    let mut found = HashSet::new();
    for chunk in vnums.chunks(500) {
        let placeholders = vec!["?"; chunk.len()].join(",");
        let sql = format!("SELECT vnum FROM player.mob_proto WHERE vnum IN ({placeholders})");
        let mut q = sqlx::query(&sql);
        for v in chunk {
            q = q.bind(*v);
        }
        for row in q.fetch_all(pool).await.map_err(|e| e.to_string())? {
            found.insert(row.try_get::<u32, _>("vnum").unwrap_or_default());
        }
    }
    Ok(found)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn ranks() {
        assert_eq!(normalize_rank("0").as_deref(), Some("PAWN"));
        assert_eq!(normalize_rank("3").as_deref(), Some("S_KNIGHT"));
        assert_eq!(normalize_rank("king").as_deref(), Some("KING"));
        assert_eq!(normalize_rank("9"), None);
        assert_eq!(common_drop_block("KNIGHT"), Some(2));
        assert_eq!(common_drop_block("BOSS"), None);
    }
}
