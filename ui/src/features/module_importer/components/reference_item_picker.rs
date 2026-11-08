use crate::components::{Button, ButtonSize, ButtonVariant, Icon};
use crate::features::module_importer::shared::{ItemProtoInput, ItemSearchResult};
use crate::tauri::invoke;
use leptos::prelude::*;
use leptos::task::spawn_local;
use serde_json::json;

#[component]
pub fn ReferenceItemPicker(#[prop(into)] on_pick: Callback<ItemProtoInput>) -> impl IntoView {
    let query = RwSignal::new(String::new());
    let results = RwSignal::new(Vec::<ItemSearchResult>::new());
    let searching = RwSignal::new(false);
    let pick_error = RwSignal::new(None::<String>);

    let run = move || {
        let q = query.get_untracked().trim().to_string();
        if q.is_empty() {
            return;
        }
        searching.set(true);
        spawn_local(async move {
            // Wie im TS ohne eigene Fehleranzeige (try/finally ohne catch).
            if let Ok(r) = invoke::<Vec<ItemSearchResult>>("search_items", &json!({ "query": q })).await {
                results.set(r);
            }
            searching.set(false);
        });
    };

    // "Übernehmen" darf nie still scheitern - ein null-Ergebnis oder Fehler
    // muss sofort sichtbar sein.
    let pick = move |vnum: i64| {
        pick_error.set(None);
        spawn_local(async move {
            match invoke::<Option<ItemProtoInput>>("get_item_proto", &json!({ "vnum": vnum })).await {
                Ok(Some(full)) => {
                    on_pick.run(full);
                    results.set(vec![]);
                    query.set(String::new());
                }
                Ok(None) => pick_error.set(Some(format!("Item {vnum} konnte nicht geladen werden (nicht gefunden)."))),
                Err(e) => pick_error.set(Some(e)),
            }
        });
    };

    view! {
        <div class="space-y-2">
            <div class="flex gap-2">
                <input
                    prop:value=move || query.get()
                    on:input=move |e| query.set(event_target_value(&e))
                    on:keydown=move |e| {
                        if e.key() == "Enter" {
                            run()
                        }
                    }
                    placeholder="Item nach Name oder VNUM suchen…"
                    class="flex-1 rounded-md border border-border bg-background px-2 py-1 text-sm"
                />
                <Button variant=ButtonVariant::Outline on_click=move |_| run() disabled=searching>
                    <Icon name="Search" class="size-4"/>
                </Button>
            </div>
            <Show when=move || results.with(|r| !r.is_empty())>
                <div class="max-h-40 space-y-1 overflow-y-auto">
                    <For each=move || results.get() key=|r| r.vnum let:r>
                        {
                            let vnum = r.vnum;
                            view! {
                                <div class="flex items-center justify-between rounded-md px-2 py-1 text-sm hover:bg-muted">
                                    <span>{r.name.clone()} " " <span class="text-muted-foreground">"#" {vnum}</span></span>
                                    <Button size=ButtonSize::Sm variant=ButtonVariant::Outline on_click=move |_| pick(vnum)>
                                        "Übernehmen"
                                    </Button>
                                </div>
                            }
                        }
                    </For>
                </div>
            </Show>
            {move || pick_error.get().map(|e| view! { <p class="text-sm text-destructive">{e}</p> })}
        </div>
    }
}
