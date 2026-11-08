//! Regen-Datei-Editor: Monster-Spawn-Dateien auf dem Server durchsuchen und
//! als Tabelle oder auf der Karte bearbeiten (Portierung von
//! `src/features/regen-editor`).

pub mod line_row;
pub mod map_view;
pub mod model;
pub mod num_input;

use crate::components::button::button_class;
use crate::components::{Button, ButtonSize, ButtonVariant, Icon, PageHeader};
use crate::lib::log_activity;
use crate::store::navigation::{report_section_dirty, Section};
use crate::tauri::invoke;
use leptos::prelude::*;
use leptos::task::spawn_local;
use line_row::RegenLineRow;
use map_view::{MapFolderInfo, RegenMapImage, RegenMapView};
use model::{breadcrumbs, go_up, new_exception, new_spawn, to_relative, RegenLine};
use serde::Deserialize;
use serde_json::json;

#[derive(Clone, Debug, PartialEq, Deserialize)]
struct RemoteEntry {
    name: String,
    is_dir: bool,
}

const DEFAULT_BASE_DIR: &str = "/usr/home/game/share";

#[derive(Clone, Copy, PartialEq, Eq)]
enum EditorMode {
    Table,
    Map,
}

#[derive(Clone, Copy)]
struct State {
    base_dir: RwSignal<Option<String>>,
    dir: RwSignal<Option<String>>,
    entries: RwSignal<Option<Vec<RemoteEntry>>>,
    dir_loading: RwSignal<bool>,
    dir_error: RwSignal<Option<String>>,
    selected_path: RwSignal<Option<String>>,
    lines: RwSignal<Option<Vec<RegenLine>>>,
    content_loading: RwSignal<bool>,
    content_error: RwSignal<Option<String>>,
    dirty: RwSignal<bool>,
    saving: RwSignal<bool>,
    save_ok: RwSignal<Option<String>>,
    save_error: RwSignal<Option<String>>,
    save_confirm: RwSignal<bool>,
    map_image: RwSignal<Option<RegenMapImage>>,
    map_loading: RwSignal<bool>,
    map_error: RwSignal<Option<String>>,
    map_folders: RwSignal<Option<Vec<MapFolderInfo>>>,
    map_folders_error: RwSignal<Option<String>>,
    show_map_picker: RwSignal<bool>,
    map_selection: RwSignal<Option<(String, String)>>,
}

impl State {
    fn new() -> Self {
        Self {
            base_dir: RwSignal::new(None),
            dir: RwSignal::new(None),
            entries: RwSignal::new(None),
            dir_loading: RwSignal::new(false),
            dir_error: RwSignal::new(None),
            selected_path: RwSignal::new(None),
            lines: RwSignal::new(None),
            content_loading: RwSignal::new(false),
            content_error: RwSignal::new(None),
            dirty: RwSignal::new(false),
            saving: RwSignal::new(false),
            save_ok: RwSignal::new(None),
            save_error: RwSignal::new(None),
            save_confirm: RwSignal::new(false),
            map_image: RwSignal::new(None),
            map_loading: RwSignal::new(false),
            map_error: RwSignal::new(None),
            map_folders: RwSignal::new(None),
            map_folders_error: RwSignal::new(None),
            show_map_picker: RwSignal::new(false),
            map_selection: RwSignal::new(None),
        }
    }

    async fn load_map_folders(self) {
        match crate::tauri::invoke0::<Vec<MapFolderInfo>>("list_client_map_folders").await {
            Ok(f) => self.map_folders.set(Some(f)),
            Err(e) => self.map_folders_error.set(Some(e)),
        }
    }

    async fn load_map_image(self, category: String, folder_name: String, force_rebuild: bool) {
        self.map_selection.set(Some((category.clone(), folder_name.clone())));
        self.map_loading.set(true);
        self.map_error.set(None);
        match invoke::<RegenMapImage>(
            "get_regen_map_image",
            &json!({ "category": category, "folderName": folder_name, "forceRebuild": force_rebuild }),
        )
        .await
        {
            Ok(img) => self.map_image.set(Some(img)),
            Err(e) => self.map_error.set(Some(e)),
        }
        self.map_loading.set(false);
    }

    async fn choose_map_folder(self, category: String, folder_name: String) {
        let Some(path) = self.selected_path.get_untracked() else { return };
        self.show_map_picker.set(false);
        let _ = crate::tauri::invoke_unit(
            "set_setting",
            &json!({ "key": format!("regen_map_mapping:{path}"), "value": format!("{category}/{folder_name}") }),
        )
        .await;
        self.load_map_image(category, folder_name, false).await;
    }

    async fn load_dir(self, path: String) {
        self.dir_loading.set(true);
        self.dir_error.set(None);
        match invoke::<Vec<RemoteEntry>>("list_remote_dir", &json!({ "path": path })).await {
            Ok(e) => self.entries.set(Some(e)),
            Err(e) => self.dir_error.set(Some(e)),
        }
        self.dir_loading.set(false);
    }

    fn rel(self, absolute: &str) -> String {
        self.base_dir.with_untracked(|b| to_relative(b.as_deref(), absolute))
    }

    async fn open_file(self, name: String) {
        let (Some(dir), Some(_)) = (self.dir.get_untracked(), self.base_dir.get_untracked()) else { return };
        let relative = self.rel(&format!("{dir}/{name}"));
        self.selected_path.set(Some(relative.clone()));
        self.lines.set(None);
        self.content_error.set(None);
        self.dirty.set(false);
        self.save_ok.set(None);
        self.save_error.set(None);
        self.content_loading.set(true);
        match invoke::<Vec<RegenLine>>("read_regen_file", &json!({ "relativePath": relative })).await {
            Ok(l) => self.lines.set(Some(l)),
            Err(e) => self.content_error.set(Some(e)),
        }
        self.content_loading.set(false);
    }

    fn update_line(self, index: usize, line: RegenLine) {
        self.lines.update(|l| {
            if let Some(v) = l {
                if let Some(slot) = v.get_mut(index) {
                    *slot = line;
                }
            }
        });
        self.dirty.set(true);
        self.save_ok.set(None);
    }

    fn remove_line(self, index: usize) {
        self.lines.update(|l| {
            if let Some(v) = l {
                if index < v.len() {
                    v.remove(index);
                }
            }
        });
        self.dirty.set(true);
        self.save_ok.set(None);
    }

    fn add_line(self, line: RegenLine) {
        self.lines.update(|l| l.get_or_insert_with(Vec::new).push(line));
        self.dirty.set(true);
        self.save_ok.set(None);
    }

    async fn save_file(self) {
        let (Some(path), Some(lines)) = (self.selected_path.get_untracked(), self.lines.get_untracked()) else {
            return;
        };
        self.save_confirm.set(false);
        self.saving.set(true);
        self.save_error.set(None);
        match invoke::<Option<String>>("write_regen_file", &json!({ "relativePath": path, "lines": lines })).await {
            Ok(backup) => {
                self.save_ok.set(Some(match backup {
                    Some(b) if !b.is_empty() => format!("Gespeichert. Backup: {b}"),
                    _ => "Gespeichert.".to_string(),
                }));
                self.dirty.set(false);
                log_activity(
                    Section::RegenEditor.into(),
                    "update",
                    &format!("Regen-Datei '{path}' gespeichert"),
                    Some("regen-file"),
                    Some(&path),
                );
            }
            Err(e) => self.save_error.set(Some(e)),
        }
        self.saving.set(false);
    }
}

#[component]
pub fn RegenEditor() -> impl IntoView {
    let s = State::new();
    // Karten-Ansicht: alternative Darstellung derselben `lines` - keine eigene
    // Datenquelle, siehe RegenMapView.
    let editor_mode = RwSignal::new(EditorMode::Table);

    Effect::new(move |_| report_section_dirty(Section::RegenEditor, s.dirty.get()));
    on_cleanup(|| report_section_dirty(Section::RegenEditor, false));

    // Entspricht den zwei React-Effekten [selectedPath] (Karte zurücksetzen)
    // und [editorMode, selectedPath] (Zuordnung laden). Der zweite sah dabei
    // die Werte *vor* dem Zurücksetzen - das bleibt hier genauso.
    let prev_path = StoredValue::new(None::<Option<String>>);
    Effect::new(move |_| {
        let mode = editor_mode.get();
        let path = s.selected_path.get();
        let had_image = s.map_image.with_untracked(|m| m.is_some());
        let was_loading = s.map_loading.get_untracked();
        if prev_path.get_value().as_ref() != Some(&path) {
            prev_path.set_value(Some(path.clone()));
            s.map_image.set(None);
            s.map_error.set(None);
            s.show_map_picker.set(false);
            s.map_selection.set(None);
        }
        let Some(path) = path else { return };
        if mode != EditorMode::Map || had_image || was_loading {
            return;
        }
        spawn_local(async move {
            let mapping = invoke::<Option<String>>("get_setting", &json!({ "key": format!("regen_map_mapping:{path}") }))
                .await
                .ok()
                .flatten();
            if let Some(mapping) = mapping.filter(|m| !m.is_empty()) {
                let mut parts = mapping.split('/');
                let category = parts.next().unwrap_or("").to_string();
                let folder_name = parts.next().unwrap_or("").to_string();
                if !category.is_empty() && !folder_name.is_empty() {
                    s.load_map_image(category, folder_name, false).await;
                    return;
                }
            }
            s.show_map_picker.set(true);
            if s.map_folders.with_untracked(|f| f.is_none()) {
                s.load_map_folders().await;
            }
        });
    });

    spawn_local(async move {
        let configured = invoke::<Option<String>>("get_setting", &json!({ "key": "regen_base_dir" }))
            .await
            .ok()
            .flatten()
            .filter(|v| !v.is_empty())
            .unwrap_or_else(|| DEFAULT_BASE_DIR.to_string());
        s.base_dir.set(Some(configured.clone()));
        s.dir.set(Some(configured));
    });

    Effect::new(move |_| {
        if let Some(d) = s.dir.get() {
            spawn_local(s.load_dir(d));
        }
    });

    let crumbs = Memo::new(move |_| {
        let d = s.dir.get();
        let b = s.base_dir.get();
        breadcrumbs(d.as_deref(), b.as_deref())
    });

    let mode_button = move |mode: EditorMode, icon: &'static str, label: &'static str| {
        view! {
            <button
                type="button"
                data-slot="button"
                class=move || button_class(
                    if editor_mode.get() == mode { ButtonVariant::Secondary } else { ButtonVariant::Ghost },
                    ButtonSize::Sm,
                    "",
                )
                on:click=move |_| editor_mode.set(mode)
            >
                <Icon name=icon class="size-3.5"/>
                {label}
            </button>
        }
    };

    let line_count = Memo::new(move |_| s.lines.with(|l| l.as_ref().map(|v| v.len())));

    view! {
        <div class="flex h-full flex-col gap-3">
            <div>
                <PageHeader module=Section::RegenEditor title="Regen-Datei-Editor"/>
                <p class="max-w-4xl text-sm text-muted-foreground">
                    "Bearbeitet die Monster-Spawn-Dateien, auf die Dungeon-Etagen im Quest Builder verweisen (z.B. "
                    <code>"data/dungeon/dt_short/deviltower3_regen.txt"</code>
                    "). Format 1:1 aus dem Server-Quellcode verifiziert ("
                    <code>"regen.cpp"</code>
                    ")."
                </p>
            </div>

            <div class="flex min-h-0 flex-1 gap-4">
                <div class="flex w-80 shrink-0 flex-col gap-2">
                    <div class="flex items-center gap-1 text-sm">
                        <Button variant=ButtonVariant::Ghost size=ButtonSize::IconSm on_click=move |_| s.dir.set(s.base_dir.get_untracked())>
                            <Icon name="Home" class="size-3.5"/>
                        </Button>
                        {move || {
                            let parts = crumbs.get();
                            parts
                                .iter()
                                .enumerate()
                                .map(|(i, part)| {
                                    let target = parts[..=i].join("/");
                                    view! {
                                        <span class="flex items-center gap-1 text-muted-foreground">
                                            <Icon name="ChevronRight" class="size-3.5"/>
                                            <button
                                                class="hover:text-foreground"
                                                on:click=move |_| {
                                                    let base = s.base_dir.get_untracked().unwrap_or_else(|| "null".into());
                                                    s.dir.set(Some(format!("{base}/{target}")));
                                                }
                                            >
                                                {part.clone()}
                                            </button>
                                        </span>
                                    }
                                })
                                .collect_view()
                        }}
                        <Button
                            variant=ButtonVariant::Ghost
                            size=ButtonSize::IconSm
                            class="ml-auto"
                            on_click=move |_| {
                                if let Some(d) = s.dir.get_untracked() {
                                    spawn_local(s.load_dir(d));
                                }
                            }
                        >
                            <Icon
                                name="RefreshCw"
                                class=Signal::derive(move || format!("size-3.5 {}", if s.dir_loading.get() { "animate-spin" } else { "" }))
                            />
                        </Button>
                    </div>

                    {move || s.dir_error.get().map(|e| view! { <p class="text-sm text-destructive">{e}</p> })}

                    <div class="flex-1 space-y-1 overflow-y-auto rounded-md border border-border p-1">
                        <Show when=move || {
                            let d = s.dir.get();
                            let b = s.base_dir.get();
                            matches!((d, b), (Some(d), Some(b)) if !d.is_empty() && !b.is_empty() && d != b)
                        }>
                            <button
                                on:click=move |_| {
                                    let d = s.dir.get_untracked();
                                    let b = s.base_dir.get_untracked();
                                    s.dir.set(go_up(d.as_deref(), b.as_deref()));
                                }
                                class="flex w-full items-center gap-2 rounded-md px-2 py-1 text-left text-sm hover:bg-muted"
                            >
                                <Icon name="Folder" class="size-4 text-muted-foreground"/>
                                ".."
                            </button>
                        </Show>
                        <For each=move || s.entries.get().unwrap_or_default() key=|e| (e.name.clone(), e.is_dir) let:entry>
                            {
                                let name = entry.name.clone();
                                let is_dir = entry.is_dir;
                                let name_for_class = name.clone();
                                let selected = move || {
                                    if is_dir {
                                        return false;
                                    }
                                    let Some(d) = s.dir.get().filter(|d| !d.is_empty()) else { return false };
                                    let rel = s.base_dir.with(|b| to_relative(b.as_deref(), &format!("{d}/{name_for_class}")));
                                    s.selected_path.with(|p| p.as_deref() == Some(rel.as_str()))
                                };
                                let click_name = name.clone();
                                view! {
                                    <button
                                        on:click=move |_| {
                                            if is_dir {
                                                s.dir.update(|d| *d = Some(format!("{}/{}", d.clone().unwrap_or_else(|| "null".into()), click_name)));
                                            } else {
                                                spawn_local(s.open_file(click_name.clone()));
                                            }
                                        }
                                        class=move || format!(
                                            "flex w-full items-center gap-2 rounded-md px-2 py-1 text-left text-sm hover:bg-muted {}",
                                            if selected() { "bg-muted" } else { "" },
                                        )
                                    >
                                        <Icon name=if is_dir { "Folder" } else { "FileText" } class="size-4 text-muted-foreground"/>
                                        {name}
                                    </button>
                                }
                            }
                        </For>
                        <Show when=move || s.entries.with(|e| e.as_ref().is_some_and(|e| e.is_empty()))>
                            <p class="p-2 text-sm text-muted-foreground">"Leerer Ordner."</p>
                        </Show>
                    </div>
                </div>

                <div class="flex min-w-0 flex-1 flex-col gap-2">
                    <Show
                        when=move || s.selected_path.with(|p| p.is_some())
                        fallback=|| view! {
                            <div class="flex flex-1 flex-col items-center justify-center rounded-lg border border-dashed border-border">
                                <p class="text-sm text-muted-foreground">"Wähle links eine Regen-Datei (.txt) aus."</p>
                            </div>
                        }
                    >
                        <div class="flex items-center justify-between">
                            <code class="text-sm">{move || s.selected_path.get().unwrap_or_default()}</code>
                            <div class="flex items-center gap-2">
                                {move || s.save_ok.get().map(|m| view! {
                                    <span class="flex items-center gap-1 text-sm text-green-600">
                                        <Icon name="CheckCircle2" class="size-4"/>
                                        " "
                                        {m}
                                    </span>
                                })}
                                <div class="flex items-center rounded-md border border-border p-0.5">
                                    {mode_button(EditorMode::Table, "Table2", "Tabelle")}
                                    {mode_button(EditorMode::Map, "Map", "Karte")}
                                </div>
                                <Button variant=ButtonVariant::Outline size=ButtonSize::Sm on_click=move |_| s.add_line(new_spawn())>
                                    <Icon name="Plus" class="size-3.5"/>
                                    "Spawn"
                                </Button>
                                <Button variant=ButtonVariant::Outline size=ButtonSize::Sm on_click=move |_| s.add_line(new_exception())>
                                    <Icon name="Plus" class="size-3.5"/>
                                    "Ausnahmezone"
                                </Button>
                                <Button
                                    size=ButtonSize::Sm
                                    on_click=move |_| s.save_confirm.set(true)
                                    disabled=Signal::derive(move || s.saving.get() || !s.dirty.get())
                                >
                                    {move || if s.saving.get() { "Speichere…" } else { "Speichern" }}
                                </Button>
                            </div>
                        </div>
                        {move || s.save_error.get().map(|e| view! { <p class="text-sm text-destructive">{e}</p> })}
                        {move || s.content_error.get().map(|e| view! { <p class="text-sm text-destructive">{e}</p> })}
                        <Show when=move || s.content_loading.get()>
                            <p class="text-sm text-muted-foreground">"Lade…"</p>
                        </Show>

                        <Show when=move || line_count.get().is_some() && editor_mode.get() == EditorMode::Table>
                            <div class="flex-1 space-y-1 overflow-y-auto">
                                <For each=move || 0..line_count.get().unwrap_or(0) key=|i| *i let:index>
                                    <RegenLineRow
                                        line=Signal::derive(move || {
                                            s.lines
                                                .with(|l| l.as_ref().and_then(|v| v.get(index).cloned()))
                                                .unwrap_or(RegenLine::Raw { text: String::new() })
                                        })
                                        on_change=move |l| s.update_line(index, l)
                                        on_remove=move |_| s.remove_line(index)
                                    />
                                </For>
                                <Show when=move || line_count.get() == Some(0)>
                                    <p class="p-2 text-sm text-muted-foreground">"Leere Datei."</p>
                                </Show>
                            </div>
                        </Show>

                        <Show when=move || line_count.get().is_some() && editor_mode.get() == EditorMode::Map>
                            <div class="min-h-0 flex-1">
                                <Show when=move || s.map_loading.get()>
                                    <p class="p-2 text-sm text-muted-foreground">"Lade Karte…"</p>
                                </Show>
                                {move || s.map_error.get().map(|e| view! { <p class="p-2 text-sm text-destructive">{e}</p> })}
                                <Show when=move || s.map_image.with(|m| m.is_some())>
                                    <RegenMapView
                                        lines=Signal::derive(move || s.lines.get().unwrap_or_default())
                                        map_image=Signal::derive(move || {
                                            s.map_image.get().unwrap_or(RegenMapImage {
                                                image_data_url: String::new(),
                                                units_per_pixel: 1.0,
                                                width_px: 1.0,
                                                height_px: 1.0,
                                            })
                                        })
                                        on_update_line=move |(i, l)| s.update_line(i, l)
                                        on_add_line=move |l| s.add_line(l)
                                        on_remove_line=move |i| s.remove_line(i)
                                        has_reload_map=Signal::derive(move || s.map_selection.with(|m| m.is_some()))
                                        on_reload_map=move |_| {
                                            if let Some((c, f)) = s.map_selection.get_untracked() {
                                                spawn_local(s.load_map_image(c, f, true));
                                            }
                                        }
                                        on_change_map_folder=move |_| {
                                            s.show_map_picker.set(true);
                                            if s.map_folders.with_untracked(|f| f.is_none()) {
                                                spawn_local(s.load_map_folders());
                                            }
                                        }
                                    />
                                </Show>
                                <Show when=move || {
                                    !s.map_loading.get()
                                        && s.map_image.with(|m| m.is_none())
                                        && s.map_error.with(|e| e.is_none())
                                        && !s.show_map_picker.get()
                                }>
                                    <p class="p-2 text-sm text-muted-foreground">"Keine Karte zugeordnet."</p>
                                </Show>
                            </div>
                        </Show>
                    </Show>
                </div>
            </div>

            <Show when=move || s.show_map_picker.get()>
                <div class="fixed inset-0 flex items-center justify-center bg-black/50">
                    <div class="max-h-[70vh] w-[32rem] space-y-3 overflow-y-auto rounded-lg border border-border bg-card p-4">
                        <h2 class="text-sm font-medium">"Kartenordner zuordnen"</h2>
                        <p class="text-xs text-muted-foreground">
                            "Wähle den Client-Kartenordner, der zu "
                            <code>{move || s.selected_path.get().unwrap_or_default()}</code>
                            " gehört. Die Zuordnung wird für diese Datei gemerkt."
                        </p>
                        {move || s.map_folders_error.get().map(|e| view! { <p class="text-sm text-destructive">{e}</p> })}
                        <Show when=move || s.map_folders.with(|f| f.is_none()) && s.map_folders_error.with(|e| e.is_none())>
                            <p class="text-sm text-muted-foreground">"Lade Kartenordner…"</p>
                        </Show>
                        <Show when=move || s.map_folders.with(|f| f.as_ref().is_some_and(|f| f.is_empty()))>
                            <p class="text-sm text-muted-foreground">
                                "Keine entpackten Kartenordner unter dem konfigurierten Client-Pfad gefunden."
                            </p>
                        </Show>
                        <div class="space-y-2">
                            {move || {
                                let folders = s.map_folders.get().unwrap_or_default();
                                let mut groups: Vec<(String, Vec<MapFolderInfo>)> = Vec::new();
                                for f in folders {
                                    match groups.iter_mut().find(|(c, _)| *c == f.category) {
                                        Some((_, v)) => v.push(f),
                                        None => groups.push((f.category.clone(), vec![f])),
                                    }
                                }
                                groups
                                    .into_iter()
                                    .map(|(category, folders)| {
                                        view! {
                                            <div>
                                                <p class="text-xs font-medium text-muted-foreground">{category}</p>
                                                <div class="flex flex-wrap gap-1 pt-1">
                                                    {folders
                                                        .into_iter()
                                                        .map(|f| {
                                                            let label = f.folder_name.clone();
                                                            view! {
                                                                <button
                                                                    on:click=move |_| {
                                                                        spawn_local(s.choose_map_folder(f.category.clone(), f.folder_name.clone()))
                                                                    }
                                                                    class="rounded-md border border-border px-2 py-1 text-xs hover:bg-muted"
                                                                >
                                                                    {label}
                                                                </button>
                                                            }
                                                        })
                                                        .collect_view()}
                                                </div>
                                            </div>
                                        }
                                    })
                                    .collect_view()
                            }}
                        </div>
                        <div class="flex justify-end">
                            <Button variant=ButtonVariant::Outline size=ButtonSize::Sm on_click=move |_| s.show_map_picker.set(false)>
                                "Abbrechen"
                            </Button>
                        </div>
                    </div>
                </div>
            </Show>

            <Show when=move || s.save_confirm.get()>
                <div class="fixed inset-0 flex items-center justify-center bg-black/50">
                    <div class="w-96 space-y-3 rounded-lg border border-border bg-card p-4">
                        <p class="flex items-start gap-2 text-sm">
                            <Icon name="AlertTriangle" class="mt-0.5 size-4 shrink-0 text-destructive"/>
                            <span>"Die Datei auf dem Server wird gesichert und dann überschrieben."</span>
                        </p>
                        <div class="flex justify-end gap-2">
                            <Button variant=ButtonVariant::Outline on_click=move |_| s.save_confirm.set(false)>
                                "Abbrechen"
                            </Button>
                            <Button on_click=move |_| spawn_local(s.save_file())>"Speichern"</Button>
                        </div>
                    </div>
                </div>
            </Show>
        </div>
    }
}
