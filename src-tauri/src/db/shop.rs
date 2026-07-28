use serde::Serialize;
use sqlx::{MySqlPool, Row};

#[derive(Debug, Clone, Serialize)]
pub struct ShopSummary {
    pub vnum: i32,
    pub name: String,
    pub npc_vnum: i16,
    pub npc_name: String,
    pub npc_folder: Option<String>,
    pub item_count: i64,
}

#[derive(Debug, Clone, Serialize)]
pub struct ShopItem {
    pub item_vnum: i32,
    pub item_name: String,
    pub count: u16,
    // Grid footprint in shop/inventory slots (1, 2, or 3 wide) - item_proto.size,
    // not .subtype (which is a type-dependent classification, e.g. weapon class).
    pub size: i8,
}

#[derive(Debug, Clone, Serialize)]
pub struct ItemSearchResult {
    pub vnum: u32,
    pub name: String,
}

// item_proto/mob_proto .name/.locale_name columns are `varbinary` holding
// Windows-1252-encoded text (umlauts etc. aren't valid UTF-8 start bytes),
// so decode explicitly rather than assuming UTF-8.
fn decode_name(bytes: &[u8]) -> String {
    let trimmed = bytes
        .split(|&b| b == 0)
        .next()
        .unwrap_or(bytes);
    let (text, _, _) = encoding_rs::WINDOWS_1252.decode(trimmed);
    text.trim().to_string()
}

#[derive(Debug, Clone, Serialize, Default)]
pub struct DatabaseStats {
    pub accounts: i64,
    pub players: i64,
    pub items: i64,
    pub shops: i64,
    pub mobs: i64,
}

pub async fn get_stats(pool: &MySqlPool) -> Result<DatabaseStats, String> {
    let row = sqlx::query(
        "SELECT \
         (SELECT COUNT(*) FROM account.account) AS accounts, \
         (SELECT COUNT(*) FROM player.player) AS players, \
         (SELECT COUNT(*) FROM player.item_proto) AS items, \
         (SELECT COUNT(*) FROM player.shop) AS shops, \
         (SELECT COUNT(*) FROM player.mob_proto) AS mobs",
    )
    .fetch_one(pool)
    .await
    .map_err(|e| e.to_string())?;

    Ok(DatabaseStats {
        accounts: row.try_get("accounts").unwrap_or_default(),
        players: row.try_get("players").unwrap_or_default(),
        items: row.try_get("items").unwrap_or_default(),
        shops: row.try_get("shops").unwrap_or_default(),
        mobs: row.try_get("mobs").unwrap_or_default(),
    })
}

pub async fn list_shops(pool: &MySqlPool) -> Result<Vec<ShopSummary>, String> {
    let rows = sqlx::query(
        "SELECT s.vnum, s.name, s.npc_vnum, m.locale_name AS npc_name_raw, m.folder AS npc_folder, \
         (SELECT COUNT(*) FROM player.shop_item si WHERE si.shop_vnum = s.vnum) AS item_count \
         FROM player.shop s \
         LEFT JOIN player.mob_proto m ON m.vnum = s.npc_vnum \
         ORDER BY s.vnum",
    )
    .fetch_all(pool)
    .await
    .map_err(|e| e.to_string())?;

    Ok(rows
        .into_iter()
        .map(|row| {
            let npc_name_raw: Option<Vec<u8>> = row.try_get("npc_name_raw").unwrap_or_default();
            ShopSummary {
                vnum: row.try_get("vnum").unwrap_or_default(),
                name: row.try_get("name").unwrap_or_default(),
                npc_vnum: row.try_get("npc_vnum").unwrap_or_default(),
                npc_name: npc_name_raw
                    .map(|b| decode_name(&b))
                    .unwrap_or_else(|| "?".to_string()),
                npc_folder: row.try_get("npc_folder").ok(),
                item_count: row.try_get("item_count").unwrap_or_default(),
            }
        })
        .collect())
}

pub async fn rename_shop(pool: &MySqlPool, shop_vnum: i32, name: &str) -> Result<(), String> {
    sqlx::query("UPDATE player.shop SET name = ? WHERE vnum = ?")
        .bind(name)
        .bind(shop_vnum)
        .execute(pool)
        .await
        .map_err(|e| e.to_string())?;
    Ok(())
}

pub async fn create_shop(pool: &MySqlPool, name: &str, npc_vnum: i16) -> Result<i32, String> {
    let next_vnum: Option<i32> = sqlx::query_scalar("SELECT MAX(vnum) + 1 FROM player.shop")
        .fetch_one(pool)
        .await
        .map_err(|e| e.to_string())?;
    let vnum = next_vnum.unwrap_or(1);

    sqlx::query("INSERT INTO player.shop (vnum, name, npc_vnum) VALUES (?, ?, ?)")
        .bind(vnum)
        .bind(name)
        .bind(npc_vnum)
        .execute(pool)
        .await
        .map_err(|e| e.to_string())?;

    Ok(vnum)
}

pub async fn get_shop_items(pool: &MySqlPool, shop_vnum: i32) -> Result<Vec<ShopItem>, String> {
    let rows = sqlx::query(
        "SELECT si.item_vnum, si.count, ip.locale_name AS item_name_raw, ip.size \
         FROM player.shop_item si \
         JOIN player.item_proto ip ON ip.vnum = si.item_vnum \
         WHERE si.shop_vnum = ? ORDER BY si.item_vnum",
    )
    .bind(shop_vnum)
    .fetch_all(pool)
    .await
    .map_err(|e| e.to_string())?;

    Ok(rows
        .into_iter()
        .map(|row| {
            let name_raw: Vec<u8> = row.try_get("item_name_raw").unwrap_or_default();
            let size: i8 = row.try_get("size").unwrap_or(1);
            ShopItem {
                item_vnum: row.try_get("item_vnum").unwrap_or_default(),
                item_name: decode_name(&name_raw),
                count: row.try_get("count").unwrap_or_default(),
                size: size.max(1),
            }
        })
        .collect())
}

pub async fn search_items(
    pool: &MySqlPool,
    query: &str,
    limit: i64,
) -> Result<Vec<ItemSearchResult>, String> {
    let rows = if let Ok(vnum) = query.trim().parse::<i32>() {
        sqlx::query("SELECT vnum, locale_name FROM player.item_proto WHERE vnum = ? LIMIT ?")
            .bind(vnum)
            .bind(limit)
            .fetch_all(pool)
            .await
    } else {
        let like = format!("%{}%", query.replace('%', "\\%").replace('_', "\\_"));
        sqlx::query(
            "SELECT vnum, locale_name FROM player.item_proto WHERE locale_name LIKE ? LIMIT ?",
        )
        .bind(like)
        .bind(limit)
        .fetch_all(pool)
        .await
    }
    .map_err(|e| e.to_string())?;

    Ok(rows
        .into_iter()
        .map(|row| {
            let name_raw: Vec<u8> = row.try_get("locale_name").unwrap_or_default();
            ItemSearchResult {
                vnum: row.try_get("vnum").unwrap_or_default(),
                name: decode_name(&name_raw),
            }
        })
        .collect())
}

pub async fn update_shop_item_count(
    pool: &MySqlPool,
    shop_vnum: i32,
    item_vnum: i32,
    count: i32,
) -> Result<(), String> {
    sqlx::query("UPDATE player.shop_item SET count = ? WHERE shop_vnum = ? AND item_vnum = ?")
        .bind(count.max(0))
        .bind(shop_vnum)
        .bind(item_vnum)
        .execute(pool)
        .await
        .map_err(|e| e.to_string())?;
    Ok(())
}

pub async fn add_shop_item(
    pool: &MySqlPool,
    shop_vnum: i32,
    item_vnum: i32,
    count: i32,
) -> Result<(), String> {
    sqlx::query(
        "INSERT INTO player.shop_item (shop_vnum, item_vnum, count) VALUES (?, ?, ?) \
         ON DUPLICATE KEY UPDATE count = VALUES(count)",
    )
    .bind(shop_vnum)
    .bind(item_vnum)
    .bind(count.max(1))
    .execute(pool)
    .await
    .map_err(|e| e.to_string())?;
    Ok(())
}

pub async fn remove_shop_item(
    pool: &MySqlPool,
    shop_vnum: i32,
    item_vnum: i32,
) -> Result<(), String> {
    sqlx::query("DELETE FROM player.shop_item WHERE shop_vnum = ? AND item_vnum = ?")
        .bind(shop_vnum)
        .bind(item_vnum)
        .execute(pool)
        .await
        .map_err(|e| e.to_string())?;
    Ok(())
}

pub async fn delete_shop(pool: &MySqlPool, shop_vnum: i32) -> Result<(), String> {
    let mut tx = pool.begin().await.map_err(|e| e.to_string())?;
    sqlx::query("DELETE FROM player.shop_item WHERE shop_vnum = ?")
        .bind(shop_vnum)
        .execute(&mut *tx)
        .await
        .map_err(|e| e.to_string())?;
    sqlx::query("DELETE FROM player.shop WHERE vnum = ?")
        .bind(shop_vnum)
        .execute(&mut *tx)
        .await
        .map_err(|e| e.to_string())?;
    tx.commit().await.map_err(|e| e.to_string())?;
    Ok(())
}
