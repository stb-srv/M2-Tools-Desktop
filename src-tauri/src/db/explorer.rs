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

fn quote_ident(name: &str) -> String {
    format!("`{}`", name.replace('`', ""))
}

/// Pure SQL-text builders, split out from the DB-touching functions below so
/// the exact statement shape is unit-testable without a live connection.
fn build_update_sql(database: &str, table: &str, pk_column: &str, columns: &[String]) -> String {
    let set_clause = columns
        .iter()
        .map(|c| format!("{} = ?", quote_ident(c)))
        .collect::<Vec<_>>()
        .join(", ");
    format!(
        "UPDATE {}.{} SET {set_clause} WHERE {} = ?",
        quote_ident(database),
        quote_ident(table),
        quote_ident(pk_column)
    )
}

fn build_insert_sql(database: &str, table: &str, columns: &[String]) -> String {
    let columns_clause = columns.iter().map(|c| quote_ident(c)).collect::<Vec<_>>().join(", ");
    let placeholders = columns.iter().map(|_| "?").collect::<Vec<_>>().join(", ");
    format!(
        "INSERT INTO {}.{} ({columns_clause}) VALUES ({placeholders})",
        quote_ident(database),
        quote_ident(table)
    )
}

fn build_delete_sql(database: &str, table: &str, pk_column: &str) -> String {
    format!(
        "DELETE FROM {}.{} WHERE {} = ? LIMIT 1",
        quote_ident(database),
        quote_ident(table),
        quote_ident(pk_column)
    )
}

/// Binds a single generic value, encoding `binary`/`varbinary` columns as
/// Windows-1252 bytes to mirror how `stringify`/`decode_bytes` read them
/// back (see the module-level comment on `decode_bytes`) - otherwise a
/// generic writer would silently corrupt any binary-collation column
/// containing non-ASCII text (e.g. `locale_name` on this server).
fn bind_value<'q>(
    query: sqlx::query::Query<'q, sqlx::MySql, sqlx::mysql::MySqlArguments>,
    is_binary: bool,
    value: Option<String>,
) -> sqlx::query::Query<'q, sqlx::MySql, sqlx::mysql::MySqlArguments> {
    match (is_binary, value) {
        (true, Some(v)) => {
            let (bytes, _, _) = encoding_rs::WINDOWS_1252.encode(&v);
            query.bind(bytes.into_owned())
        }
        (true, None) => query.bind(None::<Vec<u8>>),
        (false, Some(v)) => query.bind(v),
        (false, None) => query.bind(None::<String>),
    }
}

/// Single row lookup by primary key - used to load a row into a generic
/// edit form (Mob-Proto-Editor, Account-/Player-Verwaltung).
pub async fn get_row_by_pk(
    pool: &MySqlPool,
    database: &str,
    table: &str,
    pk_column: &str,
    pk_value: &str,
) -> Result<Option<TableRows>, String> {
    let known_columns = get_columns(pool, database, table).await?;
    if !known_columns.iter().any(|c| c.name == pk_column) {
        return Err(format!("Unbekannte Primärschlüssel-Spalte: {pk_column}"));
    }
    let sql = format!(
        "SELECT * FROM {}.{} WHERE {} = ? LIMIT 1",
        quote_ident(database),
        quote_ident(table),
        quote_ident(pk_column)
    );
    let row = sqlx::query(&sql)
        .bind(pk_value)
        .fetch_optional(pool)
        .await
        .map_err(|e| e.to_string())?;

    Ok(row.map(|r| {
        let columns: Vec<String> = r.columns().iter().map(|c| c.name().to_string()).collect();
        let data = (0..columns.len()).map(|i| stringify(&r, i)).collect();
        TableRows {
            columns,
            rows: vec![data],
            total_rows: 1,
        }
    }))
}

/// Generic parameterized UPDATE by primary key. `changes` is validated
/// against `get_columns` first (rejects unknown column names) so the
/// interpolated identifiers can only ever be real column names, never
/// arbitrary caller input.
pub async fn update_row(
    pool: &MySqlPool,
    database: &str,
    table: &str,
    pk_column: &str,
    pk_value: &str,
    changes: &[(String, Option<String>)],
) -> Result<(), String> {
    if changes.is_empty() {
        return Err("Keine Änderungen übergeben.".to_string());
    }
    let known_columns = get_columns(pool, database, table).await?;
    let find = |name: &str| known_columns.iter().find(|c| c.name == name);
    if find(pk_column).is_none() {
        return Err(format!("Unbekannte Primärschlüssel-Spalte: {pk_column}"));
    }
    for (name, _) in changes {
        if find(name).is_none() {
            return Err(format!("Unbekannte Spalte: {name}"));
        }
    }

    let column_names: Vec<String> = changes.iter().map(|(n, _)| n.clone()).collect();
    let sql = build_update_sql(database, table, pk_column, &column_names);
    let mut query = sqlx::query(&sql);
    for (name, value) in changes {
        let is_binary = find(name).unwrap().data_type.contains("binary");
        query = bind_value(query, is_binary, value.clone());
    }
    query = query.bind(pk_value);
    query.execute(pool).await.map_err(|e| e.to_string())?;
    Ok(())
}

/// Generic parameterized INSERT - used for "add row" flows (e.g. giving a
/// player an item) where the exact column semantics are core-specific and
/// not something this app can verify; the caller fills in every value.
pub async fn insert_row(
    pool: &MySqlPool,
    database: &str,
    table: &str,
    values: &[(String, Option<String>)],
) -> Result<(), String> {
    if values.is_empty() {
        return Err("Keine Werte übergeben.".to_string());
    }
    let known_columns = get_columns(pool, database, table).await?;
    let find = |name: &str| known_columns.iter().find(|c| c.name == name);
    for (name, _) in values {
        if find(name).is_none() {
            return Err(format!("Unbekannte Spalte: {name}"));
        }
    }

    let column_names: Vec<String> = values.iter().map(|(n, _)| n.clone()).collect();
    let sql = build_insert_sql(database, table, &column_names);
    let mut query = sqlx::query(&sql);
    for (name, value) in values {
        let is_binary = find(name).unwrap().data_type.contains("binary");
        query = bind_value(query, is_binary, value.clone());
    }
    query.execute(pool).await.map_err(|e| e.to_string())?;
    Ok(())
}

/// Generic DELETE by primary key (`LIMIT 1` so a caller-supplied value can
/// never wipe more than the single intended row, even without a unique
/// constraint on the column).
pub async fn delete_row(
    pool: &MySqlPool,
    database: &str,
    table: &str,
    pk_column: &str,
    pk_value: &str,
) -> Result<(), String> {
    let known_columns = get_columns(pool, database, table).await?;
    if !known_columns.iter().any(|c| c.name == pk_column) {
        return Err(format!("Unbekannte Primärschlüssel-Spalte: {pk_column}"));
    }
    let sql = build_delete_sql(database, table, pk_column);
    sqlx::query(&sql)
        .bind(pk_value)
        .execute(pool)
        .await
        .map_err(|e| e.to_string())?;
    Ok(())
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
    let column_info = known_columns
        .iter()
        .find(|c| c.name == col)
        .ok_or_else(|| format!("Unbekannte Spalte: {search_column}"))?;

    let like = format!("%{}%", query.replace('%', "\\%").replace('_', "\\_"));
    // `varbinary`/`binary` columns (item_proto.locale_name etc.) have no
    // charset/collation, so a plain LIKE is byte-for-byte case-sensitive -
    // verified this made name search on those columns silently fail for
    // anything but the exact stored casing (see [[m2manager_quest_builder]]
    // for the same bug found in shop.rs). `CONVERT(... USING latin1)` gives
    // both sides a real charset to case-fold against. Only applied to binary
    // columns - real VARCHAR columns already have a sane collation and a
    // forced CONVERT would just be unnecessary overhead there.
    let is_binary = column_info.data_type.contains("binary");
    let sql = if is_binary {
        format!(
            "SELECT * FROM `{db}`.`{tbl}` \
             WHERE LOWER(CONVERT(`{col}` USING latin1)) LIKE LOWER(CONVERT(? USING latin1)) LIMIT ?"
        )
    } else {
        format!("SELECT * FROM `{db}`.`{tbl}` WHERE `{col}` LIKE ? LIMIT ?")
    };
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn builds_update_sql_with_backtick_quoted_identifiers() {
        let sql = build_update_sql(
            "player",
            "mob_proto",
            "vnum",
            &["hp".to_string(), "exp".to_string()],
        );
        assert_eq!(
            sql,
            "UPDATE `player`.`mob_proto` SET `hp` = ?, `exp` = ? WHERE `vnum` = ?"
        );
    }

    #[test]
    fn builds_insert_sql() {
        let sql = build_insert_sql(
            "player",
            "item",
            &["owner_id".to_string(), "vnum".to_string()],
        );
        assert_eq!(
            sql,
            "INSERT INTO `player`.`item` (`owner_id`, `vnum`) VALUES (?, ?)"
        );
    }

    #[test]
    fn builds_delete_sql() {
        let sql = build_delete_sql("player", "item", "id");
        assert_eq!(sql, "DELETE FROM `player`.`item` WHERE `id` = ? LIMIT 1");
    }

    #[test]
    fn quote_ident_strips_any_embedded_backtick_rather_than_escaping_it() {
        // Defense in depth: identifiers only ever come from information_schema
        // listings (see get_columns), never raw user text, but stripping
        // backticks here means even a malformed identifier can't break out of
        // the quoted context.
        assert_eq!(quote_ident("weird`name"), "`weirdname`");
    }
}
