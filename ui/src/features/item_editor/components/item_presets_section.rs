//! Benannte, wiederverwendbare Feldwert-Vorlagen ("Standard-Ohrring", ...) -
//! unabhängig von einer echten item_proto-Zeile. Siehe
//! src-tauri/src/item_presets.rs.

use crate::components::{Button, ButtonSize, ButtonVariant, Icon};
use crate::features::item_editor::types::ItemProtoInput;
use crate::tauri::{invoke0, invoke_unit};
use leptos::prelude::*;
use leptos::task::spawn_local;
use serde::Deserialize;
use serde_json::json;

#[derive(Clone, Debug, PartialEq, Deserialize)]
struct ItemPreset {
    id: i64,
    name: String,
    item: ItemProtoInput,
}

#[component]
pub fn ItemPresetsSection(
    #[prop(into)] current_item: Signal<ItemProtoInput>,
    #[prop(into)] on_load: Callback<ItemProtoInput>,
) -> impl IntoView {
    let presets = RwSignal::new(Vec::<ItemPreset>::new());
    let loading = RwSignal::new(false);
    let save_name = RwSignal::new(String::new());
    let saving = RwSignal::new(false);
    let error = RwSignal::new(None::<String>);

    async fn refresh(presets: RwSignal<Vec<ItemPreset>>, loading: RwSignal<bool>, error: RwSignal<Option<String>>) {
        loading.set(true);
        error.set(None);
        match invoke0::<Vec<ItemPreset>>("list_item_presets").await {
            Ok(p) => presets.set(p),
            Err(e) => error.set(Some(e)),
        }
        loading.set(false);
    }

    spawn_local(refresh(presets, loading, error));

    let save = move || {
        let name = save_name.get_untracked().trim().to_string();
        if name.is_empty() {
            return;
        }
        let item = current_item.get_untracked();
        saving.set(true);
        error.set(None);
        spawn_local(async move {
            match invoke_unit("save_item_preset", &json!({ "name": name, "item": item })).await {
                Ok(()) => {
                    save_name.set(String::new());
                    refresh(presets, loading, error).await;
                }
                Err(e) => error.set(Some(e)),
            }
            saving.set(false);
        });
    };

    let remove = move |id: i64| {
        error.set(None);
        spawn_local(async move {
            match invoke_unit("delete_item_preset", &json!({ "id": id })).await {
                Ok(()) => refresh(presets, loading, error).await,
                Err(e) => error.set(Some(e)),
            }
        });
    };

    view! {
        <section class="space-y-2 rounded-lg border border-border p-4">
            <h2 class="text-sm font-medium text-muted-foreground">"Vorlagen (optional)"</h2>
            <p class="text-xs text-muted-foreground">
                "Eigene wiederverwendbare Vorlagen (z.B. \"Standard-Ohrring\") — unabhängig von einem bestehenden Item, für schnelles Anlegen mehrerer ähnlicher Custom-Items. Das Icon ist nie Teil einer Vorlage und muss immer neu gewählt werden."
            </p>

            <div class="flex items-center gap-2">
                <input
                    class="h-8 flex-1 rounded-md border border-border bg-background px-2 text-sm"
                    placeholder="Name der Vorlage"
                    prop:value=move || save_name.get()
                    on:input=move |e| save_name.set(event_target_value(&e))
                />
                <Button
                    size=ButtonSize::Sm
                    variant=ButtonVariant::Outline
                    on_click=move |_| save()
                    disabled=Signal::derive(move || saving.get() || save_name.with(|n| n.trim().is_empty()))
                >
                    <Icon name="Save" class="size-3.5"/>
                    "Als Vorlage speichern"
                </Button>
            </div>

            {move || {
                if loading.get() {
                    view! { <p class="text-xs text-muted-foreground">"Lade…"</p> }.into_any()
                } else if presets.with(|p| p.is_empty()) {
                    view! { <p class="text-xs text-muted-foreground">"Noch keine Vorlagen gespeichert."</p> }.into_any()
                } else {
                    view! {
                        <div class="space-y-1">
                            {presets.get().into_iter().map(|p| {
                                let id = p.id;
                                let item = p.item.clone();
                                view! {
                                    <div class="flex items-center gap-2 rounded-md px-2 py-1 text-sm hover:bg-muted">
                                        <span class="flex-1 truncate">{p.name.clone()}</span>
                                        <Button size=ButtonSize::Sm variant=ButtonVariant::Outline on_click=move |_| on_load.run(item.clone())>
                                            "Übernehmen"
                                        </Button>
                                        <Button size=ButtonSize::Sm variant=ButtonVariant::Ghost on_click=move |_| remove(id) title="Vorlage löschen">
                                            <Icon name="Trash2" class="size-3.5"/>
                                        </Button>
                                    </div>
                                }
                            }).collect_view()}
                        </div>
                    }
                    .into_any()
                }
            }}

            {move || error.get().map(|e| view! { <p class="text-xs text-destructive">{e}</p> })}
        </section>
    }
}
