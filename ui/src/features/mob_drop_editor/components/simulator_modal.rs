use super::super::types::MobDropGroup;
use super::super::{with_group, IconMap};
use super::shared::{js_number_or, num_sig, Modal, NumInput};
use crate::features::shared::drop_chance::simulate_drops;
use crate::lib::format::de_int;
use leptos::prelude::*;

const PRESETS: [f64; 3] = [100.0, 1000.0, 10000.0];

/// Nutzt dieselbe reale Chance pro Item wie überall sonst im Editor (siehe
/// `shared::drop_chance`) - jeder Kill ist ein unabhängiger Versuch, daher
/// kann das nicht von der "≈X% real"-Spalte abweichen.
#[component]
pub fn SimulatorModal(
    groups: RwSignal<Option<Vec<MobDropGroup>>>,
    group_index: usize,
    icons: RwSignal<IconMap>,
    #[prop(into)] on_close: Callback<()>,
) -> impl IntoView {
    let kills = RwSignal::new(1000.0);

    let results = Memo::new(move |_| {
        let items: Vec<(i64, f64)> =
            with_group(groups, group_index, |g| g.items.iter().map(|i| (i.item_vnum as i64, i.percent)).collect()).unwrap_or_default();
        simulate_drops(&items, kills.get())
    });

    view! {
        <Modal on_close=on_close width_class="w-[28rem]">
            <div class="space-y-3">
                <div>
                    <p class="text-sm font-medium">
                        {move || format!("Drop-Simulation: {}", with_group(groups, group_index, |g| g.name.clone()).unwrap_or_default())}
                    </p>
                    <p class="text-xs text-muted-foreground">
                        "Erwartete Ausbeute über eine gewählte Anzahl Kills, auf Basis der realen Drop-Chance (siehe „i\"-Info oben). Jeder Kill wird als unabhängiger Versuch gerechnet."
                    </p>
                </div>

                <div class="flex items-center gap-2">
                    {PRESETS
                        .iter()
                        .map(|p| {
                            let p = *p;
                            view! {
                                <button
                                    on:click=move |_| kills.set(p)
                                    class=move || {
                                        if kills.get() == p {
                                            "rounded-md border border-border px-2 py-1 text-xs bg-primary text-primary-foreground"
                                        } else {
                                            "rounded-md border border-border px-2 py-1 text-xs hover:bg-muted"
                                        }
                                    }
                                >
                                    {de_int(p as i64)}
                                </button>
                            }
                        })
                        .collect_view()}
                    <NumInput
                        min="0"
                        value=num_sig(move || kills.get())
                        on_input=move |v: String| kills.set(0f64.max(js_number_or(&v, 0.0)))
                        class="w-28 rounded-md border border-border bg-background px-2 py-1 text-xs"
                    />
                    <span class="text-xs text-muted-foreground">"Kills"</span>
                </div>

                <div class="max-h-80 space-y-1 overflow-y-auto">
                    {move || results.get().into_iter().map(|r| {
                        let vnum = r.item_vnum;
                        let chance_dec = if r.chance_at_least_one < 1.0 { 3 } else { 1 };
                        let exp_dec = if r.expected_count < 1.0 { 3 } else { 1 };
                        view! {
                            <div class="flex items-center gap-3 rounded-md border border-border p-2 text-sm">
                                <div class="flex size-8 shrink-0 items-center justify-center rounded-md border border-border bg-muted/40">
                                    {move || match icons.with(|m| m.get(&vnum).cloned().flatten()) {
                                        Some(src) => view! {
                                            <img src=src alt="" class="max-h-full w-6 object-contain [image-rendering:pixelated]"/>
                                        }.into_any(),
                                        None => view! { <span class="text-[10px] text-muted-foreground">{vnum}</span> }.into_any(),
                                    }}
                                </div>
                                <span class="w-16 shrink-0 text-xs text-muted-foreground">"#"{vnum}</span>
                                <div class="flex flex-1 flex-col text-xs">
                                    <span>
                                        "Mind. 1×: "
                                        <strong>{format!("{:.*}%", chance_dec, r.chance_at_least_one)}</strong>
                                    </span>
                                    <span class="text-muted-foreground">
                                        {format!("Erwartete Anzahl: {:.*}", exp_dec, r.expected_count)}
                                    </span>
                                </div>
                            </div>
                        }
                    }).collect_view()}
                    <Show when=move || results.with(|r| r.is_empty())>
                        <p class="p-2 text-sm text-muted-foreground">"Dieser Mob hat noch keine Drops."</p>
                    </Show>
                </div>
            </div>
        </Modal>
    }
}
