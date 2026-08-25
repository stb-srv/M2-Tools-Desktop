use crate::db::item::ItemProtoInput;
use rusqlite::Connection;
use serde::Serialize;

// Named, reusable Item Editor presets ("Standard-Ohrring", ...) - unlike
// "Referenz-Item übernehmen" (which copies an existing real item_proto row),
// a preset isn't tied to any specific item and lives purely in this app's
// local settings DB. Reuses `db::item::ItemProtoInput` directly (already
// Serialize/Deserialize, already the single source of truth for "what fields
// does the create form manage") instead of duplicating its ~40 fields in a
// second struct that could drift out of sync. `vnum` is stored as part of
// the blob for simplicity but is meaningless on load - the frontend always
// keeps whatever vnum is already in the form, exactly like "Referenz-Item
// übernehmen" does (see ItemEditor.tsx::loadReference).

#[derive(Debug, Clone, Serialize)]
pub struct ItemPreset {
    pub id: i64,
    pub name: String,
    pub item: ItemProtoInput,
}

/// Saving under an existing name overwrites it (matches "Speichern unter
/// diesem Namen" expectations elsewhere in the app - no separate rename
/// step needed).
pub fn save(conn: &Connection, name: &str, item: &ItemProtoInput) -> Result<i64, String> {
    let data = serde_json::to_string(item).map_err(|e| e.to_string())?;
    conn.execute(
        "INSERT INTO item_presets (name, data) VALUES (?1, ?2)
         ON CONFLICT(name) DO UPDATE SET data = excluded.data",
        rusqlite::params![name, data],
    )
    .map_err(|e| e.to_string())?;
    conn.query_row(
        "SELECT id FROM item_presets WHERE name = ?1",
        rusqlite::params![name],
        |row| row.get(0),
    )
    .map_err(|e| e.to_string())
}

pub fn list(conn: &Connection) -> Result<Vec<ItemPreset>, String> {
    let mut stmt = conn
        .prepare("SELECT id, name, data FROM item_presets ORDER BY name COLLATE NOCASE")
        .map_err(|e| e.to_string())?;
    let rows = stmt
        .query_map([], |row| {
            let id: i64 = row.get(0)?;
            let name: String = row.get(1)?;
            let data: String = row.get(2)?;
            Ok((id, name, data))
        })
        .map_err(|e| e.to_string())?;

    let mut presets = Vec::new();
    for row in rows {
        let (id, name, data) = row.map_err(|e| e.to_string())?;
        let item: ItemProtoInput = serde_json::from_str(&data).map_err(|e| e.to_string())?;
        presets.push(ItemPreset { id, name, item });
    }
    Ok(presets)
}

pub fn delete(conn: &Connection, id: i64) -> Result<(), String> {
    conn.execute("DELETE FROM item_presets WHERE id = ?1", rusqlite::params![id])
        .map_err(|e| e.to_string())?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn scratch_conn() -> Connection {
        let conn = Connection::open_in_memory().expect("open in-memory db");
        conn.execute_batch(
            "CREATE TABLE item_presets (
                id INTEGER PRIMARY KEY AUTOINCREMENT,
                name TEXT NOT NULL UNIQUE,
                data TEXT NOT NULL
            );",
        )
        .expect("create table");
        conn
    }

    fn sample_item(vnum: u32) -> ItemProtoInput {
        ItemProtoInput {
            vnum,
            vnum_range: 0,
            name: "test_item".into(),
            locale_name: "Testgegenstand".into(),
            r#type: 3,
            subtype: 0,
            weight: 0,
            size: 1,
            antiflag: 0,
            flag: 0,
            wearflag: 0,
            immuneflag: 0,
            gold: 0,
            shop_buy_price: 0,
            refined_vnum: 0,
            refine_set: 0,
            magic_pct: 0,
            limittype0: 0,
            limitvalue0: 0,
            limittype1: 0,
            limitvalue1: 0,
            applytype0: 0,
            applyvalue0: 0,
            applytype1: 0,
            applyvalue1: 0,
            applytype2: 0,
            applyvalue2: 0,
            applytype3: 0,
            applyvalue3: 0,
            value0: 0,
            value1: 0,
            value2: 0,
            value3: 0,
            value4: 0,
            value5: 0,
            socket0: 0,
            socket1: 0,
            socket2: 0,
            socket3: 0,
            socket4: 0,
            socket5: 0,
            specular: 0,
            socket_pct: 0,
            addon_type: 0,
        }
    }

    #[test]
    fn saves_and_lists_round_trip() {
        let conn = scratch_conn();
        save(&conn, "Standard-Ohrring", &sample_item(0)).unwrap();
        let presets = list(&conn).unwrap();
        assert_eq!(presets.len(), 1);
        assert_eq!(presets[0].name, "Standard-Ohrring");
        assert_eq!(presets[0].item.locale_name, "Testgegenstand");
    }

    #[test]
    fn saving_under_existing_name_overwrites_instead_of_duplicating() {
        let conn = scratch_conn();
        let id1 = save(&conn, "Vorlage", &sample_item(1)).unwrap();
        let mut updated = sample_item(2);
        updated.locale_name = "Geändert".into();
        let id2 = save(&conn, "Vorlage", &updated).unwrap();
        assert_eq!(id1, id2);
        let presets = list(&conn).unwrap();
        assert_eq!(presets.len(), 1);
        assert_eq!(presets[0].item.locale_name, "Geändert");
    }

    #[test]
    fn lists_alphabetically_case_insensitive() {
        let conn = scratch_conn();
        save(&conn, "zebra", &sample_item(0)).unwrap();
        save(&conn, "Apfel", &sample_item(0)).unwrap();
        let presets = list(&conn).unwrap();
        assert_eq!(presets.iter().map(|p| p.name.as_str()).collect::<Vec<_>>(), vec!["Apfel", "zebra"]);
    }

    #[test]
    fn delete_removes_only_the_targeted_preset() {
        let conn = scratch_conn();
        let id = save(&conn, "weg", &sample_item(0)).unwrap();
        save(&conn, "bleibt", &sample_item(0)).unwrap();
        delete(&conn, id).unwrap();
        let presets = list(&conn).unwrap();
        assert_eq!(presets.len(), 1);
        assert_eq!(presets[0].name, "bleibt");
    }
}
