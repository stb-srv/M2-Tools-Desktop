//! Wirtschafts-Dashboard (Port von `src/features/economy-dashboard/EconomyDashboard.tsx`).

use crate::components::{Button, ButtonVariant, Icon, PageHeader, Skeleton};
use crate::util::format::de_int;
use crate::store::navigation::Section;
use crate::tauri::{invoke, invoke0};
use leptos::prelude::*;
use leptos::task::spawn_local;
use serde::Deserialize;
use serde_json::json;

#[derive(Clone, Debug, PartialEq, Deserialize)]
struct EconomyStats {
    player_count: i64,
    total_gold: i64,
    shop_item_count: i64,
    avg_shop_price: Option<f64>,
    min_shop_price: Option<f64>,
    max_shop_price: Option<f64>,
}

#[derive(Clone, Debug, PartialEq, Deserialize)]
struct TopGoldHolder {
    player_id: i64,
    name: Option<String>,
    gold: i64,
}

const TOP_HOLDERS_LIMIT: i64 = 20;

/// Wie `Math.round(value).toLocaleString("de-DE")`, `None` = "–".
pub fn format_gold(value: Option<f64>) -> String {
    match value {
        None => "–".to_string(),
        // Math.round rundet .5 immer Richtung +∞.
        Some(v) => de_int((v + 0.5).floor() as i64),
    }
}

#[component]
pub fn EconomyDashboard() -> impl IntoView {
    let stats = RwSignal::new(None::<EconomyStats>);
    let holders = RwSignal::new(None::<Vec<TopGoldHolder>>);
    let loading = RwSignal::new(false);
    let error = RwSignal::new(None::<String>);

    let load = move || {
        loading.set(true);
        error.set(None);
        spawn_local(async move {
            let res: Result<(EconomyStats, Vec<TopGoldHolder>), String> = async {
                let s = invoke0::<EconomyStats>("get_economy_stats").await?;
                let h = invoke::<Vec<TopGoldHolder>>("get_top_gold_holders", &json!({ "limit": TOP_HOLDERS_LIMIT })).await?;
                Ok((s, h))
            }
            .await;
            match res {
                Ok((s, h)) => {
                    stats.try_set(Some(s));
                    holders.try_set(Some(h));
                }
                Err(e) => {
                    error.try_set(Some(e));
                }
            }
            loading.try_set(false);
        });
    };
    load();

    let tile = |label: &'static str, value: String| {
        view! {
            <div class="rounded-md border border-border p-3">
                <p class="text-xs text-muted-foreground">{label}</p>
                <p class="text-lg font-semibold">{value}</p>
            </div>
        }
    };

    view! {
        <div class="space-y-4">
            <PageHeader module=Section::EconomyDashboard title="Wirtschafts-Dashboard".to_string()/>
            <p class="max-w-4xl text-sm text-muted-foreground">
                "Momentaufnahme der Server-Wirtschaft: Goldmenge im Umlauf (Summe über "
                <code>"player.player.gold"</code>
                "), Preisspanne über alle tatsächlich in Shops gelisteten Items ("
                <code>"item_proto.gold"</code>
                ", der NPC-Verkaufspreis), und die größten Gold-Bestände. Reine Lesewerte, keine Änderungen."
            </p>

            <div class="flex items-center gap-2">
                <Button variant=ButtonVariant::Outline on_click=move |_| load() disabled=loading>
                    <Icon name="RefreshCw" class=Signal::derive(move || if loading.get() { "size-4 animate-spin".to_string() } else { "size-4 ".to_string() })/>
                    "Neu laden"
                </Button>
            </div>

            {move || error.get().map(|e| view! { <p class="whitespace-pre-wrap text-sm text-destructive">{e}</p> })}

            <Show when=move || stats.with(|s| s.is_none()) && error.with(|e| e.is_none())>
                <div class="grid grid-cols-2 gap-3 sm:grid-cols-3">
                    {(0..6).map(|_| view! {
                        <div class="space-y-1.5 rounded-md border border-border p-3">
                            <Skeleton class="h-3 w-20"/>
                            <Skeleton class="h-5 w-16"/>
                        </div>
                    }).collect_view()}
                </div>
            </Show>

            {move || stats.get().map(|s| view! {
                <div class="grid grid-cols-2 gap-3 sm:grid-cols-3">
                    {tile("Spieler", de_int(s.player_count))}
                    {tile("Gold im Umlauf", format_gold(Some(s.total_gold as f64)))}
                    {tile("Gelistete Shop-Items", de_int(s.shop_item_count))}
                    {tile("Ø Shop-Preis", format_gold(s.avg_shop_price))}
                    {tile("Min. Shop-Preis", format_gold(s.min_shop_price))}
                    {tile("Max. Shop-Preis", format_gold(s.max_shop_price))}
                </div>
            })}

            <div>
                <h2 class="mb-2 text-sm font-semibold text-muted-foreground">
                    {format!("Top {TOP_HOLDERS_LIMIT} Gold-Bestände")}
                </h2>
                <div class="space-y-1 rounded-md border border-border p-1">
                    {move || holders.get().unwrap_or_default().into_iter().enumerate().map(|(i, h)| view! {
                        <div class="flex items-center gap-2 rounded-md px-2 py-1.5 text-sm hover:bg-muted">
                            <span class="w-6 shrink-0 text-right text-xs text-muted-foreground">{format!("{}.", i + 1)}</span>
                            <span class="flex-1 truncate">{h.name.clone().unwrap_or_else(|| format!("Spieler #{}", h.player_id))}</span>
                            <span class="shrink-0 font-mono text-xs">{format_gold(Some(h.gold as f64))}</span>
                        </div>
                    }).collect_view()}
                    <Show when=move || holders.with(|h| h.as_ref().is_some_and(|h| h.is_empty()))>
                        <p class="p-2 text-sm text-muted-foreground">"Keine Spieler gefunden."</p>
                    </Show>
                    <Show when=move || holders.with(|h| h.is_none()) && error.with(|e| e.is_none())>
                        <div class="space-y-1 p-1">
                            <Skeleton class="h-8 w-full"/>
                            <Skeleton class="h-8 w-full"/>
                            <Skeleton class="h-8 w-full"/>
                        </div>
                    </Show>
                </div>
            </div>
        </div>
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn formats_gold_like_de_locale() {
        assert_eq!(format_gold(None), "–");
        assert_eq!(format_gold(Some(1234567.0)), "1.234.567");
        assert_eq!(format_gold(Some(1499.5)), "1.500");
        assert_eq!(format_gold(Some(-2.5)), "-2");
    }
}
