use serde::Serialize;
use sqlx::{MySqlPool, Row};

use super::shop::decode_name;

// Gold lives on `player.player.gold` (verified: char.h:342 `long long gold;`,
// see db/account.rs::adjust_player_gold's doc comment). Shop prices aren't
// their own column on `shop_item` - what an NPC charges is `item_proto.gold`
// (verified: db/item.rs's `gold` field doc, "NPC sell price"), so "average
// shop price" means the average `item_proto.gold` of items actually listed
// somewhere in `player.shop_item`, joined in.
//
// `player.player`'s name column was NOT independently live-verified against
// this server the way `account.account`'s columns were (see account.rs) -
// `get_top_gold_holders` below checks `information_schema.columns` at
// query-time before assuming a `name` column exists, and decodes it as
// Windows-1252 bytes only if it's actually a binary/varbinary column (same
// encoding gotcha item_proto.locale_name has - see
// m2manager_data_encoding_issue), rather than hardcoding either assumption.

#[derive(Debug, Clone, Serialize, Default)]
pub struct EconomyStats {
    pub player_count: i64,
    pub total_gold: i64,
    pub shop_item_count: i64,
    pub avg_shop_price: Option<f64>,
    pub min_shop_price: Option<i64>,
    pub max_shop_price: Option<i64>,
}

#[derive(Debug, Clone, Serialize)]
pub struct TopGoldHolder {
    pub player_id: i32,
    pub name: Option<String>,
    pub gold: i64,
}

pub async fn get_economy_stats(pool: &MySqlPool) -> Result<EconomyStats, String> {
    let totals = sqlx::query(
        "SELECT COUNT(*) AS player_count, COALESCE(SUM(gold), 0) AS total_gold FROM player.player",
    )
    .fetch_one(pool)
    .await
    .map_err(|e| e.to_string())?;

    let prices = sqlx::query(
        "SELECT COUNT(*) AS n, AVG(ip.gold) AS avg_price, MIN(ip.gold) AS min_price, MAX(ip.gold) AS max_price \
         FROM player.shop_item si JOIN player.item_proto ip ON ip.vnum = si.item_vnum",
    )
    .fetch_one(pool)
    .await
    .map_err(|e| e.to_string())?;

    Ok(EconomyStats {
        player_count: totals.try_get("player_count").unwrap_or_default(),
        total_gold: totals.try_get("total_gold").unwrap_or_default(),
        shop_item_count: prices.try_get("n").unwrap_or_default(),
        avg_shop_price: prices.try_get("avg_price").ok(),
        min_shop_price: prices.try_get("min_price").ok(),
        max_shop_price: prices.try_get("max_price").ok(),
    })
}

pub async fn get_top_gold_holders(pool: &MySqlPool, limit: i64) -> Result<Vec<TopGoldHolder>, String> {
    let columns = super::explorer::get_columns(pool, "player", "player").await?;
    let name_column = columns.iter().find(|c| c.name == "name");

    let sql = if name_column.is_some() {
        "SELECT id, name, gold FROM player.player ORDER BY gold DESC LIMIT ?"
    } else {
        "SELECT id, gold FROM player.player ORDER BY gold DESC LIMIT ?"
    };
    let rows = sqlx::query(sql)
        .bind(limit)
        .fetch_all(pool)
        .await
        .map_err(|e| e.to_string())?;

    let name_is_binary = name_column.is_some_and(|c| c.data_type.contains("binary"));

    Ok(rows
        .into_iter()
        .map(|row| {
            let name = if name_column.is_none() {
                None
            } else if name_is_binary {
                row.try_get::<Vec<u8>, _>("name").ok().map(|b| decode_name(&b))
            } else {
                row.try_get::<String, _>("name").ok()
            };
            TopGoldHolder {
                player_id: row.try_get("id").unwrap_or_default(),
                name,
                gold: row.try_get("gold").unwrap_or_default(),
            }
        })
        .collect())
}
