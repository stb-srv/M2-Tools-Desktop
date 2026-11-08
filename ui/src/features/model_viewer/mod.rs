//! Model Viewer: lädt ein .gr2-Modell über die granny2.dll des Clients und
//! zeigt es in einer WebGL2-Ansicht (Port von `src/features/model-viewer`).

mod canvas;
mod orbit;
mod renderer;
mod scene;
pub mod types;

pub use canvas::Gr2Canvas;
#[allow(unused_imports)]
pub use types::{Gr2Mesh, Gr2ModelInfo};

use crate::components::{Button, ButtonVariant, Icon, PageHeader};
use crate::i18n::t;
use crate::store::navigation::Section;
use crate::tauri::{dialog_open_one, invoke, DialogFilter, OpenDialogOptions};
use leptos::prelude::*;
use leptos::task::spawn_local;
use serde_json::json;
use std::sync::Arc;

#[component]
pub fn ModelViewer() -> impl IntoView {
    let dll_path = RwSignal::new(String::new());
    let gr2_path = RwSignal::new(String::new());
    let loading = RwSignal::new(false);
    let error = RwSignal::new(None::<String>);
    let info = RwSignal::new(None::<Arc<Gr2ModelInfo>>);

    let pick_dll_path = move |_| {
        spawn_local(async move {
            let opts = OpenDialogOptions {
                title: Some("granny2.dll auswählen".into()),
                filters: vec![DialogFilter::new("Granny2 Library", &["dll"])],
                ..Default::default()
            };
            if let Ok(Some(selected)) = dialog_open_one(&opts).await {
                dll_path.set(selected);
            }
        });
    };

    let pick_gr2_path = move |_| {
        spawn_local(async move {
            let opts = OpenDialogOptions {
                title: Some(".gr2-Modelldatei auswählen".into()),
                filters: vec![DialogFilter::new("Granny3D Model", &["gr2"])],
                ..Default::default()
            };
            if let Ok(Some(selected)) = dialog_open_one(&opts).await {
                gr2_path.set(selected);
            }
        });
    };

    let handle_load = move |_| {
        loading.set(true);
        error.set(None);
        let args = json!({ "grannyDllPath": dll_path.get_untracked(), "gr2Path": gr2_path.get_untracked() });
        spawn_local(async move {
            match invoke::<Gr2ModelInfo>("load_gr2_model", &args).await {
                Ok(result) => info.set(Some(Arc::new(result))),
                Err(e) => {
                    error.set(Some(e));
                    info.set(None);
                }
            }
            loading.set(false);
        });
    };

    view! {
        <div class="flex h-full flex-col gap-4">
            <PageHeader module=Section::ModelViewer title=Signal::derive(|| t("nav.modelViewer"))/>

            <div class="space-y-2 rounded-lg border border-border bg-card p-3">
                <div class="flex items-center gap-2">
                    <Button variant=ButtonVariant::Outline on_click=pick_dll_path class="shrink-0">
                        <Icon name="FolderOpen" class="size-4"/>
                        "granny2.dll auswählen"
                    </Button>
                    <span class="truncate text-sm text-muted-foreground">
                        {move || {
                            let p = dll_path.get();
                            if p.is_empty() { "Keine Datei ausgewählt".to_string() } else { p }
                        }}
                    </span>
                </div>
                <div class="flex items-center gap-2">
                    <Button variant=ButtonVariant::Outline on_click=pick_gr2_path class="shrink-0">
                        <Icon name="FolderOpen" class="size-4"/>
                        ".gr2-Datei auswählen"
                    </Button>
                    <span class="truncate text-sm text-muted-foreground">
                        {move || {
                            let p = gr2_path.get();
                            if p.is_empty() { "Keine Datei ausgewählt".to_string() } else { p }
                        }}
                    </span>
                </div>
                <p class="text-xs text-muted-foreground">
                    "Tipp: " <code>"granny2.dll"</code>
                    " liegt normalerweise direkt im Hauptordner deines Metin2-Clients (neben der .exe). .gr2-Modelle findest du im "
                    <code>"pack"</code> "-Ordner des Clients, meist unter " <code>"ymir work\\npc\\..."</code> ", "
                    <code>"ymir work\\monster\\..."</code> " oder " <code>"ymir work\\item\\..."</code>
                    ". Falls diese Ordner nicht existieren, liegen die Assets noch gepackt (.epk) vor und müssen zuerst entpackt werden."
                </p>
                <Button
                    on_click=handle_load
                    disabled=Signal::derive(move || loading.get() || dll_path.with(|p| p.is_empty()) || gr2_path.with(|p| p.is_empty()))
                >
                    {move || if loading.get() { "Lädt…" } else { "Laden" }}
                </Button>
            </div>

            {move || error.get().map(|e| view! { <p class="text-sm text-destructive">{e}</p> })}
            {move || info.get().map(|i| {
                let skipped = (i.skipped_meshes > 0).then(|| format!(" ({} übersprungen)", i.skipped_meshes));
                view! {
                    <p class="text-sm text-muted-foreground">
                        {format!("{} — {} Bones, {} Mesh(es)", i.name, i.bone_count, i.meshes.len())}
                        {skipped}
                    </p>
                }
            })}

            <Gr2Canvas
                model=Signal::derive(move || info.get())
                class="min-h-0 flex-1 overflow-hidden rounded-lg border border-border"
            />
        </div>
    }
}
