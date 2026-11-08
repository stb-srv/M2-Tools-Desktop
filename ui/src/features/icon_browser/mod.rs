//! Icon-Browser: blättert alle Item-Icons (`.tga`) im Client-Ordner durch
//! (Port von `src/features/icon-browser/IconBrowser.tsx`). `IconBrowserModal`
//! ist die Auswahl-Variante für den Icon-Schritt im Item Editor.

use crate::components::{Button, ButtonSize, ButtonVariant, Icon, PageHeader};
use crate::lib::format::de_int;
use crate::store::navigation::Section;
use crate::tauri::{invoke, invoke0};
use leptos::prelude::*;
use leptos::task::spawn_local;
use serde::Deserialize;
use serde_json::json;
use std::collections::{HashMap, HashSet};

#[derive(Clone, Debug, PartialEq, Deserialize)]
struct IconFile {
    absolute_path: String,
    file_name: String,
}

const PAGE_SIZE: usize = 120;

/// Seitenzahl/-begrenzung wie im TS (mindestens eine Seite).
fn page_count(len: usize) -> usize {
    len.div_ceil(PAGE_SIZE).max(1)
}

/// Gemeinsames Raster: listet jede .tga unter dem Item-Icon-Ordner
/// (`list_item_icon_files`) und lädt Vorschaubilder nur für die sichtbare
/// Seite (`load_icon_file`) - bei tausenden Icons wäre alles vorab zu
/// dekodieren langsam und eine riesige IPC-Antwort.
#[component]
fn IconGrid(#[prop(optional, into)] on_pick: Option<Callback<String>>) -> impl IntoView {
    let files = RwSignal::new(None::<Vec<IconFile>>);
    let error = RwSignal::new(None::<String>);
    let query = RwSignal::new(String::new());
    let page = RwSignal::new(0usize);
    let thumbs = RwSignal::new(HashMap::<String, Option<String>>::new());
    // Bereits angefragte Pfade - verhindert doppelte Anfragen, solange eine
    // Antwort noch aussteht.
    let requested = StoredValue::new(HashSet::<String>::new());

    spawn_local(async move {
        match invoke0::<Vec<IconFile>>("list_item_icon_files").await {
            Ok(f) => files.set(Some(f)),
            Err(e) => error.set(Some(e)),
        }
    });

    let filtered = Memo::new(move |_| {
        files.with(|f| {
            let Some(f) = f else { return vec![] };
            let q = query.with(|q| q.trim().to_lowercase());
            if q.is_empty() {
                f.clone()
            } else {
                f.iter().filter(|f| f.file_name.to_lowercase().contains(&q)).cloned().collect()
            }
        })
    });

    let pages = Memo::new(move |_| filtered.with(|f| page_count(f.len())));
    let clamped_page = Memo::new(move |_| page.get().min(pages.get() - 1));
    let visible = Memo::new(move |_| {
        let p = clamped_page.get();
        filtered.with(|f| f.iter().skip(p * PAGE_SIZE).take(PAGE_SIZE).cloned().collect::<Vec<_>>())
    });

    Effect::new(move |_| {
        query.track();
        page.set(0);
    });

    Effect::new(move |_| {
        for f in visible.get() {
            let path = f.absolute_path;
            let already = thumbs.with_untracked(|t| t.contains_key(&path)) || requested.with_value(|r| r.contains(&path));
            if already {
                continue;
            }
            requested.update_value(|r| {
                r.insert(path.clone());
            });
            spawn_local(async move {
                let res = invoke::<String>("load_icon_file", &json!({ "absolutePath": path })).await;
                thumbs.update(|t| {
                    t.insert(path, res.ok());
                });
            });
        }
    });

    move || {
        if let Some(e) = error.get() {
            return view! { <p class="text-sm text-destructive">{e}</p> }.into_any();
        }
        if files.with(|f| f.is_none()) {
            return view! { <p class="text-sm text-muted-foreground">"Lade Icons…"</p> }.into_any();
        }
        view! {
            <div class="flex h-full flex-col gap-2">
                <div class="flex items-center gap-2">
                    <div class="relative flex-1">
                        <Icon name="Search" class="absolute left-2 top-1/2 size-3.5 -translate-y-1/2 text-muted-foreground"/>
                        <input
                            prop:value=move || query.get()
                            on:input=move |e| query.set(event_target_value(&e))
                            placeholder="Dateiname filtern…"
                            class="w-full rounded-md border border-border bg-background py-1 pl-7 pr-2 text-sm"
                        />
                    </div>
                    <span class="shrink-0 text-xs text-muted-foreground">
                        {move || de_int(filtered.with(|f| f.len()) as i64)} " Icons"
                    </span>
                </div>

                <div class="grid min-h-0 flex-1 grid-cols-8 gap-1 overflow-y-auto sm:grid-cols-10">
                    <For each=move || visible.get() key=|f| f.absolute_path.clone() let:f>
                        {
                            let path = f.absolute_path.clone();
                            let pick_path = f.absolute_path.clone();
                            view! {
                                <button
                                    on:click=move |_| {
                                        if let Some(cb) = on_pick {
                                            cb.run(pick_path.clone());
                                        }
                                    }
                                    title=f.file_name.clone()
                                    class="flex flex-col items-center gap-0.5 rounded-md border border-border p-1 hover:bg-muted"
                                >
                                    <div class="flex size-9 items-center justify-center bg-muted/40">
                                        {move || match thumbs.with(|t| t.get(&path).cloned().flatten()) {
                                            Some(src) => view! {
                                                <img src=src alt="" class="max-h-full max-w-full object-contain [image-rendering:pixelated]"/>
                                            }.into_any(),
                                            None => view! { <Icon name="Images" class="size-4 text-muted-foreground"/> }.into_any(),
                                        }}
                                    </div>
                                    <span class="w-full truncate text-[9px] text-muted-foreground">{f.file_name.clone()}</span>
                                </button>
                            }
                        }
                    </For>
                    {move || visible.with(|v| v.is_empty()).then(|| view! {
                        <p class="col-span-full p-4 text-center text-sm text-muted-foreground">"Keine Icons gefunden."</p>
                    })}
                </div>

                {move || (pages.get() > 1).then(|| view! {
                    <div class="flex items-center justify-center gap-2 text-sm">
                        <Button
                            variant=ButtonVariant::Outline
                            size=ButtonSize::IconSm
                            on_click=move |_| page.update(|p| *p = p.saturating_sub(1))
                            disabled=Signal::derive(move || clamped_page.get() == 0)
                        >
                            <Icon name="ChevronLeft" class="size-3.5"/>
                        </Button>
                        <span class="text-xs text-muted-foreground">
                            {move || format!("Seite {} / {}", clamped_page.get() + 1, pages.get())}
                        </span>
                        <Button
                            variant=ButtonVariant::Outline
                            size=ButtonSize::IconSm
                            on_click=move |_| page.update(|p| *p = (*p + 1).min(pages.get_untracked() - 1))
                            disabled=Signal::derive(move || clamped_page.get() >= pages.get() - 1)
                        >
                            <Icon name="ChevronRight" class="size-3.5"/>
                        </Button>
                    </div>
                })}
            </div>
        }
        .into_any()
    }
}

/// Eigenständige Navigationsseite - nur durchblättern, keine Auswahl.
#[component]
pub fn IconBrowser() -> impl IntoView {
    view! {
        <div class="flex h-full flex-col gap-3">
            <PageHeader module=Section::IconBrowser title="Icon-Browser".to_string()/>
            <p class="max-w-4xl text-sm text-muted-foreground">
                "Durchblättert alle Item-Icons (" <code>"pack/icon/icon/item/**.tga"</code>
                ") im konfigurierten Client-Ordner - nützlich um ein Icon nach Aussehen zu finden, statt die vnum-Zuordnung schon zu kennen. Zum tatsächlichen Auswählen eines Icons beim Anlegen eines Items siehe Item Editor."
            </p>
            <div class="min-h-0 flex-1">
                <IconGrid/>
            </div>
        </div>
    }
}

/// Eingebetteter Auswahldialog - für den Icon-Schritt im Item Editor.
#[component]
pub fn IconBrowserModal(#[prop(into)] on_pick: Callback<String>, #[prop(into)] on_close: Callback<()>) -> impl IntoView {
    view! {
        <div class="fixed inset-0 z-40 flex items-center justify-center bg-black/50" on:click=move |_| on_close.run(())>
            <div
                class="flex h-[70vh] w-[48rem] flex-col gap-2 rounded-lg border border-border bg-card p-4"
                on:click=|e| e.stop_propagation()
            >
                <div class="flex items-center justify-between">
                    <p class="text-sm font-medium">"Icon auswählen"</p>
                    <Button variant=ButtonVariant::Ghost size=ButtonSize::Sm on_click=move |_| on_close.run(())>
                        "Schließen"
                    </Button>
                </div>
                <div class="min-h-0 flex-1">
                    <IconGrid on_pick=Callback::new(move |path: String| {
                        on_pick.run(path);
                        on_close.run(());
                    })/>
                </div>
            </div>
        </div>
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn page_count_has_minimum_one() {
        assert_eq!(page_count(0), 1);
        assert_eq!(page_count(120), 1);
        assert_eq!(page_count(121), 2);
        assert_eq!(page_count(240), 2);
    }
}
