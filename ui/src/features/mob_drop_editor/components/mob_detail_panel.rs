use super::super::types::{MobDropGroup, MobDropItem};
use super::super::{with_group, IconMap};
use super::shared::{clamp_percent, fmt_num, js_number_or, num_sig, Field, NumInput, MAX_PERCENT_ATTR};
use crate::components::{Button, ButtonSize, ButtonVariant, Icon};
use crate::features::shared::drop_chance::format_real_drop_chance;
use leptos::prelude::*;

#[component]
pub fn MobDetailPanel(
    groups: RwSignal<Option<Vec<MobDropGroup>>>,
    selected_index: usize,
    icons: RwSignal<IconMap>,
    dirty: RwSignal<bool>,
    #[prop(into)] on_remove_item: Callback<(usize, usize)>,
    #[prop(into)] on_delete_click: Callback<()>,
    #[prop(into)] on_add_item_click: Callback<()>,
    #[prop(into)] on_simulate_click: Callback<()>,
) -> impl IntoView {
    let gi = selected_index;
    let update_group = move |f: Box<dyn FnOnce(&mut MobDropGroup)>| {
        groups.update(|g| {
            if let Some(g) = g.as_mut().and_then(|g| g.get_mut(gi)) {
                f(g)
            }
        });
        dirty.set(true);
    };
    let update_item = move |ii: usize, f: Box<dyn FnOnce(&mut MobDropItem)>| {
        groups.update(|g| {
            if let Some(it) = g.as_mut().and_then(|g| g.get_mut(gi)).and_then(|g| g.items.get_mut(ii)) {
                f(it)
            }
        });
        dirty.set(true);
    };
    let item_count = move || with_group(groups, gi, |g| g.items.len()).unwrap_or(0);
    let item_field = move |ii: usize, f: fn(&MobDropItem) -> f64| {
        groups.with(|g| g.as_ref().and_then(|g| g.get(gi)).and_then(|g| g.items.get(ii)).map(f)).unwrap_or(0.0)
    };

    view! {
        <div class="flex items-start justify-between rounded-lg border border-border p-3">
            <div class="flex flex-wrap items-end gap-3">
                <Field label="Gruppenname">
                    <input
                        prop:value=move || with_group(groups, gi, |g| g.name.clone()).unwrap_or_default()
                        on:input=move |e| {
                            let v = event_target_value(&e);
                            update_group(Box::new(move |g| g.name = v));
                        }
                        class="w-48 rounded-md border border-border bg-background px-2 py-1 text-sm"
                    />
                </Field>
                <Field label="Mob-VNUM">
                    <NumInput
                        value=num_sig(move || with_group(groups, gi, |g| g.mob_vnum).unwrap_or(0.0))
                        on_input=move |v: String| update_group(Box::new(move |g| g.mob_vnum = js_number_or(&v, 0.0)))
                        class="w-24 rounded-md border border-border bg-background px-2 py-1 text-sm"
                    />
                </Field>
                <Field label="Type">
                    <input
                        prop:value=move || with_group(groups, gi, |g| g.drop_type.clone()).unwrap_or_default()
                        on:input=move |e| {
                            let v = event_target_value(&e);
                            update_group(Box::new(move |g| g.drop_type = v));
                        }
                        class="w-24 rounded-md border border-border bg-background px-2 py-1 text-sm"
                    />
                </Field>
            </div>
            <Button variant=ButtonVariant::Destructive size=ButtonSize::Sm on_click=move |_| on_delete_click.run(())>
                <Icon name="Trash2" class="size-4"/>
                "Mob löschen"
            </Button>
        </div>

        <div class="space-y-2 rounded-lg border border-border p-3">
            <div class="flex items-center justify-between">
                <h2 class="text-sm font-medium text-muted-foreground">{move || format!("Drops ({})", item_count())}</h2>
                <div class="flex gap-2">
                    <Button
                        size=ButtonSize::Sm
                        variant=ButtonVariant::Outline
                        on_click=move |_| on_simulate_click.run(())
                        disabled=Signal::derive(move || item_count() == 0)
                        title="Erwartete Ausbeute über X Kills durchrechnen"
                    >
                        <Icon name="Dices" class="size-3.5"/>
                        "Simulieren"
                    </Button>
                    <Button size=ButtonSize::Sm variant=ButtonVariant::Outline on_click=move |_| on_add_item_click.run(())>
                        <Icon name="Plus" class="size-3.5"/>
                        "Item hinzufügen"
                    </Button>
                </div>
            </div>
            <div class="space-y-1">
                <For each=move || 0..item_count() key=|i| *i let:ii>
                    {
                        let vnum = move || item_field(ii, |i| i.item_vnum);
                        view! {
                            <div class="flex items-center gap-3 rounded-md border border-border p-2">
                                <div class="flex size-9 shrink-0 items-center justify-center rounded-md border border-border bg-muted/40">
                                    {move || {
                                        let v = vnum();
                                        match icons.with(|m| m.get(&(v as i64)).cloned().flatten()) {
                                            Some(src) => view! {
                                                <img src=src alt="" class="max-h-full w-7 object-contain [image-rendering:pixelated]"/>
                                            }.into_any(),
                                            None => view! { <span class="text-[10px] text-muted-foreground">{fmt_num(v)}</span> }.into_any(),
                                        }
                                    }}
                                </div>
                                <span class="w-20 shrink-0 text-xs text-muted-foreground">{move || format!("#{}", fmt_num(vnum()))}</span>
                                <Field label="Anzahl">
                                    <NumInput
                                        min="1"
                                        value=num_sig(move || item_field(ii, |i| i.count))
                                        on_input=move |v: String| update_item(ii, Box::new(move |it| it.count = 1f64.max(js_number_or(&v, 1.0))))
                                        class="w-16 rounded-md border border-border bg-background px-2 py-1 text-sm"
                                    />
                                </Field>
                                <Field label="Prozent">
                                    <NumInput
                                        min="0"
                                        max=MAX_PERCENT_ATTR
                                        step="0.01"
                                        value=num_sig(move || item_field(ii, |i| i.percent))
                                        on_input=move |v: String| update_item(ii, Box::new(move |it| it.percent = clamp_percent(js_number_or(&v, 0.0))))
                                        class="w-20 rounded-md border border-border bg-background px-2 py-1 text-sm"
                                    />
                                </Field>
                                <span class="text-xs text-muted-foreground" title="Reale Drop-Chance auf diesem Server (siehe i-Info oben)">
                                    {move || format!("≈{}", format_real_drop_chance(item_field(ii, |i| i.percent)))}
                                </span>
                                <Button
                                    variant=ButtonVariant::Ghost
                                    size=ButtonSize::IconSm
                                    class="ml-auto"
                                    on_click=move |_| on_remove_item.run((gi, ii))
                                >
                                    <Icon name="X" class="size-4"/>
                                </Button>
                            </div>
                        }
                    }
                </For>
                <Show when=move || item_count() == 0>
                    <p class="text-sm text-muted-foreground">"Noch keine Drops."</p>
                </Show>
            </div>
        </div>
    }
}
