use crate::components::{Button, ButtonSize, ButtonVariant, Icon};
use crate::features::module_importer::shared::join_vnums;
use crate::util::format::de_datetime;
use crate::tauri::{invoke0, invoke_unit};
use leptos::prelude::*;
use leptos::task::spawn_local;
use serde::Deserialize;
use serde_json::json;

#[derive(Clone, Debug, PartialEq, Deserialize)]
struct ImportBatch {
    id: i64,
    module_name: String,
    item_type: i64,
    created_at: String,
    vnums: Vec<i64>,
    #[allow(dead_code)]
    had_effects: bool,
}

async fn refresh(batches: RwSignal<Vec<ImportBatch>>) {
    // Noch kein Verlauf / nicht erreichbar - einfach leeren Zustand zeigen.
    if let Ok(b) = invoke0::<Vec<ImportBatch>>("list_import_batches").await {
        batches.set(b);
    }
}

#[component]
pub fn ImportHistory(#[prop(into)] refresh_key: Signal<u32>) -> impl IntoView {
    let batches = RwSignal::new(Vec::<ImportBatch>::new());
    let undoing_id = RwSignal::new(None::<i64>);
    let confirm_id = RwSignal::new(None::<i64>);
    let error = RwSignal::new(None::<String>);

    Effect::new(move |_| {
        refresh_key.track();
        spawn_local(refresh(batches));
    });

    let undo = move |id: i64| {
        confirm_id.set(None);
        undoing_id.set(Some(id));
        error.set(None);
        spawn_local(async move {
            match invoke_unit("undo_import_batch", &json!({ "id": id })).await {
                Ok(()) => refresh(batches).await,
                Err(e) => error.set(Some(e)),
            }
            undoing_id.set(None);
        });
    };

    let confirm_batch = move || {
        let id = confirm_id.get()?;
        batches.with(|b| b.iter().find(|b| b.id == id).cloned())
    };

    view! {
        <section class="space-y-2 rounded-lg border border-border p-4">
            <h2 class="text-sm font-medium text-muted-foreground">"Importierte Pakete (Verlauf)"</h2>
            {move || if batches.with(|b| b.is_empty()) {
                view! { <p class="text-sm text-muted-foreground">"Noch keine Importe in diesem Verlauf."</p> }.into_any()
            } else {
                view! {
                    <div class="space-y-2">
                        {batches.get().into_iter().map(|b| {
                            let id = b.id;
                            view! {
                                <div class="flex flex-wrap items-center justify-between gap-3 rounded-md border border-border p-2 text-sm">
                                    <div>
                                        <p class="font-medium">
                                            {b.module_name.clone()} " "
                                            <span class="text-xs font-normal text-muted-foreground">
                                                "(" {if b.item_type == 1 { "Waffen" } else { "Rüstung/gemischt" }} ")"
                                            </span>
                                        </p>
                                        <p class="text-xs text-muted-foreground">
                                            {de_datetime(&b.created_at)} " · " {b.vnums.len()} " Item(s): " {join_vnums(&b.vnums)}
                                        </p>
                                    </div>
                                    <Button
                                        variant=ButtonVariant::Destructive
                                        size=ButtonSize::Sm
                                        disabled=Signal::derive(move || undoing_id.get() == Some(id))
                                        on_click=move |_| confirm_id.set(Some(id))
                                    >
                                        <Icon name="Trash2" class="size-3.5"/>
                                        {move || if undoing_id.get() == Some(id) { "Entferne…" } else { "Rückgängig machen" }}
                                    </Button>
                                </div>
                            }
                        }).collect_view()}
                    </div>
                }
                .into_any()
            }}
            {move || error.get().map(|e| view! { <p class="whitespace-pre-wrap text-sm text-destructive">{e}</p> })}

            {move || confirm_batch().map(|b| {
                let id = b.id;
                view! {
                    <div class="fixed inset-0 flex items-center justify-center bg-black/50">
                        <div class="w-[26rem] space-y-3 rounded-lg border border-border bg-card p-4">
                            <p class="text-sm font-medium">
                                {b.vnums.len()} " Item(s) aus „" {b.module_name.clone()} "\" endgültig entfernen?"
                            </p>
                            <ul class="list-disc space-y-1 pl-5 text-sm text-muted-foreground">
                                <li>"Datenbankeinträge werden gelöscht"</li>
                                <li>
                                    "Icons und " <code>"item_list.txt"</code> "-Einträge werden entfernt, " <code>"icon.epk"</code>
                                    " neu gepackt"
                                </li>
                                <li><code>"item_proto"</code> " wird neu erzeugt und im Client ersetzt"</li>
                                <li>"Kopierte 3D-Modell-/Effekt-/msm-Dateien bleiben liegen (harmlos ohne DB-Eintrag)"</li>
                            </ul>
                            <div class="flex justify-end gap-2">
                                <Button variant=ButtonVariant::Outline on_click=move |_| confirm_id.set(None)>"Abbrechen"</Button>
                                <Button variant=ButtonVariant::Destructive on_click=move |_| undo(id)>"Endgültig entfernen"</Button>
                            </div>
                        </div>
                    </div>
                }
            })}
        </section>
    }
}
