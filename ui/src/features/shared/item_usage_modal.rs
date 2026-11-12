//! "Wo wird das benutzt?" - fasst Shops, Mob-Drops (mob_drop_item.txt/
//! common_drop_item.txt), etc_drop_item.txt, drop_item_group.txt,
//! special_item_group.txt (Kisten), cube.txt und eine Quest-Volltextsuche zu
//! einem Bericht zusammen (`find_item_usages`, siehe
//! src-tauri/src/commands/item_usage.rs). Jede Quelle, die nicht geprüft
//! werden konnte, landet unten als Warnung statt den ganzen Bericht
//! scheitern zu lassen.

use crate::components::{Button, ButtonVariant, Icon};
use crate::features::shared::drop_chance::format_real_drop_chance;
use crate::tauri::invoke;
use leptos::prelude::*;
use leptos::task::spawn_local;
use serde::Deserialize;
use serde_json::json;

#[derive(Clone, Debug, Deserialize)]
struct ShopUsage {
    shop_vnum: i64,
    shop_name: String,
    count: i64,
}
#[derive(Clone, Debug, Deserialize)]
struct MobDropUsage {
    source: String,
    mob_vnum: i64,
    group_name: String,
    percent: f64,
}
#[derive(Clone, Debug, Deserialize)]
struct CommonDropUsage {
    rank: String,
    level_from: i64,
    level_to: i64,
    chance: f64,
}
#[derive(Clone, Debug, Deserialize)]
struct DropGroupUsage {
    mob_vnum: i64,
    group_name: String,
    percent: f64,
}
#[derive(Clone, Debug, Deserialize)]
struct BoxRewardUsage {
    group_name: String,
    box_vnum: i64,
}
#[derive(Clone, Debug, Deserialize)]
struct CubeUsage {
    npc_vnums: Vec<i64>,
    percent: f64,
}
#[derive(Clone, Debug, Deserialize)]
struct EtcDropUsage {
    percent: f64,
}
#[derive(Clone, Debug, Deserialize)]
struct QuestSearchLine {
    line_number: i64,
    text: String,
}
#[derive(Clone, Debug, Deserialize)]
struct QuestSearchMatch {
    relative_path: String,
    #[allow(dead_code)]
    category: String,
    #[allow(dead_code)]
    name: String,
    lines: Vec<QuestSearchLine>,
}

#[derive(Clone, Debug, Deserialize)]
struct ItemUsageReport {
    shops: Vec<ShopUsage>,
    mob_drops: Vec<MobDropUsage>,
    #[serde(default)]
    common_drops: Vec<CommonDropUsage>,
    etc_drop: Option<EtcDropUsage>,
    drop_item_groups: Vec<DropGroupUsage>,
    is_box_of: Option<String>,
    possible_reward_in: Vec<BoxRewardUsage>,
    cube_ingredient_in: Vec<CubeUsage>,
    cube_reward_in: Vec<CubeUsage>,
    quests: Vec<QuestSearchMatch>,
    warnings: Vec<String>,
}

impl ItemUsageReport {
    fn nothing_found(&self) -> bool {
        self.shops.is_empty()
            && self.mob_drops.is_empty()
            && self.common_drops.is_empty()
            && self.etc_drop.is_none()
            && self.drop_item_groups.is_empty()
            && self.is_box_of.is_none()
            && self.possible_reward_in.is_empty()
            && self.cube_ingredient_in.is_empty()
            && self.cube_reward_in.is_empty()
            && self.quests.is_empty()
    }
}

/// Wie `String(n)` in JS: ganze Zahlen ohne ".0".
fn js_num(n: f64) -> String {
    if n.fract() == 0.0 && n.is_finite() && n.abs() < 1e15 {
        format!("{}", n as i64)
    } else {
        format!("{n}")
    }
}

fn join_vnums(v: &[i64]) -> String {
    v.iter().map(|n| n.to_string()).collect::<Vec<_>>().join(", ")
}

fn usage_section(title: String, children: AnyView) -> AnyView {
    view! {
        <div class="space-y-1">
            <p class="text-xs font-medium text-muted-foreground">{title}</p>
            {children}
        </div>
    }
    .into_any()
}

fn cube_list(items: &[CubeUsage]) -> AnyView {
    view! {
        <ul class="space-y-0.5">
            {items.iter().map(|c| view! {
                <li>"NPC " {join_vnums(&c.npc_vnums)} " - " {js_num(c.percent)} "% Erfolgschance"</li>
            }).collect_view()}
        </ul>
    }
    .into_any()
}

fn report_view(vnum: i64, report: ItemUsageReport) -> AnyView {
    let mut parts: Vec<AnyView> = vec![];
    if report.nothing_found() {
        parts.push(
            view! { <p class="text-sm text-muted-foreground">"Keine Verwendung in den geprüften Quellen gefunden."</p> }
                .into_any(),
        );
    }
    if !report.shops.is_empty() {
        parts.push(usage_section(
            format!("Shops ({})", report.shops.len()),
            view! {
                <ul class="space-y-0.5">
                    {report.shops.iter().map(|s| view! {
                        <li>{s.shop_name.clone()} " (#" {s.shop_vnum} ") - " {s.count} "×"</li>
                    }).collect_view()}
                </ul>
            }
            .into_any(),
        ));
    }
    if !report.mob_drops.is_empty() {
        parts.push(usage_section(
            format!("Mob-Drops ({})", report.mob_drops.len()),
            view! {
                <ul class="space-y-0.5">
                    {report.mob_drops.iter().map(|d| view! {
                        <li>
                            "Mob #" {d.mob_vnum} " „" {d.group_name.clone()} "\" (" {d.source.clone()} ") - ≈"
                            {format_real_drop_chance(d.percent)} " real"
                        </li>
                    }).collect_view()}
                </ul>
            }
            .into_any(),
        ));
    }
    if !report.common_drops.is_empty() {
        parts.push(usage_section(
            format!("common_drop_item.txt ({})", report.common_drops.len()),
            view! {
                <ul class="space-y-0.5">
                    {report.common_drops.iter().map(|d| view! {
                        <li>
                            "Alle " {d.rank.clone()} "-Mobs, Spieler-Level " {d.level_from} "–" {d.level_to} " - ≈"
                            {format_real_drop_chance(d.chance)} " real"
                        </li>
                    }).collect_view()}
                </ul>
            }
            .into_any(),
        ));
    }
    if let Some(etc) = &report.etc_drop {
        parts.push(usage_section(
            "etc_drop_item.txt".into(),
            view! { <p>"≈" {format_real_drop_chance(etc.percent)} " real"</p> }.into_any(),
        ));
    }
    if !report.drop_item_groups.is_empty() {
        parts.push(usage_section(
            format!("drop_item_group.txt ({})", report.drop_item_groups.len()),
            view! {
                <ul class="space-y-0.5">
                    {report.drop_item_groups.iter().map(|d| view! {
                        <li>
                            "Mob #" {d.mob_vnum} " „" {d.group_name.clone()} "\" - ≈" {format_real_drop_chance(d.percent)}
                            " real"
                        </li>
                    }).collect_view()}
                </ul>
            }
            .into_any(),
        ));
    }
    if let Some(group) = &report.is_box_of {
        parts.push(usage_section(
            "Kiste (special_item_group.txt)".into(),
            view! { <p>"Ist die Kiste der Gruppe „" {group.clone()} "\""</p> }.into_any(),
        ));
    }
    if !report.possible_reward_in.is_empty() {
        parts.push(usage_section(
            format!("Möglicher Kisten-Inhalt ({})", report.possible_reward_in.len()),
            view! {
                <ul class="space-y-0.5">
                    {report.possible_reward_in.iter().map(|b| view! {
                        <li>"Gruppe „" {b.group_name.clone()} "\" (Kiste #" {b.box_vnum} ")"</li>
                    }).collect_view()}
                </ul>
            }
            .into_any(),
        ));
    }
    if !report.cube_ingredient_in.is_empty() {
        parts.push(usage_section(
            format!("Cube - als Zutat ({})", report.cube_ingredient_in.len()),
            cube_list(&report.cube_ingredient_in),
        ));
    }
    if !report.cube_reward_in.is_empty() {
        parts.push(usage_section(
            format!("Cube - als Belohnung ({})", report.cube_reward_in.len()),
            cube_list(&report.cube_reward_in),
        ));
    }
    if !report.quests.is_empty() {
        parts.push(usage_section(
            format!("Quests ({}, Volltextsuche nach \"{}\")", report.quests.len(), vnum),
            view! {
                <ul class="space-y-1">
                    {report.quests.iter().map(|q| view! {
                        <li>
                            <p class="font-mono text-xs">{q.relative_path.clone()}</p>
                            {q.lines.iter().take(2).map(|l| view! {
                                <p class="truncate pl-2 text-xs text-muted-foreground">
                                    "Zeile " {l.line_number} ": " {l.text.trim().to_string()}
                                </p>
                            }).collect_view()}
                        </li>
                    }).collect_view()}
                </ul>
                <p class="text-xs text-muted-foreground">
                    "Reine Teilstring-Suche - kann auch Treffer enthalten, bei denen die Zahl in einem anderen Zusammenhang vorkommt (z.B. eine andere vnum, ein Betrag)."
                </p>
            }
            .into_any(),
        ));
    }
    if !report.warnings.is_empty() {
        parts.push(
            view! {
                <p class="flex items-start gap-2 rounded-md border border-amber-500/40 bg-amber-500/10 p-2 text-xs text-amber-700 dark:text-amber-400">
                    <Icon name="AlertTriangle" class="mt-0.5 size-3.5 shrink-0"/>
                    <span>{report.warnings.join(" · ")}</span>
                </p>
            }
            .into_any(),
        );
    }
    view! { <div class="min-h-0 flex-1 space-y-3 overflow-y-auto pr-1 text-sm">{parts}</div> }.into_any()
}

#[component]
pub fn ItemUsageModal(vnum: i64, #[prop(into)] on_close: Callback<()>) -> impl IntoView {
    let report = RwSignal::new(None::<ItemUsageReport>);
    let loading = RwSignal::new(false);
    let error = RwSignal::new(None::<String>);

    loading.set(true);
    error.set(None);
    spawn_local(async move {
        match invoke::<ItemUsageReport>("find_item_usages", &json!({ "vnum": vnum })).await {
            Ok(r) => report.set(Some(r)),
            Err(e) => error.set(Some(e)),
        }
        loading.set(false);
    });

    view! {
        <div class="fixed inset-0 z-50 flex items-center justify-center bg-black/50">
            <div class="flex max-h-[80vh] w-[36rem] flex-col gap-3 rounded-lg border border-border bg-card p-4">
                <div class="flex items-center justify-between">
                    <p class="text-sm font-medium">"Wo wird Item #" {vnum} " benutzt?"</p>
                    <button on:click=move |_| on_close.run(()) class="text-muted-foreground hover:text-foreground">
                        <Icon name="X" class="size-4"/>
                    </button>
                </div>

                <Show when=move || loading.get()>
                    <p class="text-sm text-muted-foreground">"Prüfe…"</p>
                </Show>
                {move || error.get().map(|e| view! { <p class="text-sm text-destructive">{e}</p> })}

                {move || report.get().map(|r| report_view(vnum, r))}

                <div class="flex justify-end">
                    <Button variant=ButtonVariant::Outline on_click=move |_| on_close.run(())>"Schließen"</Button>
                </div>
            </div>
        </div>
    }
}
