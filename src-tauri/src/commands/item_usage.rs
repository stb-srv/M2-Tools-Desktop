//! Item-Abhängigkeits-Übersicht ("Wo wird das benutzt?") - Idee #3 der
//! Ideen-Session vom 2026-08-24 (siehe [[m2manager_activity_log]]-
//! Nachfolgeplan). Fasst 6 unabhängige Quellen (Shops, mob_drop_item.txt/
//! common_drop_item.txt, etc_drop_item.txt, drop_item_group.txt,
//! special_item_group.txt, cube.txt, Aufwertungs-Rezepte, Quests) zu einem Bericht
//! zusammen, ruft dafür ausschließlich bereits bestehende, andernorts
//! verifizierte Parser/Queries auf statt selbst zu parsen. Eine nicht
//! erreichbare/nicht konfigurierte Quelle landet als Warnung im Bericht statt
//! den ganzen Befehl fehlschlagen zu lassen - SSH bzw. MySQL werden dafür
//! jeweils nur einmal aufgelöst, nicht pro Datei neu.

use super::support::{require_pool, stored_ssh_auth};
use crate::common_drop;
use crate::cube;
use crate::db::item;
use crate::db::shop::{self, ShopUsage};
use crate::drop_item_group;
use crate::etc_drop;
use crate::mobdrop;
use crate::quest;
use crate::quest_item_refs::{self, QuestItemRef};
use crate::refine::{self, RefineMaterialUsage};
use crate::settings;
use crate::special_item_group;
use crate::ssh::{self, SshAuth, SshConfig};
use crate::state::AppState;
use serde::Serialize;
use tauri::State;

#[derive(Debug, Clone, Serialize)]
pub struct MobDropUsage {
    pub source: String, // "mob_drop_item.txt"
    pub mob_vnum: u32,
    pub group_name: String,
    pub percent: f64,
}

/// Ein aktiver Rang-Block in common_drop_item.txt.
#[derive(Debug, Clone, Serialize)]
pub struct CommonDropUsage {
    pub rank: String,
    pub level_from: i64,
    pub level_to: i64,
    pub chance: f64,
}

#[derive(Debug, Clone, Serialize)]
pub struct DropGroupUsage {
    pub mob_vnum: u32,
    pub group_name: String,
    pub percent: f64,
}

#[derive(Debug, Clone, Serialize)]
pub struct BoxRewardUsage {
    pub group_name: String,
    pub box_vnum: i32,
}

#[derive(Debug, Clone, Serialize)]
pub struct CubeUsage {
    pub npc_vnums: Vec<u32>,
    pub percent: i32,
}

#[derive(Debug, Clone, Serialize)]
pub struct EtcDropUsage {
    pub percent: f64,
}

#[derive(Debug, Clone, Default, Serialize)]
pub struct ItemUsageReport {
    pub shops: Vec<ShopUsage>,
    pub mob_drops: Vec<MobDropUsage>,
    pub common_drops: Vec<CommonDropUsage>,
    pub etc_drop: Option<EtcDropUsage>,
    pub drop_item_groups: Vec<DropGroupUsage>,
    pub is_box_of: Option<String>,
    pub possible_reward_in: Vec<BoxRewardUsage>,
    pub cube_ingredient_in: Vec<CubeUsage>,
    pub cube_reward_in: Vec<CubeUsage>,
    pub refine_materials: Vec<RefineMaterialUsage>,
    /// Items, die zu diesem Item aufgewertet werden (refined_vnum).
    pub refine_from: Vec<(u32, String)>,
    /// Quest-Stellen, an denen die VNUM erkennbar als Item steht.
    pub quest_refs: Vec<QuestItemRef>,
    /// Quest-Stellen, an denen die Zahl eigenständig steht, aber nicht als
    /// Item erkennbar ist (z.B. in einer Tabelle) - nur als Hinweis.
    pub quest_unconfirmed: Vec<QuestItemRef>,
    pub warnings: Vec<String>,
}

fn path_setting(state: &State<'_, AppState>, key: &str, default: &str) -> Result<String, String> {
    let conn = state.settings_db.lock().map_err(|e| e.to_string())?;
    Ok(settings::get_path(&conn, key)?
        .filter(|v| !v.is_empty())
        .unwrap_or_else(|| default.to_string()))
}

async fn read_optional_file(
    config: &SshConfig,
    auth: &SshAuth,
    path: &str,
) -> Result<Option<String>, String> {
    ssh::read_remote_file_if_exists(config, auth, path).await
}

#[tauri::command]
pub async fn find_item_usages(state: State<'_, AppState>, vnum: u32) -> Result<ItemUsageReport, String> {
    let mut report = ItemUsageReport::default();

    // ---- DB-Quellen (Shops, etc_drop-Namensauflösung) ----
    let pool = require_pool(&state).await;
    match &pool {
        Ok(pool) => match shop::find_shops_selling_item(pool, vnum).await {
            Ok(shops) => report.shops = shops,
            Err(e) => report.warnings.push(format!("Shops konnten nicht geprüft werden: {e}")),
        },
        Err(e) => report.warnings.push(format!("Shops konnten nicht geprüft werden: {e}")),
    }

    if let Ok(pool) = &pool {
        match refine::find_recipes_using_material(pool, vnum).await {
            Ok(r) => report.refine_materials = r,
            Err(e) => report.warnings.push(format!("Aufwertungs-Rezepte konnten nicht geprüft werden: {e}")),
        }
        match refine::find_refine_predecessors(pool, vnum).await {
            Ok(r) => report.refine_from = r,
            Err(e) => report.warnings.push(format!("Aufwertungs-Kette konnte nicht geprüft werden: {e}")),
        }
    }

    let internal_name: Option<String> = match &pool {
        Ok(pool) => match item::get_item_internal_name(pool, vnum).await {
            Ok(name) => Some(name),
            Err(_) => None, // vnum ist gar kein bekanntes Item - etc_drop kann dann ohnehin nichts referenzieren
        },
        Err(e) => {
            report
                .warnings
                .push(format!("etc_drop_item.txt konnte nicht geprüft werden (keine DB-Verbindung): {e}"));
            None
        }
    };

    // ---- SSH-Quellen (alle übrigen 5) ----
    let ssh_auth = stored_ssh_auth(&state);
    let Ok((config, auth)) = ssh_auth else {
        report.warnings.push(format!(
            "Drop-/Kisten-/Cube-/Quest-Dateien konnten nicht geprüft werden: {}",
            ssh_auth.err().unwrap()
        ));
        return Ok(report);
    };

    // mob_drop_item.txt
    match path_setting(&state, "mob_drop_file_path", "/usr/home/game/share/mob_drop_item.txt") {
        Ok(path) => match read_optional_file(&config, &auth, &path).await {
            Ok(Some(content)) => match mobdrop::parse(&content) {
                Ok(groups) => {
                    for g in &groups {
                        for it in &g.items {
                            if it.item_vnum == vnum {
                                report.mob_drops.push(MobDropUsage {
                                    source: "mob_drop_item.txt".to_string(),
                                    mob_vnum: g.mob_vnum,
                                    group_name: g.name.clone(),
                                    percent: it.percent,
                                });
                            }
                        }
                    }
                }
                Err(e) => report.warnings.push(format!("mob_drop_item.txt konnte nicht gelesen werden: {e}")),
            },
            Ok(None) => {}
            Err(e) => report.warnings.push(format!("mob_drop_item.txt konnte nicht gelesen werden: {e}")),
        },
        Err(e) => report.warnings.push(format!("mob_drop_item.txt: {e}")),
    }

    // common_drop_item.txt (Rang-Format, siehe common_drop.rs)
    match path_setting(&state, "common_drop_file_path", "/usr/home/game/share/common_drop_item.txt") {
        Ok(path) => match read_optional_file(&config, &auth, &path).await {
            Ok(Some(content)) => {
                let file = common_drop::parse(&content);
                let mut invalid = 0;
                for line in &file.lines {
                    match line {
                        common_drop::CommonDropLine::Rule { blocks } => {
                            for (r, b) in blocks.iter().enumerate() {
                                if b.vnum == vnum && b.is_active() {
                                    report.common_drops.push(CommonDropUsage {
                                        rank: common_drop::RANK_NAMES.get(r).unwrap_or(&"?").to_string(),
                                        level_from: b.level_from,
                                        level_to: b.level_to,
                                        chance: b.chance,
                                    });
                                }
                            }
                        }
                        common_drop::CommonDropLine::Invalid { .. } => invalid += 1,
                        common_drop::CommonDropLine::Text { .. } => {}
                    }
                }
                if invalid > 0 {
                    report
                        .warnings
                        .push(format!("common_drop_item.txt: {invalid} fehlerhafte Zeile(n) übersprungen."));
                }
            }
            Ok(None) => {}
            Err(e) => report.warnings.push(format!("common_drop_item.txt konnte nicht gelesen werden: {e}")),
        },
        Err(e) => report.warnings.push(format!("common_drop_item.txt: {e}")),
    }

    // etc_drop_item.txt - braucht den intern aufgelösten Namen von oben
    if let Some(name) = &internal_name {
        match path_setting(&state, "etc_drop_file_path", "/usr/home/game/share/etc_drop_item.txt") {
            Ok(path) => match read_optional_file(&config, &auth, &path).await {
                Ok(Some(content)) => match etc_drop::parse(&content) {
                    Ok(entries) => {
                        if let Some(entry) = entries.iter().find(|e| &e.item_name == name) {
                            report.etc_drop = Some(EtcDropUsage { percent: entry.percent });
                        }
                    }
                    Err(e) => report.warnings.push(format!("etc_drop_item.txt konnte nicht gelesen werden: {e}")),
                },
                Ok(None) => {}
                Err(e) => report.warnings.push(format!("etc_drop_item.txt konnte nicht gelesen werden: {e}")),
            },
            Err(e) => report.warnings.push(format!("etc_drop_item.txt: {e}")),
        }
    }

    // drop_item_group.txt
    match path_setting(&state, "drop_item_group_file_path", "/usr/home/game/share/drop_item_group.txt") {
        Ok(path) => match read_optional_file(&config, &auth, &path).await {
            Ok(Some(content)) => match drop_item_group::parse(&content) {
                Ok(groups) => {
                    for g in &groups {
                        for it in &g.items {
                            if it.item_ref.parse::<u32>() == Ok(vnum) {
                                report.drop_item_groups.push(DropGroupUsage {
                                    mob_vnum: g.mob_vnum,
                                    group_name: g.name.clone(),
                                    percent: it.percent,
                                });
                            }
                        }
                    }
                }
                Err(e) => report.warnings.push(format!("drop_item_group.txt konnte nicht gelesen werden: {e}")),
            },
            Ok(None) => {}
            Err(e) => report.warnings.push(format!("drop_item_group.txt konnte nicht gelesen werden: {e}")),
        },
        Err(e) => report.warnings.push(format!("drop_item_group.txt: {e}")),
    }

    // special_item_group.txt (Kisten) - zwei getrennte Fragen: ist `vnum`
    // selbst eine Kiste, und/oder kommt `vnum` irgendwo als möglicher
    // Kisten-Inhalt vor.
    match path_setting(&state, "special_item_group_file_path", "/usr/home/game/share/special_item_group.txt") {
        Ok(path) => match read_optional_file(&config, &auth, &path).await {
            Ok(Some(content)) => match special_item_group::parse(&content) {
                Ok(groups) => {
                    for g in &groups {
                        if g.vnum == vnum as i32 {
                            report.is_box_of = Some(g.name.clone());
                        }
                        for entry in &g.entries {
                            if entry.item_ref.parse::<u32>() == Ok(vnum) {
                                report.possible_reward_in.push(BoxRewardUsage {
                                    group_name: g.name.clone(),
                                    box_vnum: g.vnum,
                                });
                            }
                        }
                    }
                }
                Err(e) => report.warnings.push(format!("special_item_group.txt konnte nicht gelesen werden: {e}")),
            },
            Ok(None) => {}
            Err(e) => report.warnings.push(format!("special_item_group.txt konnte nicht gelesen werden: {e}")),
        },
        Err(e) => report.warnings.push(format!("special_item_group.txt: {e}")),
    }

    // cube.txt - getrennt nach Zutat vs. Belohnung
    match path_setting(&state, "cube_file_path", "/usr/home/game/share/cube.txt") {
        Ok(path) => match read_optional_file(&config, &auth, &path).await {
            Ok(Some(content)) => match cube::parse(&content) {
                Ok(recipes) => {
                    for r in &recipes {
                        if r.items.iter().any(|v| v.vnum == vnum) {
                            report.cube_ingredient_in.push(CubeUsage {
                                npc_vnums: r.npc_vnums.clone(),
                                percent: r.percent,
                            });
                        }
                        if r.rewards.iter().any(|v| v.vnum == vnum) {
                            report.cube_reward_in.push(CubeUsage {
                                npc_vnums: r.npc_vnums.clone(),
                                percent: r.percent,
                            });
                        }
                    }
                }
                Err(e) => report.warnings.push(format!("cube.txt konnte nicht gelesen werden: {e}")),
            },
            Ok(None) => {}
            Err(e) => report.warnings.push(format!("cube.txt konnte nicht gelesen werden: {e}")),
        },
        Err(e) => report.warnings.push(format!("cube.txt: {e}")),
    }

    // Quests - nur eigenständige Zahlen außerhalb von Texten/Kommentaren,
    // getrennt nach erkennbarem Item-Bezug und ungeprüft (quest_item_refs.rs).
    match path_setting(&state, "quest_dir", "/usr/home/game/share/quest") {
        Ok(dir) => match read_optional_file(&config, &auth, &format!("{dir}/quest_list")).await {
            Ok(Some(list_content)) => {
                let files = quest::parse_quest_list(&list_content);
                let paths: Vec<String> = files.iter().map(|f| format!("{dir}/{}", f.relative_path)).collect();
                match ssh::read_remote_files(&config, &auth, &paths).await {
                    Ok(contents) => {
                        let scan = quest_item_refs::scan_files(&files, &contents, vnum);
                        report.quest_refs = scan.confirmed;
                        report.quest_unconfirmed = scan.unconfirmed;
                    }
                    Err(e) => report.warnings.push(format!("Quest-Dateien konnten nicht durchsucht werden: {e}")),
                }
            }
            Ok(None) => {}
            Err(e) => report.warnings.push(format!("Quest-Verzeichnis konnte nicht gelesen werden: {e}")),
        },
        Err(e) => report.warnings.push(format!("Quest-Verzeichnis: {e}")),
    }

    Ok(report)
}
