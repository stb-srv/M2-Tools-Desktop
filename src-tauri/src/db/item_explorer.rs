use serde::Serialize;
use sqlx::{MySqlPool, Row};

use super::shop::decode_name;

#[derive(Debug, Clone, Serialize)]
pub struct ItemProtoSummary {
    pub vnum: i32,
    pub name: String,
    pub item_type: i8,
    pub subtype: i8,
}

#[derive(Debug, Clone, Serialize)]
pub struct ItemProtoPage {
    pub rows: Vec<ItemProtoSummary>,
    pub total: i64,
}

/// Standalone browsable overview of the whole `item_proto` table (name/vnum
/// search like `browse_items` in `shop.rs`, plus an optional exact `type`
/// filter) - used by the Item-Proto-Explorer page. Kept separate from
/// `browse_items`/`EntityBrowser` (which only ever return vnum+name for the
/// pickers embedded in other editors) because this page additionally shows
/// type/subtype per row, which those callers don't need.
pub async fn browse_item_proto(
    pool: &MySqlPool,
    query: Option<&str>,
    type_filter: Option<i8>,
    offset: i64,
    limit: i64,
) -> Result<ItemProtoPage, String> {
    let trimmed = query.map(str::trim).filter(|q| !q.is_empty());
    let mut conditions: Vec<&str> = Vec::new();
    let like_value: Option<String> = match trimmed {
        None => None,
        Some(q) if q.chars().all(|c| c.is_ascii_digit()) => {
            conditions.push("CAST(vnum AS CHAR) LIKE ?");
            Some(format!("%{q}%"))
        }
        Some(q) => {
            conditions.push("LOWER(CONVERT(locale_name USING latin1)) LIKE LOWER(CONVERT(? USING latin1))");
            Some(format!("%{}%", q.replace('%', "\\%").replace('_', "\\_")))
        }
    };
    if type_filter.is_some() {
        conditions.push("type = ?");
    }
    let where_clause = if conditions.is_empty() {
        String::new()
    } else {
        format!("WHERE {}", conditions.join(" AND "))
    };

    let count_sql = format!("SELECT COUNT(*) AS c FROM player.item_proto {where_clause}");
    let mut count_query = sqlx::query(&count_sql);
    if let Some(v) = &like_value {
        count_query = count_query.bind(v);
    }
    if let Some(t) = type_filter {
        count_query = count_query.bind(t);
    }
    let total: i64 = count_query
        .fetch_one(pool)
        .await
        .map_err(|e| e.to_string())?
        .try_get::<i64, _>("c")
        .unwrap_or(0);

    let rows_sql = format!(
        "SELECT vnum, locale_name, type, subtype FROM player.item_proto {where_clause} ORDER BY vnum LIMIT ? OFFSET ?"
    );
    let mut rows_query = sqlx::query(&rows_sql);
    if let Some(v) = &like_value {
        rows_query = rows_query.bind(v);
    }
    if let Some(t) = type_filter {
        rows_query = rows_query.bind(t);
    }
    let rows = rows_query
        .bind(limit)
        .bind(offset)
        .fetch_all(pool)
        .await
        .map_err(|e| e.to_string())?;

    let rows = rows
        .into_iter()
        .map(|row| {
            let name_raw: Vec<u8> = row.try_get("locale_name").unwrap_or_default();
            ItemProtoSummary {
                vnum: row.try_get("vnum").unwrap_or_default(),
                name: decode_name(&name_raw),
                item_type: row.try_get("type").unwrap_or_default(),
                subtype: row.try_get("subtype").unwrap_or_default(),
            }
        })
        .collect();

    Ok(ItemProtoPage { rows, total })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn summary_serializes_type_and_subtype() {
        let s = ItemProtoSummary { vnum: 7000, name: "Schwert".into(), item_type: 1, subtype: 2 };
        let json = serde_json::to_string(&s).unwrap();
        assert!(json.contains("\"item_type\":1"));
        assert!(json.contains("\"subtype\":2"));
    }
}
