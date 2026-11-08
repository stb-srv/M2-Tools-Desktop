//! TGA Konverter: wandelt PNG/JPG/BMP/GIF stapelweise in `.tga` um
//! (Port von `src/features/tga-converter/TgaConverter.tsx`).

mod paths;

use crate::components::{Button, ButtonSize, ButtonVariant, Icon, PageHeader};
use crate::store::navigation::Section;
use crate::tauri::{dialog_open, dialog_open_one, invoke, invoke_unit, DialogFilter, OpenDialogOptions};
use leptos::prelude::*;
use leptos::task::spawn_local;
use paths::{base_name_no_ext, default_dest_path, split_path};
use serde_json::json;

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
enum RowStatus {
    Pending,
    Converting,
    Done,
    Error,
}

#[derive(Clone, Debug)]
struct ConvertRow {
    source_path: String,
    dest_path: String,
    preview: Option<String>,
    status: RowStatus,
    error: Option<String>,
}

const IMAGE_EXTENSIONS: [&str; 6] = ["png", "jpg", "jpeg", "bmp", "gif", "tga"];

#[component]
pub fn TgaConverter() -> impl IntoView {
    let rows = RwSignal::new(Vec::<ConvertRow>::new());
    let converting = RwSignal::new(false);

    let set_row = move |index: usize, f: &dyn Fn(&mut ConvertRow)| {
        rows.update(|rs| {
            if let Some(r) = rs.get_mut(index) {
                f(r)
            }
        });
    };

    let pick_files = move |_| {
        spawn_local(async move {
            let opts = OpenDialogOptions {
                title: Some("Bilder auswählen".into()),
                filters: vec![DialogFilter::new("Bild", &IMAGE_EXTENSIONS)],
                multiple: true,
                ..Default::default()
            };
            let paths = dialog_open(&opts).await.unwrap_or_default();
            if paths.is_empty() {
                return;
            }
            let new_rows: Vec<ConvertRow> = paths
                .iter()
                .map(|p| ConvertRow {
                    source_path: p.clone(),
                    dest_path: default_dest_path(p),
                    preview: None,
                    status: RowStatus::Pending,
                    error: None,
                })
                .collect();
            rows.update(|rs| rs.extend(new_rows));
            for p in paths {
                spawn_local(async move {
                    if let Ok(data_url) = invoke::<String>("preview_image_file", &json!({ "path": p })).await {
                        rows.update(|rs| {
                            for r in rs.iter_mut().filter(|r| r.source_path == p) {
                                r.preview = Some(data_url.clone());
                            }
                        });
                    }
                });
            }
        });
    };

    let set_dest_folder = move |_| {
        spawn_local(async move {
            let opts = OpenDialogOptions {
                title: Some("Zielordner für alle Dateien".into()),
                directory: true,
                ..Default::default()
            };
            let Ok(Some(folder)) = dialog_open_one(&opts).await else { return };
            rows.update(|rs| {
                for r in rs.iter_mut() {
                    r.dest_path = format!("{folder}\\{}.tga", base_name_no_ext(split_path(&r.source_path).1));
                }
            });
        });
    };

    let clear_done = move |_| rows.update(|rs| rs.retain(|r| r.status != RowStatus::Done));

    let convert_row = move |index: usize| {
        let Some(row) = rows.with_untracked(|rs| rs.get(index).cloned()) else { return };
        set_row(index, &|r| r.status = RowStatus::Converting);
        spawn_local(async move {
            let res = invoke_unit(
                "convert_image_to_tga",
                &json!({ "sourcePath": row.source_path, "destPath": row.dest_path }),
            )
            .await;
            match res {
                Ok(()) => set_row(index, &|r| r.status = RowStatus::Done),
                Err(e) => set_row(index, &move |r| {
                    r.status = RowStatus::Error;
                    r.error = Some(e.clone());
                }),
            }
        });
    };

    let convert_all = move |_| {
        converting.set(true);
        // Wie im TS: über den Stand zum Zeitpunkt des Klicks iterieren.
        let snapshot = rows.get_untracked();
        spawn_local(async move {
            for (i, row) in snapshot.iter().enumerate() {
                if row.status == RowStatus::Done {
                    continue;
                }
                set_row(i, &|r| r.status = RowStatus::Converting);
                let res = invoke_unit(
                    "convert_image_to_tga",
                    &json!({ "sourcePath": row.source_path, "destPath": row.dest_path }),
                )
                .await;
                match res {
                    Ok(()) => set_row(i, &|r| r.status = RowStatus::Done),
                    Err(e) => set_row(i, &move |r| {
                        r.status = RowStatus::Error;
                        r.error = Some(e.clone());
                    }),
                }
            }
            converting.set(false);
        });
    };

    let row_view = move |(index, source_path): (usize, String)| {
        let field = move |f: fn(&ConvertRow) -> Option<String>| move || rows.with(|rs| rs.get(index).and_then(f));
        let status = move || rows.with(|rs| rs.get(index).map(|r| r.status));
        let src_title = source_path.clone();
        view! {
            <div class="flex items-center gap-3 rounded-md border border-border p-2">
                <div class="flex size-12 shrink-0 items-center justify-center rounded-md border border-border bg-muted/40">
                    {move || match field(|r| r.preview.clone())() {
                        Some(src) => view! { <img src=src alt="" class="max-h-full max-w-full object-contain"/> }.into_any(),
                        None => view! { <Icon name="ImagePlus" class="size-4 text-muted-foreground"/> }.into_any(),
                    }}
                </div>
                <div class="min-w-0 flex-1 space-y-1">
                    <p class="truncate text-xs text-muted-foreground" title=src_title>
                        {source_path}
                    </p>
                    <input
                        prop:value=field(|r| Some(r.dest_path.clone()))
                        on:input=move |e| {
                            let v = event_target_value(&e);
                            set_row(index, &move |r| r.dest_path = v.clone());
                        }
                        class="w-full rounded-md border border-border bg-background px-2 py-1 text-sm"
                    />
                    {move || (status() == Some(RowStatus::Error)).then(|| view! {
                        <p class="flex items-center gap-1 text-xs text-destructive">
                            <Icon name="AlertTriangle" class="size-3.5 shrink-0"/>
                            " "
                            {field(|r| r.error.clone())}
                        </p>
                    })}
                </div>
                <div class="flex shrink-0 items-center gap-2">
                    {move || match status() {
                        Some(RowStatus::Done) => view! { <Icon name="CheckCircle2" class="size-5 text-green-600"/> }.into_any(),
                        Some(RowStatus::Converting) => view! {
                            <span class="size-4 animate-spin rounded-full border-2 border-muted-foreground border-t-transparent"></span>
                        }.into_any(),
                        Some(RowStatus::Error) => view! {
                            <Button size=ButtonSize::Sm variant=ButtonVariant::Outline on_click=move |_| convert_row(index)>
                                "Erneut"
                            </Button>
                        }.into_any(),
                        _ => ().into_any(),
                    }}
                    <Button
                        variant=ButtonVariant::Ghost
                        size=ButtonSize::IconSm
                        on_click=move |_| rows.update(|rs| {
                            if index < rs.len() {
                                rs.remove(index);
                            }
                        })
                    >
                        <Icon name="X" class="size-4"/>
                    </Button>
                </div>
            </div>
        }
    };

    view! {
        <div class="space-y-4">
            <PageHeader module=Section::TgaConverter title="TGA Konverter".to_string()/>
            <p class="max-w-4xl text-sm text-muted-foreground">
                "Wandelt PNG/JPG/BMP/GIF in " <code>".tga"</code>
                " um (dieselbe Konvertierung, die auch beim Icon-Schritt im Item Editor automatisch läuft) — praktisch, um mehrere Icons vorab vorzubereiten oder eine Datei einzeln zu prüfen."
            </p>

            <div class="flex gap-2">
                <Button on_click=pick_files>
                    <Icon name="ImagePlus" class="size-4"/>
                    "Bilder auswählen…"
                </Button>
                <Button
                    variant=ButtonVariant::Outline
                    on_click=set_dest_folder
                    disabled=Signal::derive(move || rows.with(|r| r.is_empty()))
                >
                    <Icon name="FolderOpen" class="size-4"/>
                    "Zielordner für alle setzen"
                </Button>
                <Button
                    on_click=convert_all
                    disabled=Signal::derive(move || {
                        converting.get() || rows.with(|r| r.is_empty() || r.iter().all(|r| r.status == RowStatus::Done))
                    })
                >
                    {move || if converting.get() { "Konvertiere…" } else { "Alle konvertieren" }}
                </Button>
                {move || rows.with(|r| r.iter().any(|r| r.status == RowStatus::Done)).then(|| view! {
                    <Button variant=ButtonVariant::Ghost on_click=clear_done>
                        "Erledigte entfernen"
                    </Button>
                })}
            </div>

            <div class="space-y-2">
                <For
                    each=move || rows.with(|rs| rs.iter().enumerate().map(|(i, r)| (i, r.source_path.clone())).collect::<Vec<_>>())
                    key=|k| k.clone()
                    children=row_view
                />
                {move || rows.with(|r| r.is_empty()).then(|| view! {
                    <p class="text-sm text-muted-foreground">"Noch keine Dateien ausgewählt."</p>
                })}
            </div>
        </div>
    }
}
