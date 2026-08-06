use serde::{Deserialize, Serialize};
use sqlx::{MySqlPool, Row};
use std::collections::HashSet;

// Column set verified against Mysql2Proto's IProto enum, but the *types*
// below are verified against this server's real `DESCRIBE`/information_schema
// output (2026-07-29) - Mysql2Proto's C struct types (BYTE/DWORD/LONG) don't
// necessarily match how this specific DB schema declared each column, and
// several didn't: `type`/`subtype`/`weight`/`size`/`magic_pct`/`limittype0-1`/
// `applytype0-3`/`specular`/`socket_pct` are SIGNED tinyint here (not
// unsigned - they were u8, sqlx rejects that mismatch and every read of
// those columns silently became 0 via `.unwrap_or_default()`), and
// `antiflag`/`flag`/`wearflag`/`gold` are SIGNED int (not unsigned - were
// u32, same silent-zero failure). This is exactly why editing an existing
// item showed empty/default values instead of the real ones. `immuneflag`
// is not an integer at all here - it's a MySQL `SET('PARA','CURSE','STUN',
// 'SLEEP','SLOW','POISON','TERROR')`, a completely different storage
// mechanism from the `EImmuneFlags` C++ enum bit values - see the
// `immuneflag_*` helpers below.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ItemProtoInput {
    pub vnum: u32,
    pub vnum_range: u32,
    pub name: String,
    pub locale_name: String,
    pub r#type: i8,
    pub subtype: i8,
    pub weight: i8,
    pub size: i8,
    pub antiflag: i32,
    pub flag: i32,
    pub wearflag: i32,
    /// Bitmask in Rust/TS for a consistent checkbox UI, even though the DB
    /// stores it as a `SET(...)` of names - converted at the DB boundary,
    /// see `immuneflag_set_to_bits`/`immuneflag_bits_to_set`.
    pub immuneflag: u32,
    /// NPC sell price (what a player pays to buy it from a shop NPC).
    pub gold: i32,
    /// NPC buy price (what a player receives selling it to a shop NPC).
    pub shop_buy_price: u32,
    pub refined_vnum: u32,
    pub refine_set: u16,
    pub magic_pct: i8,
    pub limittype0: i8,
    pub limitvalue0: i32,
    pub limittype1: i8,
    pub limitvalue1: i32,
    pub applytype0: i8,
    pub applyvalue0: i32,
    pub applytype1: i8,
    pub applyvalue1: i32,
    pub applytype2: i8,
    pub applyvalue2: i32,
    pub applytype3: i8,
    pub applyvalue3: i32,
    pub value0: i32,
    pub value1: i32,
    pub value2: i32,
    pub value3: i32,
    pub value4: i32,
    pub value5: i32,
    pub socket0: i8,
    pub socket1: i8,
    pub socket2: i8,
    pub socket3: i8,
    pub socket4: i8,
    pub socket5: i8,
    pub specular: i8,
    pub socket_pct: i8,
    pub addon_type: i32,
}

// item_proto.immuneflag on this server is `SET('PARA','CURSE','STUN',
// 'SLEEP','SLOW','POISON','TERROR')` - a different vocabulary from the
// server code's `EImmuneFlags` enum (which has FALL/REFLECT instead of
// PARA/SLEEP), so it isn't a mirror of that enum, just this DB schema's own
// simplified set. Bit order below matches the SET's declaration order,
// which is what determines MySQL's own internal bit assignment.
const IMMUNE_SET_MEMBERS: [(&str, u32); 7] = [
    ("PARA", 1 << 0),
    ("CURSE", 1 << 1),
    ("STUN", 1 << 2),
    ("SLEEP", 1 << 3),
    ("SLOW", 1 << 4),
    ("POISON", 1 << 5),
    ("TERROR", 1 << 6),
];

fn immuneflag_set_to_bits(set_value: &str) -> u32 {
    let mut bits = 0u32;
    for member in set_value.split(',').map(str::trim) {
        if let Some((_, bit)) = IMMUNE_SET_MEMBERS.iter().find(|(name, _)| *name == member) {
            bits |= bit;
        }
    }
    bits
}

fn immuneflag_bits_to_set(bits: u32) -> String {
    IMMUNE_SET_MEMBERS
        .iter()
        .filter(|(_, bit)| bits & bit != 0)
        .map(|(name, _)| *name)
        .collect::<Vec<_>>()
        .join(",")
}

pub type ItemProtoFull = ItemProtoInput;

// item_proto.name/.locale_name are varbinary holding Windows-1252-encoded
// text, same as shop.rs's decode_name - kept local since db/shop.rs's
// version isn't exported.
fn decode_name(bytes: &[u8]) -> String {
    let trimmed = bytes.split(|&b| b == 0).next().unwrap_or(bytes);
    let (text, _, _) = encoding_rs::WINDOWS_1252.decode(trimmed);
    text.trim().to_string()
}

pub async fn vnum_exists(pool: &MySqlPool, vnum: u32) -> Result<bool, String> {
    let row = sqlx::query("SELECT 1 FROM player.item_proto WHERE vnum = ? LIMIT 1")
        .bind(vnum)
        .fetch_optional(pool)
        .await
        .map_err(|e| e.to_string())?;
    Ok(row.is_some())
}

/// Scans upward from `range_start` for the first vnum with no item_proto row.
pub async fn next_free_vnum(pool: &MySqlPool, range_start: u32) -> Result<u32, String> {
    let rows = sqlx::query("SELECT vnum FROM player.item_proto WHERE vnum >= ? ORDER BY vnum")
        .bind(range_start)
        .fetch_all(pool)
        .await
        .map_err(|e| e.to_string())?;
    let taken: HashSet<u32> = rows
        .into_iter()
        .map(|row| row.try_get::<u32, _>("vnum").unwrap_or_default())
        .collect();
    let mut candidate = range_start;
    while taken.contains(&candidate) {
        candidate += 1;
    }
    Ok(candidate)
}

pub async fn create_item_proto(pool: &MySqlPool, item: &ItemProtoInput) -> Result<(), String> {
    if vnum_exists(pool, item.vnum).await? {
        return Err(format!("Item mit vnum {} existiert bereits.", item.vnum));
    }

    sqlx::query(
        "INSERT INTO player.item_proto (\
         vnum, vnum_range, name, locale_name, type, subtype, weight, size, \
         antiflag, flag, wearflag, immuneflag, gold, shop_buy_price, refined_vnum, refine_set, magic_pct, \
         limittype0, limitvalue0, limittype1, limitvalue1, \
         applytype0, applyvalue0, applytype1, applyvalue1, applytype2, applyvalue2, applytype3, applyvalue3, \
         value0, value1, value2, value3, value4, value5, \
         socket0, socket1, socket2, socket3, socket4, socket5, \
         specular, socket_pct, addon_type\
         ) VALUES (\
         ?, ?, ?, ?, ?, ?, ?, ?, \
         ?, ?, ?, ?, ?, ?, ?, ?, ?, \
         ?, ?, ?, ?, \
         ?, ?, ?, ?, ?, ?, ?, ?, \
         ?, ?, ?, ?, ?, ?, \
         ?, ?, ?, ?, ?, ?, \
         ?, ?, ?)",
    )
    .bind(item.vnum)
    .bind(item.vnum_range)
    .bind(&item.name)
    .bind(&item.locale_name)
    .bind(item.r#type)
    .bind(item.subtype)
    .bind(item.weight)
    .bind(item.size)
    .bind(item.antiflag)
    .bind(item.flag)
    .bind(item.wearflag)
    .bind(immuneflag_bits_to_set(item.immuneflag))
    .bind(item.gold)
    .bind(item.shop_buy_price)
    .bind(item.refined_vnum)
    .bind(item.refine_set)
    .bind(item.magic_pct)
    .bind(item.limittype0)
    .bind(item.limitvalue0)
    .bind(item.limittype1)
    .bind(item.limitvalue1)
    .bind(item.applytype0)
    .bind(item.applyvalue0)
    .bind(item.applytype1)
    .bind(item.applyvalue1)
    .bind(item.applytype2)
    .bind(item.applyvalue2)
    .bind(item.applytype3)
    .bind(item.applyvalue3)
    .bind(item.value0)
    .bind(item.value1)
    .bind(item.value2)
    .bind(item.value3)
    .bind(item.value4)
    .bind(item.value5)
    .bind(item.socket0)
    .bind(item.socket1)
    .bind(item.socket2)
    .bind(item.socket3)
    .bind(item.socket4)
    .bind(item.socket5)
    .bind(item.specular)
    .bind(item.socket_pct)
    .bind(item.addon_type)
    .execute(pool)
    .await
    .map_err(|e| e.to_string())?;

    Ok(())
}

pub async fn update_item_proto(pool: &MySqlPool, item: &ItemProtoInput) -> Result<(), String> {
    let result = sqlx::query(
        "UPDATE player.item_proto SET \
         vnum_range = ?, name = ?, locale_name = ?, type = ?, subtype = ?, weight = ?, size = ?, \
         antiflag = ?, flag = ?, wearflag = ?, immuneflag = ?, gold = ?, shop_buy_price = ?, \
         refined_vnum = ?, refine_set = ?, magic_pct = ?, \
         limittype0 = ?, limitvalue0 = ?, limittype1 = ?, limitvalue1 = ?, \
         applytype0 = ?, applyvalue0 = ?, applytype1 = ?, applyvalue1 = ?, \
         applytype2 = ?, applyvalue2 = ?, applytype3 = ?, applyvalue3 = ?, \
         value0 = ?, value1 = ?, value2 = ?, value3 = ?, value4 = ?, value5 = ?, \
         socket0 = ?, socket1 = ?, socket2 = ?, socket3 = ?, socket4 = ?, socket5 = ?, \
         specular = ?, socket_pct = ?, addon_type = ? \
         WHERE vnum = ?",
    )
    .bind(item.vnum_range)
    .bind(&item.name)
    .bind(&item.locale_name)
    .bind(item.r#type)
    .bind(item.subtype)
    .bind(item.weight)
    .bind(item.size)
    .bind(item.antiflag)
    .bind(item.flag)
    .bind(item.wearflag)
    .bind(immuneflag_bits_to_set(item.immuneflag))
    .bind(item.gold)
    .bind(item.shop_buy_price)
    .bind(item.refined_vnum)
    .bind(item.refine_set)
    .bind(item.magic_pct)
    .bind(item.limittype0)
    .bind(item.limitvalue0)
    .bind(item.limittype1)
    .bind(item.limitvalue1)
    .bind(item.applytype0)
    .bind(item.applyvalue0)
    .bind(item.applytype1)
    .bind(item.applyvalue1)
    .bind(item.applytype2)
    .bind(item.applyvalue2)
    .bind(item.applytype3)
    .bind(item.applyvalue3)
    .bind(item.value0)
    .bind(item.value1)
    .bind(item.value2)
    .bind(item.value3)
    .bind(item.value4)
    .bind(item.value5)
    .bind(item.socket0)
    .bind(item.socket1)
    .bind(item.socket2)
    .bind(item.socket3)
    .bind(item.socket4)
    .bind(item.socket5)
    .bind(item.specular)
    .bind(item.socket_pct)
    .bind(item.addon_type)
    .bind(item.vnum)
    .execute(pool)
    .await
    .map_err(|e| e.to_string())?;

    if result.rows_affected() == 0 {
        return Err(format!("Item mit vnum {} existiert nicht.", item.vnum));
    }
    Ok(())
}

/// Highest `value3` used by any armor row - `value3` doubles as the
/// female-race `.msm` `ShapeIndex` for body armor (see `msm.rs`), so a
/// newly imported armor piece must pick an index above this to avoid
/// silently reusing (and thus visually colliding with) an existing item's
/// body shape.
pub async fn max_armor_value3(pool: &MySqlPool) -> Result<u32, String> {
    let row = sqlx::query("SELECT MAX(value3) AS m FROM player.item_proto WHERE type = 2")
        .fetch_one(pool)
        .await
        .map_err(|e| e.to_string())?;
    Ok(row.try_get::<Option<u32>, _>("m").unwrap_or_default().unwrap_or(0))
}

pub async fn delete_item_proto(pool: &MySqlPool, vnum: u32) -> Result<(), String> {
    sqlx::query("DELETE FROM player.item_proto WHERE vnum = ?")
        .bind(vnum)
        .execute(pool)
        .await
        .map_err(|e| e.to_string())?;
    Ok(())
}

/// Fetches a full item_proto row, used to pre-fill the "create from
/// reference item" flow in the Item Editor.
pub async fn get_item_proto(pool: &MySqlPool, vnum: u32) -> Result<Option<ItemProtoFull>, String> {
    let row = sqlx::query(
        "SELECT vnum, vnum_range, name, locale_name, type, subtype, weight, size, \
         antiflag, flag, wearflag, immuneflag, gold, shop_buy_price, refined_vnum, refine_set, magic_pct, \
         limittype0, limitvalue0, limittype1, limitvalue1, \
         applytype0, applyvalue0, applytype1, applyvalue1, applytype2, applyvalue2, applytype3, applyvalue3, \
         value0, value1, value2, value3, value4, value5, \
         socket0, socket1, socket2, socket3, socket4, socket5, \
         specular, socket_pct, addon_type \
         FROM player.item_proto WHERE vnum = ?",
    )
    .bind(vnum)
    .fetch_optional(pool)
    .await
    .map_err(|e| e.to_string())?;

    let Some(row) = row else {
        return Ok(None);
    };

    let name_raw: Vec<u8> = row.try_get("name").unwrap_or_default();
    let locale_name_raw: Vec<u8> = row.try_get("locale_name").unwrap_or_default();

    Ok(Some(ItemProtoFull {
        vnum: row.try_get("vnum").unwrap_or_default(),
        vnum_range: row.try_get("vnum_range").unwrap_or_default(),
        name: decode_name(&name_raw),
        locale_name: decode_name(&locale_name_raw),
        r#type: row.try_get("type").unwrap_or_default(),
        subtype: row.try_get("subtype").unwrap_or_default(),
        weight: row.try_get("weight").unwrap_or_default(),
        size: row.try_get("size").unwrap_or_default(),
        antiflag: row.try_get("antiflag").unwrap_or_default(),
        flag: row.try_get("flag").unwrap_or_default(),
        wearflag: row.try_get("wearflag").unwrap_or_default(),
        immuneflag: immuneflag_set_to_bits(
            &row.try_get::<String, _>("immuneflag").unwrap_or_default(),
        ),
        gold: row.try_get("gold").unwrap_or_default(),
        shop_buy_price: row.try_get("shop_buy_price").unwrap_or_default(),
        refined_vnum: row.try_get("refined_vnum").unwrap_or_default(),
        refine_set: row.try_get("refine_set").unwrap_or_default(),
        magic_pct: row.try_get("magic_pct").unwrap_or_default(),
        limittype0: row.try_get("limittype0").unwrap_or_default(),
        limitvalue0: row.try_get("limitvalue0").unwrap_or_default(),
        limittype1: row.try_get("limittype1").unwrap_or_default(),
        limitvalue1: row.try_get("limitvalue1").unwrap_or_default(),
        applytype0: row.try_get("applytype0").unwrap_or_default(),
        applyvalue0: row.try_get("applyvalue0").unwrap_or_default(),
        applytype1: row.try_get("applytype1").unwrap_or_default(),
        applyvalue1: row.try_get("applyvalue1").unwrap_or_default(),
        applytype2: row.try_get("applytype2").unwrap_or_default(),
        applyvalue2: row.try_get("applyvalue2").unwrap_or_default(),
        applytype3: row.try_get("applytype3").unwrap_or_default(),
        applyvalue3: row.try_get("applyvalue3").unwrap_or_default(),
        value0: row.try_get("value0").unwrap_or_default(),
        value1: row.try_get("value1").unwrap_or_default(),
        value2: row.try_get("value2").unwrap_or_default(),
        value3: row.try_get("value3").unwrap_or_default(),
        value4: row.try_get("value4").unwrap_or_default(),
        value5: row.try_get("value5").unwrap_or_default(),
        socket0: row.try_get("socket0").unwrap_or_default(),
        socket1: row.try_get("socket1").unwrap_or_default(),
        socket2: row.try_get("socket2").unwrap_or_default(),
        socket3: row.try_get("socket3").unwrap_or_default(),
        socket4: row.try_get("socket4").unwrap_or_default(),
        socket5: row.try_get("socket5").unwrap_or_default(),
        specular: row.try_get("specular").unwrap_or_default(),
        socket_pct: row.try_get("socket_pct").unwrap_or_default(),
        addon_type: row.try_get("addon_type").unwrap_or_default(),
    }))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn immuneflag_set_round_trips() {
        assert_eq!(immuneflag_set_to_bits(""), 0);
        let bits = immuneflag_set_to_bits("STUN,POISON");
        assert_eq!(bits, (1 << 2) | (1 << 5));
        assert_eq!(immuneflag_bits_to_set(bits), "STUN,POISON");
    }

    // Connects with this machine's stored M2Manager credentials against the
    // real dev DB - same pattern as icons.rs's tests against the real
    // client. Regression test for the 2026-07-29 bug: editing an existing
    // item (vnum 3219) showed default/zero values because several columns
    // are SIGNED in this schema while the Rust struct requested unsigned
    // types, which sqlx silently turned into decode failures swallowed by
    // `.unwrap_or_default()`.
    #[tokio::test]
    async fn get_item_proto_reads_real_nonzero_values() {
        let conn = rusqlite::Connection::open(
            r"C:\Users\DevSteven\AppData\Roaming\com.m2manager.app\m2manager.sqlite",
        )
        .expect("open settings db");
        let get = |key: &str| -> Option<String> {
            conn.query_row("SELECT value FROM paths WHERE key = ?1", [key], |r| r.get(0))
                .ok()
        };
        let host = get("mysql_host").expect("mysql_host not configured on this machine");
        let port: u16 = get("mysql_port")
            .unwrap_or_else(|| "3306".into())
            .parse()
            .unwrap();
        let user = get("mysql_username").expect("mysql_username not configured");
        let password = keyring::Entry::new("m2manager", "mysql_password")
            .unwrap()
            .get_password()
            .expect("mysql_password credential not stored");

        let url = format!("mysql://{user}:{password}@{host}:{port}/player");
        let pool = sqlx::mysql::MySqlPoolOptions::new()
            .max_connections(1)
            .connect(&url)
            .await
            .expect("connect to dev DB");

        let item = get_item_proto(&pool, 3219)
            .await
            .expect("query failed")
            .expect("vnum 3219 should exist on the dev server");

        assert_ne!(item.r#type, 0, "type decoded as 0 - signed/unsigned mismatch regressed");
        assert!(!item.name.is_empty(), "name should not be empty for a real item");
    }
}
