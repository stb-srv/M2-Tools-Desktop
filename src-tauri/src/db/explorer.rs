use serde::Serialize;
use sqlx::{Column, MySqlPool, Row};

const SYSTEM_DATABASES: &[&str] = &["information_schema", "performance_schema", "mysql", "sys"];

#[derive(Debug, Clone, Serialize)]
pub struct TableInfo {
    pub name: String,
    pub approx_rows: i64,
}

#[derive(Debug, Clone, Serialize)]
pub struct ColumnInfo {
    pub name: String,
    pub data_type: String,
    pub is_nullable: bool,
    pub is_primary_key: bool,
}

#[derive(Debug, Clone, Serialize)]
pub struct TableRows {
    pub columns: Vec<String>,
    /// Every cell stringified best-effort - a generic browser can't know each
    /// column's real semantics (see decode_cell), unlike the Shop Editor's
    /// purpose-built queries which decode locale_name etc. correctly.
    pub rows: Vec<Vec<Option<String>>>,
    pub total_rows: i64,
}

pub async fn list_databases(pool: &MySqlPool) -> Result<Vec<String>, String> {
    let names: Vec<String> = sqlx::query_scalar("SHOW DATABASES")
        .fetch_all(pool)
        .await
        .map_err(|e| e.to_string())?;
    Ok(names
        .into_iter()
        .filter(|n| !SYSTEM_DATABASES.contains(&n.as_str()))
        .collect())
}

pub async fn list_tables(pool: &MySqlPool, database: &str) -> Result<Vec<TableInfo>, String> {
    let rows = sqlx::query(
        "SELECT TABLE_NAME, TABLE_ROWS FROM information_schema.TABLES \
         WHERE TABLE_SCHEMA = ? ORDER BY TABLE_NAME",
    )
    .bind(database)
    .fetch_all(pool)
    .await
    .map_err(|e| e.to_string())?;

    Ok(rows
        .into_iter()
        .map(|row| TableInfo {
            name: row.try_get("TABLE_NAME").unwrap_or_default(),
            approx_rows: row.try_get("TABLE_ROWS").unwrap_or(0),
        })
        .collect())
}

pub async fn get_columns(
    pool: &MySqlPool,
    database: &str,
    table: &str,
) -> Result<Vec<ColumnInfo>, String> {
    let rows = sqlx::query(
        "SELECT COLUMN_NAME, COLUMN_TYPE, IS_NULLABLE, COLUMN_KEY \
         FROM information_schema.COLUMNS \
         WHERE TABLE_SCHEMA = ? AND TABLE_NAME = ? ORDER BY ORDINAL_POSITION",
    )
    .bind(database)
    .bind(table)
    .fetch_all(pool)
    .await
    .map_err(|e| e.to_string())?;

    Ok(rows
        .into_iter()
        .map(|row| {
            let nullable: String = row.try_get("IS_NULLABLE").unwrap_or_default();
            let key: String = row.try_get("COLUMN_KEY").unwrap_or_default();
            ColumnInfo {
                name: row.try_get("COLUMN_NAME").unwrap_or_default(),
                data_type: row.try_get("COLUMN_TYPE").unwrap_or_default(),
                is_nullable: nullable == "YES",
                is_primary_key: key == "PRI",
            }
        })
        .collect())
}

// varbinary/text columns in this schema are Windows-1252 (see
// m2manager-item-icons memory), which decodes essentially any byte
// sequence - so try it after UTF-8 rather than falling back to a raw
// byte-count placeholder, which would make most text columns unreadable.
fn decode_bytes(bytes: &[u8]) -> String {
    if let Ok(s) = std::str::from_utf8(bytes) {
        return s.trim_end_matches('\0').to_string();
    }
    let trimmed = bytes.split(|&b| b == 0).next().unwrap_or(bytes);
    let (text, _, _) = encoding_rs::WINDOWS_1252.decode(trimmed);
    text.into_owned()
}

fn stringify(row: &sqlx::mysql::MySqlRow, index: usize) -> Option<String> {
    if let Ok(v) = row.try_get::<Option<String>, _>(index) {
        return v;
    }
    if let Ok(v) = row.try_get::<Option<i64>, _>(index) {
        return v.map(|x| x.to_string());
    }
    if let Ok(v) = row.try_get::<Option<u64>, _>(index) {
        return v.map(|x| x.to_string());
    }
    if let Ok(v) = row.try_get::<Option<f64>, _>(index) {
        return v.map(|x| x.to_string());
    }
    if let Ok(v) = row.try_get::<Option<bool>, _>(index) {
        return v.map(|x| x.to_string());
    }
    if let Ok(v) = row.try_get::<Option<chrono::NaiveDateTime>, _>(index) {
        return v.map(|x| x.to_string());
    }
    if let Ok(v) = row.try_get::<Option<chrono::NaiveDate>, _>(index) {
        return v.map(|x| x.to_string());
    }
    if let Ok(v) = row.try_get::<Option<Vec<u8>>, _>(index) {
        return v.map(|bytes| decode_bytes(&bytes));
    }
    None
}

pub async fn get_rows(
    pool: &MySqlPool,
    database: &str,
    table: &str,
    limit: i64,
    offset: i64,
) -> Result<TableRows, String> {
    // Table/database names can't be bound as query parameters in MySQL;
    // both come from information_schema listings we generated ourselves
    // (never raw user text), so backtick-quoting them is safe here.
    let query = format!(
        "SELECT * FROM `{}`.`{}` LIMIT ? OFFSET ?",
        database.replace('`', ""),
        table.replace('`', "")
    );
    let rows = sqlx::query(&query)
        .bind(limit)
        .bind(offset)
        .fetch_all(pool)
        .await
        .map_err(|e| e.to_string())?;

    let total_rows: i64 = sqlx::query_scalar(&format!(
        "SELECT COUNT(*) FROM `{}`.`{}`",
        database.replace('`', ""),
        table.replace('`', "")
    ))
    .fetch_one(pool)
    .await
    .map_err(|e| e.to_string())?;

    let columns: Vec<String> = rows
        .first()
        .map(|r| r.columns().iter().map(|c| c.name().to_string()).collect())
        .unwrap_or_default();

    let data = rows
        .iter()
        .map(|row| (0..columns.len()).map(|i| stringify(row, i)).collect())
        .collect();

    Ok(TableRows {
        columns,
        rows: data,
        total_rows,
    })
}

pub async fn search_rows(
    pool: &MySqlPool,
    database: &str,
    table: &str,
    search_column: &str,
    query: &str,
    limit: i64,
) -> Result<TableRows, String> {
    let db = database.replace('`', "");
    let tbl = table.replace('`', "");
    let col = search_column.replace('`', "");

    // Column identity is validated against information_schema server-side
    // (get_columns), so by the time this runs it's a known-good name - still
    // worth the defensive check since it's interpolated, not bound.
    let known_columns = get_columns(pool, database, table).await?;
    if !known_columns.iter().any(|c| c.name == col) {
        return Err(format!("Unbekannte Spalte: {search_column}"));
    }

    let like = format!("%{}%", query.replace('%', "\\%").replace('_', "\\_"));
    let sql = format!("SELECT * FROM `{db}`.`{tbl}` WHERE `{col}` LIKE ? LIMIT ?");
    let rows = sqlx::query(&sql)
        .bind(like)
        .bind(limit)
        .fetch_all(pool)
        .await
        .map_err(|e| e.to_string())?;

    let columns: Vec<String> = rows
        .first()
        .map(|r| r.columns().iter().map(|c| c.name().to_string()).collect())
        .unwrap_or_else(|| known_columns.iter().map(|c| c.name.clone()).collect());

    let total = rows.len() as i64;
    let data = rows
        .iter()
        .map(|row| (0..columns.len()).map(|i| stringify(row, i)).collect())
        .collect();

    Ok(TableRows {
        columns,
        rows: data,
        total_rows: total,
    })
}
