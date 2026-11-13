//! Drop-Check über alle 5 Drop-Dateien, Änderungsvorschau vor dem
//! Hochladen und „Was droppt dieser Mob?“. Die reine Logik steht in
//! `crate::drop_check`, hier nur Dateien lesen und Datenbank fragen.

use super::support::{require_pool, stored_ssh_auth};
use crate::common_drop;
use crate::db::{item, mob};
use crate::drop_check::{self, DbFacts, DropCheckIssue, DropFileKind, DropRef, ItemFacts};
use crate::drop_item_group;
use crate::etc_drop;
use crate::mobdrop;
use crate::settings;
use crate::special_item_group;
use crate::ssh::{self, SshAuth, SshConfig};
use crate::state::AppState;
use serde::Serialize;
use std::collections::HashMap;
use tauri::State;

pub(super) fn drop_file_path(state: &State<'_, AppState>, kind: DropFileKind) -> Result<String, String> {
    let conn = state.settings_db.lock().map_err(|e| e.to_string())?;
    Ok(settings::get_path(&conn, kind.settings_key())?
        .filter(|v| !v.is_empty())
        .unwrap_or_else(|| kind.default_path()))
}

/// Fragt die Datenbank nach allem, was `refs` braucht.
async fn db_facts(pool: &sqlx::MySqlPool, refs: &[DropRef]) -> Result<DbFacts, String> {
    let mut facts = DbFacts::default();

    let vnums: Vec<u32> = drop_check::item_vnums(refs).into_iter().collect();
    if !vnums.is_empty() {
        let existing = item::existing_item_vnums(pool, &vnums).await?;
        let as_names: Vec<String> = vnums.iter().map(|v| v.to_string()).collect();
        let collisions = item::items_by_internal_names(pool, &as_names).await?;
        for v in vnums {
            let name_collision = collisions
                .get(&v.to_string())
                .filter(|other| other.vnum != v)
                .map(|other| (other.vnum, other.locale_name.clone()));
            facts.items.insert(v, ItemFacts { exists: existing.contains(&v), name_collision });
        }
    }

    let names: Vec<String> = drop_check::item_names(refs).into_iter().collect();
    if !names.is_empty() {
        let found = item::items_by_internal_names(pool, &names).await?;
        for n in names {
            let exists = found.contains_key(&n.to_lowercase());
            facts.item_names.insert(n, exists);
        }
    }

    let mobs: Vec<u32> = drop_check::mob_vnums(refs).into_iter().collect();
    if !mobs.is_empty() {
        let existing = mob::existing_mob_vnums(pool, &mobs).await?;
        for m in mobs {
            facts.mobs.insert(m, existing.contains(&m));
        }
    }
    Ok(facts)
}

/// Sperre vor dem Speichern: verweist die Datei auf Items, die der Server
/// nicht findet, wird nicht geschrieben. Ohne Datenbankverbindung lässt
/// sich das nicht prüfen - dann wird wie bisher gespeichert.
pub(super) async fn guard_items(state: &State<'_, AppState>, kind: DropFileKind, refs: Vec<DropRef>) -> Result<(), String> {
    let Ok(pool) = require_pool(state).await else { return Ok(()) };
    let facts = db_facts(&pool, &refs).await?;
    match drop_check::blocking_message(&drop_check::evaluate(kind, &refs, &facts)) {
        Some(msg) => Err(msg),
        None => Ok(()),
    }
}

/// Liest eine Datei-Art und liefert ihre Verweise, die Anzahl Einträge und
/// Strukturfehler (z.B. kaputte Zeilen).
fn refs_from_content(kind: DropFileKind, content: &str) -> Result<(Vec<DropRef>, usize, Vec<DropCheckIssue>), String> {
    let file = kind.file_name().to_string();
    Ok(match kind {
        DropFileKind::MobDrop => {
            let groups = mobdrop::parse(content)?;
            (drop_check::refs_mob_drop(&groups), groups.len(), vec![])
        }
        DropFileKind::CommonDrop => {
            let parsed = common_drop::parse(content);
            let structural = common_drop::validate(&parsed)
                .into_iter()
                .filter(|i| i.level == common_drop::IssueLevel::Error)
                .map(|i| DropCheckIssue {
                    file: file.clone(),
                    location: format!("Zeile {}", i.line_index + 1),
                    level: "error".into(),
                    message: i.message,
                })
                .collect();
            let rules = parsed.lines.iter().filter(|l| matches!(l, common_drop::CommonDropLine::Rule { .. })).count();
            (drop_check::refs_common_drop(&parsed), rules, structural)
        }
        DropFileKind::EtcDrop => {
            let entries = etc_drop::parse(content)?;
            (drop_check::refs_etc_drop(&entries), entries.len(), vec![])
        }
        DropFileKind::SpecialItemGroup => {
            let groups = special_item_group::parse(content)?;
            (drop_check::refs_special_item_group(&groups), groups.len(), vec![])
        }
        DropFileKind::DropItemGroup => {
            let groups = drop_item_group::parse(content)?;
            (drop_check::refs_drop_item_group(&groups), groups.len(), vec![])
        }
    })
}

#[derive(Debug, Clone, Serialize)]
pub struct DropFileStatus {
    pub kind: DropFileKind,
    pub file: String,
    /// "ok" | "missing" | "unreadable"
    pub state: String,
    pub detail: Option<String>,
    pub entries: usize,
}

#[derive(Debug, Clone, Serialize)]
pub struct DropCheckReport {
    pub files: Vec<DropFileStatus>,
    pub issues: Vec<DropCheckIssue>,
    /// `None` = Items/Mobs wurden gegen die Datenbank geprüft.
    pub db_error: Option<String>,
}

pub(super) async fn build_drop_check_report(
    state: &State<'_, AppState>,
    config: &SshConfig,
    auth: &SshAuth,
) -> Result<DropCheckReport, String> {
    let mut files = Vec::new();
    let mut issues = Vec::new();
    let mut per_file_refs = Vec::new();

    for kind in DropFileKind::ALL {
        let path = drop_file_path(state, kind)?;
        let mut status = DropFileStatus { kind, file: kind.file_name().into(), state: "ok".into(), detail: None, entries: 0 };
        match ssh::read_remote_file_if_exists(config, auth, &path).await {
            Ok(None) => {
                status.state = "missing".into();
                status.detail = Some(format!("{path} gibt es nicht."));
            }
            Ok(Some(content)) => match refs_from_content(kind, &content) {
                Ok((refs, entries, structural)) => {
                    status.entries = entries;
                    issues.extend(structural);
                    per_file_refs.push((kind, refs));
                }
                Err(e) => {
                    status.state = "unreadable".into();
                    status.detail = Some(e.clone());
                    issues.push(DropCheckIssue {
                        file: kind.file_name().into(),
                        location: "Datei".into(),
                        level: "error".into(),
                        message: format!("Lässt sich nicht lesen: {e}"),
                    });
                }
            },
            Err(e) => {
                status.state = "unreadable".into();
                status.detail = Some(e);
            }
        }
        files.push(status);
    }

    let all_refs: Vec<DropRef> = per_file_refs.iter().flat_map(|(_, r)| r.iter().cloned()).collect();
    let db_error = match require_pool(state).await {
        Ok(pool) => match db_facts(&pool, &all_refs).await {
            Ok(facts) => {
                for (kind, refs) in &per_file_refs {
                    issues.extend(drop_check::evaluate(*kind, refs, &facts));
                }
                None
            }
            Err(e) => Some(e),
        },
        Err(e) => Some(e),
    };

    Ok(DropCheckReport { files, issues, db_error })
}

#[tauri::command]
pub async fn run_drop_check(state: State<'_, AppState>) -> Result<DropCheckReport, String> {
    let (config, auth) = stored_ssh_auth(&state)?;
    build_drop_check_report(&state, &config, &auth).await
}

// ---- Änderungsvorschau ----

#[derive(Debug, Clone, Serialize)]
pub struct FilePreview {
    pub file: String,
    pub path: String,
    /// `None` = Datei gibt es auf dem Server noch nicht.
    pub old: Option<String>,
    pub new: String,
}

/// Erzeugt die Datei, so wie das Speichern sie schreiben würde, und liest
/// die aktuelle Fassung vom Server - ohne etwas zu schreiben.
#[tauri::command]
pub async fn preview_drop_file(
    state: State<'_, AppState>,
    kind: String,
    data: serde_json::Value,
) -> Result<FilePreview, String> {
    let kind = DropFileKind::from_id(&kind).ok_or_else(|| format!("Unbekannte Datei-Art: {kind}"))?;
    let bad = |e: serde_json::Error| format!("Ungültige Daten: {e}");
    let new = match kind {
        DropFileKind::MobDrop => mobdrop::serialize(&serde_json::from_value::<Vec<mobdrop::MobDropGroup>>(data).map_err(bad)?),
        DropFileKind::CommonDrop => {
            common_drop::serialize(&serde_json::from_value::<common_drop::CommonDropFile>(data).map_err(bad)?)?
        }
        DropFileKind::EtcDrop => etc_drop::serialize(&serde_json::from_value::<Vec<etc_drop::EtcDropEntry>>(data).map_err(bad)?),
        DropFileKind::SpecialItemGroup => special_item_group::serialize(
            &serde_json::from_value::<Vec<special_item_group::SpecialItemGroup>>(data).map_err(bad)?,
        ),
        DropFileKind::DropItemGroup => drop_item_group::serialize(
            &serde_json::from_value::<Vec<drop_item_group::DropItemGroup>>(data).map_err(bad)?,
        ),
    };
    let (config, auth) = stored_ssh_auth(&state)?;
    let path = drop_file_path(&state, kind)?;
    let old = ssh::read_remote_file_if_exists(&config, &auth, &path).await?;
    Ok(FilePreview { file: kind.file_name().into(), path, old, new })
}

// ---- Was droppt dieser Mob? ----

#[derive(Debug, Clone, Serialize)]
pub struct MobDropSource {
    /// "mob_drop" | "drop_item_group" | "common_drop"
    pub source: String,
    pub group: String,
    pub item_vnum: u32,
    pub item_name: Option<String>,
    /// Datei-Wert (real ≈ ÷ 4).
    pub chance: f64,
    pub count: i64,
    /// Nur bei common_drop: Spieler-Level-Bereich.
    pub level_from: Option<i64>,
    pub level_to: Option<i64>,
    /// mob_drop: "drop", "kill", "limit", ...
    pub drop_type: Option<String>,
}

#[derive(Debug, Clone, Serialize)]
pub struct MobDropReport {
    pub mob: Option<mob::MobBrief>,
    pub drops: Vec<MobDropSource>,
    pub warnings: Vec<String>,
}

#[tauri::command]
pub async fn get_mob_brief(state: State<'_, AppState>, vnum: u32) -> Result<Option<mob::MobBrief>, String> {
    let pool = require_pool(&state).await?;
    mob::get_mob_brief(&pool, vnum).await
}

#[tauri::command]
pub async fn find_mob_drops(state: State<'_, AppState>, vnum: u32) -> Result<MobDropReport, String> {
    let mut warnings = Vec::new();
    let pool = require_pool(&state).await;
    let brief = match &pool {
        Ok(pool) => match mob::get_mob_brief(pool, vnum).await {
            Ok(b) => b,
            Err(e) => {
                warnings.push(format!("Mob konnte nicht gelesen werden: {e}"));
                None
            }
        },
        Err(e) => {
            warnings.push(format!("Keine Datenbank: Rang und Level unbekannt, Common-Drops können nicht zugeordnet werden ({e})."));
            None
        }
    };

    let (config, auth) = stored_ssh_auth(&state)?;
    let mut drops = Vec::new();
    let read = |kind: DropFileKind| {
        let path = drop_file_path(&state, kind);
        let config = config.clone();
        let auth = auth.clone();
        async move {
            let path = path?;
            ssh::read_remote_file_if_exists(&config, &auth, &path).await
        }
    };

    match read(DropFileKind::MobDrop).await {
        Ok(Some(content)) => match mobdrop::parse(&content) {
            Ok(groups) => {
                for g in groups.iter().filter(|g| g.mob_vnum == vnum) {
                    for it in &g.items {
                        drops.push(MobDropSource {
                            source: "mob_drop".into(),
                            group: g.name.clone(),
                            item_vnum: it.item_vnum,
                            item_name: None,
                            chance: it.percent,
                            count: it.count as i64,
                            level_from: None,
                            level_to: None,
                            drop_type: Some(g.drop_type.clone()),
                        });
                    }
                }
            }
            Err(e) => warnings.push(format!("mob_drop_item.txt: {e}")),
        },
        Ok(None) => {}
        Err(e) => warnings.push(format!("mob_drop_item.txt: {e}")),
    }

    match read(DropFileKind::DropItemGroup).await {
        Ok(Some(content)) => match drop_item_group::parse(&content) {
            Ok(groups) => {
                for g in groups.iter().filter(|g| g.mob_vnum == vnum) {
                    for it in &g.items {
                        drops.push(MobDropSource {
                            source: "drop_item_group".into(),
                            group: g.name.clone(),
                            item_vnum: it.item_ref.trim().parse().unwrap_or(0),
                            item_name: it.item_ref.trim().parse::<u32>().is_err().then(|| it.item_ref.clone()),
                            chance: it.percent,
                            count: it.count as i64,
                            level_from: None,
                            level_to: None,
                            drop_type: None,
                        });
                    }
                }
            }
            Err(e) => warnings.push(format!("drop_item_group.txt: {e}")),
        },
        Ok(None) => {}
        Err(e) => warnings.push(format!("drop_item_group.txt: {e}")),
    }

    let block = brief.as_ref().and_then(|b| b.rank.as_deref()).and_then(mob::common_drop_block);
    if let Some(r) = block {
        match read(DropFileKind::CommonDrop).await {
            Ok(Some(content)) => {
                let file = common_drop::parse(&content);
                for (li, line) in file.lines.iter().enumerate() {
                    if let common_drop::CommonDropLine::Rule { blocks } = line {
                        if let Some(b) = blocks.get(r).filter(|b| b.is_active()) {
                            drops.push(MobDropSource {
                                source: "common_drop".into(),
                                group: format!("Zeile {}", li + 1),
                                item_vnum: b.vnum,
                                item_name: None,
                                chance: b.chance,
                                count: 1,
                                level_from: Some(b.level_from),
                                level_to: Some(b.level_to),
                                drop_type: None,
                            });
                        }
                    }
                }
            }
            Ok(None) => {}
            Err(e) => warnings.push(format!("common_drop_item.txt: {e}")),
        }
    }

    if let Ok(pool) = &pool {
        let vnums: Vec<u32> = drops.iter().map(|d| d.item_vnum).filter(|v| *v != 0).collect();
        if let Ok(names) = item::item_locale_names(pool, &vnums).await {
            let names: HashMap<u32, String> = names;
            for d in drops.iter_mut() {
                if d.item_name.is_none() {
                    d.item_name = names.get(&d.item_vnum).cloned();
                }
            }
        }
    }

    Ok(MobDropReport { mob: brief, drops, warnings })
}
