use super::super::types::MobDropGroup;
use super::shared::{fmt_num, Modal};
use crate::features::shared::drop_chance::format_real_drop_chance;
use crate::features::shared::{EntityBrowser, EntityKind, EntityRow};
use leptos::prelude::*;

/// "Wer droppt Item X" - rein clientseitig, da `groups` beim Öffnen bereits
/// die komplette Datei enthält (eine einzelne Datei, kein Mob-für-Mob-
/// Nachladen nötig).
#[component]
pub fn ReverseLookupModal(
    groups: RwSignal<Option<Vec<MobDropGroup>>>,
    #[prop(into)] on_select_mob: Callback<usize>,
    #[prop(into)] on_close: Callback<()>,
) -> impl IntoView {
    let selected_item = RwSignal::new(None::<EntityRow>);

    // (Index, Name, Mob-VNUM, Anzahl, Prozent)
    let matches = Memo::new(move |_| {
        let Some(item) = selected_item.get() else { return vec![] };
        groups.with(|g| {
            g.as_deref()
                .unwrap_or_default()
                .iter()
                .enumerate()
                .filter_map(|(index, g)| {
                    g.items
                        .iter()
                        .find(|it| it.item_vnum == item.vnum as f64)
                        .map(|drop| (index, g.name.clone(), g.mob_vnum, drop.count, drop.percent))
                })
                .collect::<Vec<_>>()
        })
    });

    view! {
        <Modal on_close=on_close>
            <div class="space-y-2">
                <p class="text-sm font-medium">"Wer droppt dieses Item?"</p>

                <Show when=move || selected_item.with(|s| s.is_none())>
                    <EntityBrowser
                        kind=EntityKind::Item
                        pick_label="Auswählen"
                        auto_focus=true
                        max_height_class="max-h-64"
                        on_pick=move |r: EntityRow| selected_item.set(Some(r))
                    />
                </Show>

                {move || selected_item.get().map(|item| view! {
                    <div class="space-y-2">
                        <div class="flex items-center justify-between">
                            <p class="text-sm">
                                <strong>{item.name.clone()}</strong>
                                " "
                                <span class="text-muted-foreground">"#"{item.vnum}</span>
                                " wird von "
                                {move || matches.with(|m| m.len())}
                                " Mob(s) gedroppt:"
                            </p>
                            <button
                                class="text-xs text-muted-foreground underline hover:text-foreground"
                                on:click=move |_| selected_item.set(None)
                            >
                                "Anderes Item"
                            </button>
                        </div>
                        <div class="max-h-72 space-y-1 overflow-y-auto">
                            {move || matches.get().into_iter().map(|(index, name, mob_vnum, count, percent)| view! {
                                <button
                                    on:click=move |_| on_select_mob.run(index)
                                    class="flex w-full items-center justify-between rounded-md border border-border px-2 py-1 text-left text-sm hover:bg-muted"
                                >
                                    <span>
                                        {name}
                                        " "
                                        <span class="text-muted-foreground">{format!("(Mob #{})", fmt_num(mob_vnum))}</span>
                                    </span>
                                    <span class="text-xs text-muted-foreground">
                                        {format!(
                                            "Anzahl {} · {}% (≈{} real)",
                                            fmt_num(count),
                                            fmt_num(percent),
                                            format_real_drop_chance(percent),
                                        )}
                                    </span>
                                </button>
                            }).collect_view()}
                            <Show when=move || matches.with(|m| m.is_empty())>
                                <p class="p-2 text-sm text-muted-foreground">"Kein geladener Mob droppt dieses Item."</p>
                            </Show>
                        </div>
                    </div>
                })}
            </div>
        </Modal>
    }
}
