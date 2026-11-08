use super::super::types::MobDropGroup;
use super::super::with_group;
use crate::components::{Button, ButtonSize, ButtonVariant, Icon};
use leptos::prelude::*;

#[component]
pub fn GroupSidebar(
    groups: RwSignal<Option<Vec<MobDropGroup>>>,
    /// Indizes der Gruppen, die zur Suche passen.
    #[prop(into)] filtered_groups: Signal<Vec<usize>>,
    group_search: RwSignal<String>,
    selected_index: RwSignal<Option<usize>>,
    #[prop(into)] on_create_click: Callback<()>,
) -> impl IntoView {
    view! {
        <div class="flex w-72 shrink-0 flex-col gap-2">
            <div class="flex gap-2">
                <div class="relative flex-1">
                    <Icon name="Search" class="absolute left-2 top-1/2 size-3.5 -translate-y-1/2 text-muted-foreground"/>
                    <input
                        prop:value=move || group_search.get()
                        on:input=move |e| group_search.set(event_target_value(&e))
                        placeholder="Mob suchen (Name/VNUM)…"
                        class="w-full rounded-md border border-border bg-background py-1 pl-7 pr-2 text-sm"
                    />
                </div>
                <Button variant=ButtonVariant::Outline size=ButtonSize::Icon on_click=move |_| on_create_click.run(())>
                    <Icon name="Plus" class="size-4"/>
                </Button>
            </div>
            <div class="flex-1 space-y-1 overflow-y-auto">
                <For each=move || filtered_groups.get() key=|i| *i let:index>
                    <div
                        on:click=move |_| selected_index.set(Some(index))
                        class=move || {
                            if selected_index.get() == Some(index) {
                                "cursor-pointer rounded-md border border-border p-2 text-sm hover:bg-muted bg-muted"
                            } else {
                                "cursor-pointer rounded-md border border-border p-2 text-sm hover:bg-muted "
                            }
                        }
                    >
                        <div class="font-medium">{move || with_group(groups, index, |g| g.name.clone()).unwrap_or_default()}</div>
                        <div class="text-xs text-muted-foreground">
                            {move || with_group(groups, index, |g| format!("Mob: {} · {} Drops", super::shared::fmt_num(g.mob_vnum), g.items.len()))
                                .unwrap_or_default()}
                        </div>
                    </div>
                </For>
                <Show when=move || filtered_groups.with(|f| f.is_empty())>
                    <p class="p-2 text-sm text-muted-foreground">"Keine Mobs in der Datei."</p>
                </Show>
            </div>
        </div>
    }
}
