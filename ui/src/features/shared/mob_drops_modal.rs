//! „Was droppt dieser Mob?“ - alle Drops eines Mobs aus mob_drop_item.txt,
//! drop_item_group.txt und (über Rang und Spieler-Level) common_drop_item.txt
//! in einer Liste (`find_mob_drops`).

use crate::components::{Button, ButtonVariant, Icon};
use crate::features::shared::drop_chance::format_real_drop_chance;
use crate::tauri::invoke;
use leptos::prelude::*;
use leptos::task::spawn_local;
use serde::Deserialize;
use serde_json::json;

#[derive(Clone, Debug, Deserialize, PartialEq)]
pub struct MobBrief {
    pub vnum: u32,
    pub name: String,
    pub rank: Option<String>,
    pub level: Option<i64>,
}

#[derive(Clone, Debug, Deserialize)]
struct MobDropSource {
    source: String,
    group: String,
    item_vnum: u32,
    item_name: Option<String>,
    chance: f64,
    count: i64,
    level_from: Option<i64>,
    level_to: Option<i64>,
    drop_type: Option<String>,
}

#[derive(Clone, Debug, Deserialize)]
struct MobDropReport {
    mob: Option<MobBrief>,
    drops: Vec<MobDropSource>,
    warnings: Vec<String>,
}

pub fn rank_label(rank: &str) -> &'static str {
    match rank {
        "PAWN" => "normaler Mob",
        "S_PAWN" => "stärkerer Mob",
        "KNIGHT" => "Elite",
        "S_KNIGHT" => "starke Elite",
        "BOSS" => "Boss",
        "KING" => "End-Boss",
        _ => "",
    }
}

fn source_title(source: &str) -> &'static str {
    match source {
        "mob_drop" => "Mob-Drops (mob_drop_item.txt)",
        "drop_item_group" => "Zufalls-Gruppen (drop_item_group.txt)",
        "common_drop" => "Common-Drops nach Rang (common_drop_item.txt)",
        _ => "Sonstige",
    }
}

#[component]
pub fn MobDropsModal(vnum: u32, #[prop(into)] on_close: Callback<()>) -> impl IntoView {
    let report = RwSignal::new(None::<Result<MobDropReport, String>>);
    spawn_local(async move {
        report.set(Some(invoke::<MobDropReport>("find_mob_drops", &json!({ "vnum": vnum })).await));
    });

    let section = move |r: &MobDropReport, source: &'static str| {
        let rows: Vec<MobDropSource> = r.drops.iter().filter(|d| d.source == source).cloned().collect();
        let is_common = source == "common_drop";
        let rank = r.mob.as_ref().and_then(|m| m.rank.clone());
        let empty_text = if is_common {
            match rank.as_deref() {
                Some("BOSS") | Some("KING") => "Bosse bekommen keine Common-Drops.".to_string(),
                Some(_) => "Keine aktive Regel für diesen Rang.".to_string(),
                None => "Rang unbekannt - nicht zuordenbar.".to_string(),
            }
        } else {
            "Keine Einträge.".to_string()
        };
        view! {
            <div class="space-y-1">
                <p class="text-xs font-medium text-muted-foreground">{format!("{} ({})", source_title(source), rows.len())}</p>
                {if rows.is_empty() {
                    view! { <p class="text-sm text-muted-foreground">{empty_text}</p> }.into_any()
                } else {
                    view! {
                        <ul class="divide-y divide-border rounded-md border border-border text-sm">
                            {rows.into_iter().map(|d| {
                                let name = d.item_name.clone().unwrap_or_else(|| format!("Item {}", d.item_vnum));
                                let extra = if is_common {
                                    format!("Spieler-Level {}–{}", d.level_from.unwrap_or(0), d.level_to.unwrap_or(0))
                                } else {
                                    let t = d.drop_type.clone().filter(|t| !t.is_empty() && t != "drop").map(|t| format!(", Type {t}")).unwrap_or_default();
                                    format!("„{}“{}", d.group, t)
                                };
                                view! {
                                    <li class="flex items-center gap-3 px-2 py-1.5">
                                        <div class="min-w-0 flex-1">
                                            <div class="truncate">{name} <span class="text-xs text-muted-foreground">{format!(" #{}", d.item_vnum)}</span></div>
                                            <div class="truncate text-xs text-muted-foreground">{extra}</div>
                                        </div>
                                        <span class="shrink-0 text-xs text-muted-foreground">{format!("{}×", d.count.max(1))}</span>
                                        <span class="w-16 shrink-0 text-right font-medium">{format!("≈{}", format_real_drop_chance(d.chance))}</span>
                                    </li>
                                }
                            }).collect_view()}
                        </ul>
                    }.into_any()
                }}
            </div>
        }
    };

    view! {
        <div class="fixed inset-0 z-50 flex items-center justify-center bg-black/50" on:click=move |_| on_close.run(())>
            <div class="max-h-[85vh] w-[36rem] space-y-3 overflow-y-auto rounded-lg border border-border bg-card p-4" on:click=|e| e.stop_propagation()>
                <div class="flex items-start justify-between gap-2">
                    <div>
                        <p class="font-medium">"Was droppt dieser Mob?"</p>
                        <p class="text-sm text-muted-foreground">
                            {move || report.with(|r| match r {
                                Some(Ok(r)) => match &r.mob {
                                    Some(m) => format!(
                                        "{} (#{}){}{}",
                                        m.name,
                                        m.vnum,
                                        m.level.map(|l| format!(", Level {l}")).unwrap_or_default(),
                                        m.rank.as_deref().map(|rk| format!(", {rk} – {}", rank_label(rk))).unwrap_or_default(),
                                    ),
                                    None => format!("Mob #{vnum}"),
                                },
                                _ => format!("Mob #{vnum}"),
                            })}
                        </p>
                    </div>
                    <Button variant=ButtonVariant::Ghost on_click=move |_| on_close.run(())>
                        <Icon name="X" class="size-4"/>
                    </Button>
                </div>
                {move || match report.get() {
                    None => view! {
                        <p class="flex items-center gap-2 text-sm text-muted-foreground">
                            <Icon name="Loader2" class="size-4 animate-spin"/>
                            "Lese die Drop-Dateien vom Server…"
                        </p>
                    }.into_any(),
                    Some(Err(e)) => view! { <p class="text-sm text-destructive">{e}</p> }.into_any(),
                    Some(Ok(r)) => view! {
                        <div class="space-y-3">
                            {section(&r, "mob_drop")}
                            {section(&r, "drop_item_group")}
                            {section(&r, "common_drop")}
                            <p class="text-xs text-muted-foreground">
                                "Jeder Eintrag wird bei einem Kill einzeln gewürfelt. Die Chance gilt bei gleichem Level von Spieler und Mob. Common-Drops gelten nur für Spieler im angegebenen Level-Bereich."
                            </p>
                            {r.warnings.into_iter().map(|w| view! {
                                <p class="flex items-start gap-1 text-xs text-amber-600 dark:text-amber-400">
                                    <Icon name="AlertTriangle" class="mt-0.5 size-3.5 shrink-0"/>
                                    {w}
                                </p>
                            }).collect_view()}
                        </div>
                    }.into_any(),
                }}
            </div>
        </div>
    }
}
