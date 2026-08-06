use serde::{Deserialize, Serialize};
use sqlx::{MySqlPool, Row};

// item_proto.refine_set -> refine_proto.id is a plain foreign key to a
// *recipe*, not related to any item vnum despite the naming - the recipe id
// is commonly reused across several unrelated items (verified live: recipe
// id 1 alone is shared by 8 different items' first refine step). The real
// upgrade destination is item_proto.refined_vnum, a completely separate
// column. refine_proto.src_vnum/result_vnum exist in the SQL schema but are
// vestigial - verified against the real server source
// (source/db/src/ClientManagerBoot.cpp's own `SELECT id, cost, prob,
// vnum0, count0, ... FROM refine_proto` never selects them, and the C++
// struct `TRefineTable` in source/common/tables.h has no such fields at
// all), so they are deliberately not modeled here.
//
// Column types below are verified against `information_schema.columns` for
// this server (2026-08-06), not assumed - `refine_proto.id`/`cost`/`prob`
// are SIGNED, `vnum0-4` are unsigned, matching neither "all signed" nor
// "all unsigned" naively (see db/item.rs's own comment for why this
// project checks every column instead of assuming).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RefineMaterial {
    pub vnum: u32,
    pub count: i16,
    /// Populated by `hydrate_material_names` - `None` if the vnum has no
    /// matching item_proto row (e.g. a recipe referencing a since-deleted
    /// item).
    pub locale_name: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RefineRecipe {
    pub id: i32,
    pub cost: i32,
    /// Success chance in percent (0-100), straight from `refine_proto.prob`.
    pub prob: i16,
    /// Only the populated slots (vnum != 0) - `refine_proto`'s 5 fixed
    /// vnum0-4/count0-4 columns are collapsed into this list so the
    /// frontend never has to know about the fixed-slot storage shape.
    pub materials: Vec<RefineMaterial>,
    /// How many item_proto rows currently point at this recipe via
    /// refine_set - shown in the editor so deleting/repurposing a shared
    /// recipe doesn't silently affect other items the user isn't looking
    /// at right now.
    pub used_by_count: i64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RefineChainNode {
    pub vnum: u32,
    pub locale_name: String,
    pub refine_set: u16,
    pub refined_vnum: u32,
    /// `None` when `refine_set == 0` (chain end) or the id doesn't resolve
    /// to any real `refine_proto` row (orphaned link - shown as a warning
    /// in the UI rather than silently treated as "no recipe").
    pub recipe: Option<RefineRecipe>,
}

fn row_to_recipe(row: &sqlx::mysql::MySqlRow, used_by_count: i64) -> RefineRecipe {
    let mut materials = Vec::new();
    for i in 0..5 {
        let vnum: u32 = row.try_get(format!("vnum{i}").as_str()).unwrap_or_default();
        let count: i16 = row.try_get(format!("count{i}").as_str()).unwrap_or_default();
        if vnum != 0 {
            materials.push(RefineMaterial { vnum, count, locale_name: None });
        }
    }
    RefineRecipe {
        id: row.try_get("id").unwrap_or_default(),
        cost: row.try_get("cost").unwrap_or_default(),
        prob: row.try_get("prob").unwrap_or_default(),
        materials,
        used_by_count,
    }
}

// Takes a plain `i32` matching `refine_proto.id`'s real column type -
// callers with a `u16` `refine_set` (item_proto's FK column, which can
// never exceed 65535) widen losslessly via `.into()`; a user-typed recipe
// id for the "reuse an existing recipe" UI can be any i32 without a lossy
// truncation landmine (an earlier version of this function took `u16`
// directly and silently wrapped a too-large typed id to the wrong recipe).
async fn fetch_recipe(pool: &MySqlPool, id: i32) -> Result<Option<RefineRecipe>, String> {
    if id <= 0 {
        return Ok(None);
    }
    let row = sqlx::query("SELECT * FROM player.refine_proto WHERE id = ?")
        .bind(id)
        .fetch_optional(pool)
        .await
        .map_err(|e| e.to_string())?;
    let Some(row) = row else {
        return Ok(None);
    };
    let used_by_count: i64 = sqlx::query("SELECT COUNT(*) AS c FROM player.item_proto WHERE refine_set = ?")
        .bind(id)
        .fetch_one(pool)
        .await
        .map_err(|e| e.to_string())?
        .try_get("c")
        .unwrap_or_default();
    Ok(Some(row_to_recipe(&row, used_by_count)))
}

async fn hydrate_material_names(pool: &MySqlPool, recipe: &mut RefineRecipe) -> Result<(), String> {
    for material in &mut recipe.materials {
        let name: Option<Vec<u8>> = sqlx::query("SELECT locale_name FROM player.item_proto WHERE vnum = ?")
            .bind(material.vnum)
            .fetch_optional(pool)
            .await
            .map_err(|e| e.to_string())?
            .map(|row| row.try_get("locale_name").unwrap_or_default());
        material.locale_name = name.map(|bytes| {
            let (text, _, _) = encoding_rs::WINDOWS_1252.decode(&bytes);
            text.trim().to_string()
        });
    }
    Ok(())
}

async fn fetch_node(pool: &MySqlPool, vnum: u32) -> Result<Option<RefineChainNode>, String> {
    let row = sqlx::query(
        "SELECT vnum, locale_name, refine_set, refined_vnum FROM player.item_proto WHERE vnum = ?",
    )
    .bind(vnum)
    .fetch_optional(pool)
    .await
    .map_err(|e| e.to_string())?;
    let Some(row) = row else {
        return Ok(None);
    };

    let name_raw: Vec<u8> = row.try_get("locale_name").unwrap_or_default();
    let (locale_name, _, _) = encoding_rs::WINDOWS_1252.decode(&name_raw);
    let refine_set: u16 = row.try_get("refine_set").unwrap_or_default();
    let refined_vnum: u32 = row.try_get("refined_vnum").unwrap_or_default();

    let mut recipe = fetch_recipe(pool, refine_set.into()).await?;
    if let Some(recipe) = &mut recipe {
        hydrate_material_names(pool, recipe).await?;
    }

    Ok(Some(RefineChainNode {
        vnum,
        locale_name: locale_name.trim().to_string(),
        refine_set,
        refined_vnum,
        recipe,
    }))
}

/// Builds the full upgrade chain containing `vnum`, in order from the base
/// item to the final (non-refinable) item. Walks backward first (any item
/// whose `refined_vnum` points at the current one) to find the chain's
/// start, then forward via `refined_vnum` to the end - so picking *any*
/// item in the middle of a chain (e.g. "Schwert+5") still shows the whole
/// chain, not just what comes after it. Both walks are capped at 30 steps
/// - refine chains are always short (+0..+9 at most on this server, per
/// the real data), so a longer walk almost certainly means a data error
/// (a cycle from a bad manual edit), and the cap turns that into an
/// incomplete-but-safe result instead of an infinite loop.
pub async fn get_refine_chain(pool: &MySqlPool, vnum: u32) -> Result<Vec<RefineChainNode>, String> {
    const MAX_STEPS: usize = 30;

    let mut start = vnum;
    for _ in 0..MAX_STEPS {
        let prev: Option<u32> = sqlx::query(
            "SELECT vnum FROM player.item_proto WHERE refined_vnum = ? ORDER BY vnum LIMIT 1",
        )
        .bind(start)
        .fetch_optional(pool)
        .await
        .map_err(|e| e.to_string())?
        .map(|row| row.try_get("vnum").unwrap_or_default());

        match prev {
            Some(p) if p != start => start = p,
            _ => break,
        }
    }

    let mut chain = Vec::new();
    let mut current = start;
    let mut seen = std::collections::HashSet::new();
    for _ in 0..MAX_STEPS {
        if !seen.insert(current) {
            break; // cycle guard
        }
        let Some(node) = fetch_node(pool, current).await? else {
            break;
        };
        let next = node.refined_vnum;
        chain.push(node);
        if next == 0 || next == current {
            break;
        }
        current = next;
    }

    if chain.is_empty() {
        return Err(format!("Item {vnum} nicht gefunden."));
    }
    Ok(chain)
}

/// Looks up one recipe by id with hydrated material names - used by the UI
/// to preview a recipe before reusing its id on another item (see
/// `set_item_refine_link`), separate from `get_refine_chain` since that
/// only resolves recipes reachable from a specific item's own chain.
pub async fn get_recipe(pool: &MySqlPool, id: i32) -> Result<Option<RefineRecipe>, String> {
    let Some(mut recipe) = fetch_recipe(pool, id).await? else {
        return Ok(None);
    };
    hydrate_material_names(pool, &mut recipe).await?;
    Ok(Some(recipe))
}

/// Writes (creates if `id` is `None`, replaces all fields if `Some`) one
/// recipe row. Materials shorter than 5 entries are padded with vnum=0 /
/// count=0, matching the fixed-slot storage `refine_proto` actually uses.
pub async fn save_recipe(
    pool: &MySqlPool,
    id: Option<i32>,
    cost: i32,
    prob: i16,
    materials: &[RefineMaterial],
) -> Result<i32, String> {
    if materials.len() > 5 {
        return Err("Maximal 5 Materialien pro Rezept möglich.".into());
    }
    let mut slots = [(0u32, 0i16); 5];
    for (i, m) in materials.iter().enumerate() {
        slots[i] = (m.vnum, m.count);
    }

    if let Some(id) = id {
        sqlx::query(
            "UPDATE player.refine_proto SET \
             vnum0=?, count0=?, vnum1=?, count1=?, vnum2=?, count2=?, \
             vnum3=?, count3=?, vnum4=?, count4=?, cost=?, prob=? WHERE id=?",
        )
        .bind(slots[0].0).bind(slots[0].1)
        .bind(slots[1].0).bind(slots[1].1)
        .bind(slots[2].0).bind(slots[2].1)
        .bind(slots[3].0).bind(slots[3].1)
        .bind(slots[4].0).bind(slots[4].1)
        .bind(cost).bind(prob).bind(id)
        .execute(pool)
        .await
        .map_err(|e| e.to_string())?;
        Ok(id)
    } else {
        let result = sqlx::query(
            "INSERT INTO player.refine_proto \
             (vnum0, count0, vnum1, count1, vnum2, count2, vnum3, count3, vnum4, count4, cost, src_vnum, result_vnum, prob) \
             VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, 0, 0, ?)",
        )
        .bind(slots[0].0).bind(slots[0].1)
        .bind(slots[1].0).bind(slots[1].1)
        .bind(slots[2].0).bind(slots[2].1)
        .bind(slots[3].0).bind(slots[3].1)
        .bind(slots[4].0).bind(slots[4].1)
        .bind(cost).bind(prob)
        .execute(pool)
        .await
        .map_err(|e| e.to_string())?;
        Ok(result.last_insert_id() as i32)
    }
}

/// Refuses to delete a recipe still referenced by any item - deleting it
/// out from under those items would leave their `refine_set` pointing at
/// nothing (silently "no recipe found" in-game, not an error the server
/// would ever surface), so the caller must first repoint or clear those
/// items' `refine_set` (via `set_item_refine_link`).
pub async fn delete_recipe(pool: &MySqlPool, id: i32) -> Result<(), String> {
    let used_by: i64 = sqlx::query("SELECT COUNT(*) AS c FROM player.item_proto WHERE refine_set = ?")
        .bind(id)
        .fetch_one(pool)
        .await
        .map_err(|e| e.to_string())?
        .try_get("c")
        .unwrap_or_default();
    if used_by > 0 {
        return Err(format!(
            "Rezept wird noch von {used_by} Item(s) verwendet - erst dort die Verknüpfung ändern."
        ));
    }
    sqlx::query("DELETE FROM player.refine_proto WHERE id = ?")
        .bind(id)
        .execute(pool)
        .await
        .map_err(|e| e.to_string())?;
    Ok(())
}

/// Points `vnum`'s upgrade step at `refine_set` (an existing recipe id, or
/// 0 to remove the recipe link) and `refined_vnum` (the target item, or 0
/// for "chain ends here"). Does not touch anything else on the item row -
/// deliberately narrower than `db/item.rs::update_item_proto`, which needs
/// the item's *entire* payload and would be a heavy, error-prone way to
/// change two columns.
pub async fn set_item_refine_link(
    pool: &MySqlPool,
    vnum: u32,
    refine_set: u16,
    refined_vnum: u32,
) -> Result<(), String> {
    let result = sqlx::query("UPDATE player.item_proto SET refine_set = ?, refined_vnum = ? WHERE vnum = ?")
        .bind(refine_set)
        .bind(refined_vnum)
        .bind(vnum)
        .execute(pool)
        .await
        .map_err(|e| e.to_string())?;
    if result.rows_affected() == 0 {
        return Err(format!("Item {vnum} existiert nicht."));
    }
    Ok(())
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ShopSource {
    pub shop_vnum: i32,
    pub shop_name: String,
    pub npc_vnum: i16,
    pub count: i32,
}

/// Reverse lookup for the material-source hint: which shops sell `vnum`
/// directly, joined to the shop's own name/NPC for display. Mirrors the
/// join `db/shop.rs` already uses elsewhere in this project (`shop_item`
/// -> `shop`), just in the opposite direction (by item, not by shop).
pub async fn find_shop_sources(pool: &MySqlPool, vnum: u32) -> Result<Vec<ShopSource>, String> {
    let rows = sqlx::query(
        "SELECT s.vnum AS shop_vnum, s.name AS shop_name, s.npc_vnum, si.count \
         FROM player.shop_item si JOIN player.shop s ON si.shop_vnum = s.vnum \
         WHERE si.item_vnum = ? ORDER BY s.name",
    )
    .bind(vnum)
    .fetch_all(pool)
    .await
    .map_err(|e| e.to_string())?;

    Ok(rows
        .iter()
        .map(|row| ShopSource {
            shop_vnum: row.try_get("shop_vnum").unwrap_or_default(),
            shop_name: row.try_get("shop_name").unwrap_or_default(),
            npc_vnum: row.try_get("npc_vnum").unwrap_or_default(),
            count: row.try_get("count").unwrap_or_default(),
        })
        .collect())
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
            conn.query_row("SELECT value FROM paths WHERE key = ?1", [key], |r| r.get(0))
                .ok()
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

    // Regression test for the exact confusion that motivated this feature:
    // item 180's refine_set (19) looks like it could be a required
    // material vnum, but it's actually just the recipe id and recipe 19
    // requires no items at all (verified live against this server's real
    // data, 2026-08-06) - the chain must reflect that faithfully, not
    // invent a material that doesn't exist.
    #[tokio::test]
    async fn chain_for_vnum_180_has_no_materials_but_real_recipe() {
        let pool = real_pool().await;
        let chain = get_refine_chain(&pool, 180).await.expect("chain fetch failed");

        let node180 = chain.iter().find(|n| n.vnum == 180).expect("180 must be in its own chain");
        assert_eq!(node180.refined_vnum, 181);
        let recipe = node180.recipe.as_ref().expect("180 must have a resolvable recipe");
        assert_eq!(recipe.id, 19);
        assert_eq!(recipe.prob, 90);
        assert_eq!(recipe.cost, 1200);
        assert!(recipe.materials.is_empty(), "recipe 19 genuinely requires no items");
    }

    // The well-known stock sword chain (10 Schwert+0 .. 19 Schwert+9),
    // verified live to be exactly 10 steps ending with refined_vnum=0.
    #[tokio::test]
    async fn chain_for_vnum_15_covers_the_whole_sword_chain() {
        let pool = real_pool().await;
        let chain = get_refine_chain(&pool, 15).await.expect("chain fetch failed");

        let vnums: Vec<u32> = chain.iter().map(|n| n.vnum).collect();
        assert_eq!(vnums, (10..=19).collect::<Vec<_>>(), "must include the full chain, not just from 15 onward");
        assert_eq!(chain.last().unwrap().refined_vnum, 0, "chain must end at the non-refinable item");
        // Item vnum and recipe id are unrelated (refine_set is an opaque
        // recipe id, not a vnum) - this chain's own steps happen to use
        // recipe ids 1-9 in order, purely coincidentally.
        assert_eq!(chain[0].refine_set, 1);
    }

    // A different chain whose recipe actually requires a material - proves
    // material hydration end-to-end, deliberately using an item vnum that
    // does NOT match its own recipe id (vnum 66's refine_set is 16), the
    // exact mismatch that makes reading refine_proto directly confusing.
    #[tokio::test]
    async fn chain_step_with_real_material_gets_its_name_hydrated() {
        let pool = real_pool().await;
        let chain = get_refine_chain(&pool, 66).await.expect("chain fetch failed");

        let node = chain.iter().find(|n| n.vnum == 66).expect("66 must be in its own chain");
        assert_eq!(node.refine_set, 16, "recipe id, not related to the item's own vnum 66");
        assert_eq!(node.refined_vnum, 67);
        let recipe = node.recipe.as_ref().unwrap();
        assert_eq!(recipe.id, 16);
        assert_eq!(recipe.materials.len(), 1);
        assert_eq!(recipe.materials[0].vnum, 27799);
        assert!(recipe.materials[0].locale_name.is_some(), "material name must be hydrated");
    }

    #[tokio::test]
    async fn recipe_used_by_count_reflects_real_sharing() {
        let pool = real_pool().await;
        // Live-verified: recipe id 1 is shared by 8 different items.
        let recipe = fetch_recipe(&pool, 1i32).await.unwrap().expect("recipe 1 must exist");
        assert_eq!(recipe.used_by_count, 8);
    }
}
