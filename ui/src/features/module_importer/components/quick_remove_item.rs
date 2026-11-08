use crate::components::{Button, ButtonSize, ButtonVariant, Icon};
use crate::features::module_importer::shared::ItemSearchResult;
use crate::lib::log_activity;
use crate::store::navigation::Section;
use crate::tauri::{invoke, invoke_unit};
use leptos::prelude::*;
use leptos::task::spawn_local;
use serde_json::json;

#[component]
pub fn QuickRemoveItem() -> impl IntoView {
    let query = RwSignal::new(String::new());
    let results = RwSignal::new(Vec::<ItemSearchResult>::new());
    let searching = RwSignal::new(false);
    let removing_vnum = RwSignal::new(None::<i64>);
    let confirm_vnum = RwSignal::new(None::<i64>);
    let last_removed = RwSignal::new(None::<i64>);
    let error = RwSignal::new(None::<String>);

    let run = move || {
        let q = query.get_untracked().trim().to_string();
        if q.is_empty() {
            return;
        }
        searching.set(true);
        spawn_local(async move {
            if let Ok(r) = invoke::<Vec<ItemSearchResult>>("search_items", &json!({ "query": q })).await {
                results.set(r);
            }
            searching.set(false);
        });
    };

    let remove = move |vnum: i64| {
        confirm_vnum.set(None);
        removing_vnum.set(Some(vnum));
        error.set(None);
        spawn_local(async move {
            match invoke_unit("remove_single_item", &json!({ "vnum": vnum })).await {
                Ok(()) => {
                    results.update(|r| r.retain(|x| x.vnum != vnum));
                    last_removed.set(Some(vnum));
                    log_activity(
                        Section::ModuleImporter.into(),
                        "remove-item",
                        &format!("Einzelnes importiertes Item entfernt (vnum {vnum})"),
                        Some("item"),
                        Some(&vnum.to_string()),
                    );
                }
                Err(e) => error.set(Some(e)),
            }
            removing_vnum.set(None);
        });
    };

    view! {
        <section class="space-y-2 rounded-lg border border-border p-4">
            <h2 class="text-sm font-medium text-muted-foreground">"Einzelnes Item entfernen"</h2>
            <p class="text-xs text-muted-foreground">
                "Für Items, die nicht (mehr) im Verlauf oben stehen. Löscht Datenbankeintrag, Icon und "
                <code>"item_list.txt"</code>
                "-Eintrag - genau wie „Rückgängig machen\" oben, nur für ein einzelnes Item."
            </p>
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
                                    <Button
                                        size=ButtonSize::Sm
                                        variant=ButtonVariant::Destructive
                                        disabled=Signal::derive(move || removing_vnum.get() == Some(vnum))
                                        on_click=move |_| confirm_vnum.set(Some(vnum))
                                    >
                                        <Icon name="Trash2" class="size-3.5"/>
                                        {move || if removing_vnum.get() == Some(vnum) { "Entferne…" } else { "Entfernen" }}
                                    </Button>
                                </div>
                            }
                        }
                    </For>
                </div>
            </Show>
            {move || last_removed.get().map(|v| view! {
                <p class="flex items-center gap-1 text-sm text-green-600">
                    <Icon name="CheckCircle2" class="size-4"/>
                    " vnum " {v} " vollständig entfernt."
                </p>
            })}
            {move || error.get().map(|e| view! { <p class="whitespace-pre-wrap text-sm text-destructive">{e}</p> })}

            {move || confirm_vnum.get().map(|v| view! {
                <div class="fixed inset-0 flex items-center justify-center bg-black/50">
                    <div class="w-[24rem] space-y-3 rounded-lg border border-border bg-card p-4">
                        <p class="text-sm font-medium">"Item vnum " {v} " endgültig entfernen?"</p>
                        <p class="text-xs text-muted-foreground">
                            "Datenbankeintrag, Icon und " <code>"item_list.txt"</code> "-Eintrag werden entfernt, "
                            <code>"item_proto"</code> " wird neu erzeugt und im Client ersetzt."
                        </p>
                        <div class="flex justify-end gap-2">
                            <Button variant=ButtonVariant::Outline on_click=move |_| confirm_vnum.set(None)>"Abbrechen"</Button>
                            <Button variant=ButtonVariant::Destructive on_click=move |_| remove(v)>"Endgültig entfernen"</Button>
                        </div>
                    </div>
                </div>
            })}
        </section>
    }
}
