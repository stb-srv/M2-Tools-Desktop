//! Paket-Import: Import-Pakete im Format m2pkg v1 (vom DB-Tool erzeugt) prüfen,
//! in einer Vorschau zeigen und in Client und Server einspielen. Völlig
//! getrennt vom Datei-/Ordner-Importer: Items, Namen und Dateizuordnung
//! kommen komplett aus der `manifest.json` des Pakets.

use crate::components::{toast, Button, ButtonSize, ButtonVariant, Icon};
use crate::tauri::{dialog_open_one, invoke, invoke0, invoke_unit, DialogFilter, OpenDialogOptions};
use crate::util::format::{bytes, de_datetime};
use leptos::prelude::*;
use leptos::task::spawn_local;
use serde::Deserialize;
use serde_json::json;
use std::collections::{BTreeMap, HashSet};

#[derive(Clone, Debug, PartialEq, Deserialize)]
struct Warning {
    vnum: Option<u32>,
    code: String,
    message: String,
}

#[derive(Clone, Debug, PartialEq, Deserialize)]
struct Assumption {
    name: String,
    bit: u64,
    #[serde(default)]
    basis: String,
}

#[derive(Clone, Debug, PartialEq, Deserialize)]
struct PkgFile {
    path: String,
    role: String,
    guessed: bool,
    size: u64,
    used_by: usize,
}

#[derive(Clone, Debug, PartialEq, Deserialize)]
struct PkgItem {
    vnum: u32,
    name: Option<String>,
    item_type: String,
    icon_data_url: Option<String>,
    files: Vec<PkgFile>,
    assumed_antiflags: Vec<String>,
    warnings: Vec<Warning>,
    has_proto: bool,
}

#[derive(Clone, Debug, PartialEq, Deserialize)]
struct Inspect {
    zip_path: String,
    dbtool_version: Option<String>,
    created: Option<String>,
    collection: String,
    client_version: Option<String>,
    proto_source: Option<String>,
    assumptions: Vec<Assumption>,
    global_warnings: Vec<Warning>,
    items: Vec<PkgItem>,
    file_count: usize,
    shared_file_count: usize,
    has_sql: bool,
}

#[derive(Clone, Debug, PartialEq, Deserialize)]
struct Preflight {
    existing_in_db: Option<Vec<u32>>,
    existing_in_client_list: Vec<u32>,
    differing_files: Vec<String>,
    identical_files: usize,
    item_list_files: usize,
    item_names_files: usize,
    notes: Vec<String>,
}

#[derive(Clone, Debug, PartialEq, Deserialize)]
struct DbReport {
    inserted: usize,
    replaced: usize,
    kept: usize,
}

#[derive(Clone, Debug, PartialEq, Deserialize)]
struct Report {
    backup_id: String,
    backup_dir: String,
    items_selected: usize,
    files_created: usize,
    files_replaced: usize,
    files_unchanged: usize,
    files_kept: Vec<String>,
    list_added: usize,
    list_replaced: usize,
    list_kept: usize,
    db_written: Option<DbReport>,
    server_files: Vec<String>,
    notes: Vec<String>,
}

#[derive(Clone, Debug, PartialEq, Deserialize)]
struct ImportRecord {
    id: String,
    created_at: String,
    collection: String,
    package_file: String,
    item_count: usize,
    db_applied: bool,
    undone: bool,
}

#[derive(Clone, Debug, PartialEq, Deserialize)]
struct UndoReport {
    files_removed: usize,
    files_restored: usize,
    db_deleted: usize,
    db_restored: usize,
}

const KEY_DB: &str = "m2pkg_write_db";
const KEY_FILES: &str = "m2pkg_write_files";
const KEY_OUT: &str = "m2pkg_out_dir";
const KEY_CONFLICT: &str = "m2pkg_conflict";

async fn load_setting(key: &str) -> Option<String> {
    invoke::<Option<String>>("get_setting", &json!({ "key": key })).await.ok().flatten()
}

fn save_setting(key: &'static str, value: String) {
    spawn_local(async move {
        let _ = invoke_unit("set_setting", &json!({ "key": key, "value": value })).await;
    });
}

fn join_vnums(v: &[u32]) -> String {
    let mut s: Vec<String> = v.iter().take(12).map(|n| n.to_string()).collect();
    if v.len() > 12 {
        s.push(format!("… (+{})", v.len() - 12));
    }
    s.join(", ")
}

fn role_label(role: &str) -> &'static str {
    match role {
        "icon" => "Icon",
        "model" => "Modell",
        "texture" => "Textur",
        "effect" => "Effekt",
        _ => "Datei",
    }
}

#[component]
pub fn M2PkgImporter(#[prop(into)] on_imported: Callback<()>) -> impl IntoView {
    let inspect = RwSignal::new(None::<Inspect>);
    let loading = RwSignal::new(false);
    let error = RwSignal::new(None::<String>);
    let selected = RwSignal::new(HashSet::<u32>::new());
    let guessed_ok = RwSignal::new(HashSet::<String>::new());
    let confirm_flags = RwSignal::new(false);

    let write_db = RwSignal::new(true);
    let write_files = RwSignal::new(true);
    let out_dir = RwSignal::new(String::new());
    // "ask" | "overwrite" | "keep"
    let conflict_pref = RwSignal::new("ask".to_string());

    let preflight = RwSignal::new(None::<Preflight>);
    let dialog_conflict = RwSignal::new("keep".to_string());
    let checking = RwSignal::new(false);
    let running = RwSignal::new(false);
    let report = RwSignal::new(None::<Report>);
    let history = RwSignal::new(Vec::<ImportRecord>::new());
    let undo_confirm = RwSignal::new(None::<String>);
    let undoing = RwSignal::new(false);

    let refresh_history = move || {
        spawn_local(async move {
            if let Ok(h) = invoke0::<Vec<ImportRecord>>("m2pkg_list_imports").await {
                history.set(h);
            }
        });
    };

    // gespeicherte Einstellungen laden
    spawn_local(async move {
        if let Some(v) = load_setting(KEY_DB).await {
            write_db.set(v != "0");
        }
        if let Some(v) = load_setting(KEY_FILES).await {
            write_files.set(v != "0");
        }
        if let Some(v) = load_setting(KEY_OUT).await {
            out_dir.set(v);
        }
        if let Some(v) = load_setting(KEY_CONFLICT).await {
            if ["ask", "overwrite", "keep"].contains(&v.as_str()) {
                conflict_pref.set(v);
            }
        }
    });
    refresh_history();

    let choose_zip = move || {
        spawn_local(async move {
            let picked = dialog_open_one(&OpenDialogOptions {
                title: Some("Import-Paket (ZIP) wählen".into()),
                filters: vec![DialogFilter::new("Import-Paket", &["zip"])],
                ..Default::default()
            })
            .await;
            let Ok(Some(path)) = picked else { return };
            loading.set(true);
            error.set(None);
            report.set(None);
            inspect.set(None);
            match invoke::<Inspect>("m2pkg_inspect", &json!({ "zipPath": path })).await {
                Ok(i) => {
                    // Standard: alles gewählt, vermutete Dateien nicht
                    selected.set(i.items.iter().map(|x| x.vnum).collect());
                    guessed_ok.set(HashSet::new());
                    confirm_flags.set(false);
                    inspect.set(Some(i));
                }
                Err(e) => error.set(Some(e)),
            }
            loading.set(false);
        });
    };

    let choose_out_dir = move || {
        spawn_local(async move {
            if let Ok(Some(p)) =
                dialog_open_one(&OpenDialogOptions { directory: true, title: Some("Ordner für die SQL-Dateien".into()), ..Default::default() }).await
            {
                out_dir.set(p.clone());
                save_setting(KEY_OUT, p);
            }
        });
    };

    // Items mit angenommenen AntiFlags unter den gewählten
    let flagged = Memo::new(move |_| {
        inspect.with(|i| {
            i.as_ref()
                .map(|i| {
                    selected.with(|s| {
                        let mut names: Vec<String> = i
                            .items
                            .iter()
                            .filter(|x| s.contains(&x.vnum))
                            .flat_map(|x| x.assumed_antiflags.clone())
                            .collect();
                        names.sort();
                        names.dedup();
                        names
                    })
                })
                .unwrap_or_default()
        })
    });

    let can_import = Memo::new(move |_| {
        inspect.with(|i| i.is_some())
            && !selected.with(|s| s.is_empty())
            && (write_db.get() || write_files.get())
            && (flagged.with(|f| f.is_empty()) || confirm_flags.get())
            && !running.get()
            && !checking.get()
    });

    let options_json = move |conflict: &str| {
        let Some(zip) = inspect.with(|i| i.as_ref().map(|i| i.zip_path.clone())) else { return json!({}) };
        let mut vnums: Vec<u32> = selected.with(|s| s.iter().copied().collect());
        vnums.sort_unstable();
        let mut g: Vec<String> = guessed_ok.with(|g| g.iter().cloned().collect());
        g.sort();
        json!({ "options": {
            "zip_path": zip,
            "vnums": vnums,
            "guessed_paths": g,
            "confirm_assumptions": confirm_flags.get_untracked(),
            "write_db": write_db.get_untracked(),
            "write_files": write_files.get_untracked(),
            "out_dir": out_dir.get_untracked(),
            "conflict": conflict,
        }})
    };

    // Schritt 1: Vorprüfung, dann Bestätigungsdialog
    let start_check = move || {
        checking.set(true);
        error.set(None);
        let args = options_json(if conflict_pref.get_untracked() == "overwrite" { "overwrite" } else { "keep" });
        spawn_local(async move {
            match invoke::<Preflight>("m2pkg_preflight", &args).await {
                Ok(p) => {
                    dialog_conflict.set(if conflict_pref.get_untracked() == "overwrite" { "overwrite".into() } else { "keep".into() });
                    preflight.set(Some(p));
                }
                Err(e) => error.set(Some(e)),
            }
            checking.set(false);
        });
    };

    // Schritt 2: wirklich einspielen
    let run_import = move || {
        let pref = conflict_pref.get_untracked();
        let conflict = if pref == "ask" { dialog_conflict.get_untracked() } else { pref };
        let args = options_json(&conflict);
        preflight.set(None);
        running.set(true);
        error.set(None);
        spawn_local(async move {
            match invoke::<Report>("m2pkg_import", &args).await {
                Ok(r) => {
                    toast::success("Paket eingespielt");
                    report.set(Some(r));
                    on_imported.run(());
                    refresh_history();
                }
                Err(e) => error.set(Some(e)),
            }
            running.set(false);
        });
    };

    let run_undo = move |id: String| {
        undo_confirm.set(None);
        undoing.set(true);
        error.set(None);
        spawn_local(async move {
            match invoke::<UndoReport>("m2pkg_undo", &json!({ "id": id })).await {
                Ok(u) => {
                    toast::success(&format!(
                        "Rückgängig gemacht: {} Datei(en) entfernt, {} wiederhergestellt, {} Datenbankzeile(n) gelöscht, {} wiederhergestellt",
                        u.files_removed, u.files_restored, u.db_deleted, u.db_restored
                    ));
                    report.set(None);
                    refresh_history();
                }
                Err(e) => error.set(Some(e)),
            }
            undoing.set(false);
        });
    };

    // vermutete Dateien einmalig (geteilte nur ein Mal) auflisten
    let guessed_files = Memo::new(move |_| {
        inspect.with(|i| {
            let mut m: BTreeMap<String, (String, u64, usize)> = BTreeMap::new();
            if let Some(i) = i {
                for it in &i.items {
                    for f in it.files.iter().filter(|f| f.guessed) {
                        m.entry(f.path.clone()).or_insert((f.role.clone(), f.size, f.used_by));
                    }
                }
            }
            m.into_iter().collect::<Vec<_>>()
        })
    });

    view! {
        <div class="space-y-4">
            <p class="text-sm text-muted-foreground">
                "Spielt Import-Pakete aus dem DB-Tool (Format m2pkg) ein: Items, Namen und Dateizuordnung kommen vollständig aus dem Paket. "
                "Vor dem Einspielen wird alles gesichert; am Ende gibt es einen Bericht und „Rückgängig machen“."
            </p>

            <div class="flex flex-wrap items-center gap-3">
                <Button on_click=move |_| choose_zip() disabled=Signal::derive(move || loading.get() || running.get())>
                    <Icon name="FolderOpen" class="size-4"/>
                    "Paket (ZIP) wählen…"
                </Button>
                {move || loading.get().then(|| view! { <span class="text-sm text-muted-foreground">"Prüfe Paket und Prüfsummen…"</span> })}
            </div>

            {move || error.get().map(|e| view! {
                <p class="whitespace-pre-wrap rounded-md border border-destructive/40 bg-destructive/10 p-3 text-sm text-destructive">{e}</p>
            })}

            {move || inspect.get().map(|i| {
                let total = i.items.len();
                let head_warnings = i.global_warnings.clone();
                view! {
                    <section class="space-y-3 rounded-lg border border-border p-4">
                        <div>
                            <h2 class="text-base font-medium">{i.collection.clone()}</h2>
                            <p class="text-xs text-muted-foreground">
                                {total} " Items · " {i.file_count} " Dateien (" {i.shared_file_count} " geteilt) · Prüfsummen in Ordnung"
                                {i.client_version.clone().map(|v| format!(" · Client {v}"))}
                                {i.dbtool_version.clone().map(|v| format!(" · DB-Tool {v}"))}
                                {i.created.clone().map(|c| format!(" · erstellt {}", de_datetime(&c)))}
                            </p>
                            {i.proto_source.clone().map(|p| view! { <p class="text-xs text-muted-foreground">"Quelle der Werte: " {p}</p> })}
                        </div>
                        {(!i.has_sql).then(|| view! {
                            <p class="text-sm text-amber-600">"Das Paket enthält kein Server-SQL. Es kann nur der Client eingespielt werden."</p>
                        })}
                        {(!head_warnings.is_empty()).then(|| view! {
                            <ul class="space-y-0.5 text-xs text-muted-foreground">
                                {head_warnings.into_iter().map(|w| view! { <li>"ℹ " {w.message}</li> }).collect_view()}
                            </ul>
                        })}

                        <div class="flex flex-wrap items-center gap-2 text-sm">
                            <Button variant=ButtonVariant::Outline size=ButtonSize::Sm on_click=move |_| {
                                inspect.with_untracked(|i| if let Some(i) = i { selected.set(i.items.iter().map(|x| x.vnum).collect()) })
                            }>"Alle wählen"</Button>
                            <Button variant=ButtonVariant::Outline size=ButtonSize::Sm on_click=move |_| selected.set(HashSet::new())>
                                "Keine"
                            </Button>
                            <span class="text-muted-foreground">{move || selected.with(|s| s.len())} " von " {total} " gewählt"</span>
                        </div>

                        <div class="max-h-[28rem] space-y-1 overflow-y-auto rounded-md border border-border p-2">
                            {i.items.iter().cloned().map(|it| {
                                let vnum = it.vnum;
                                let missing_name = it.name.is_none();
                                view! {
                                    <label class="flex items-start gap-3 rounded-md p-1.5 text-sm hover:bg-muted/50">
                                        <input
                                            type="checkbox"
                                            class="mt-3"
                                            prop:checked=move || selected.with(|s| s.contains(&vnum))
                                            on:change=move |e| {
                                                let c = event_target_checked(&e);
                                                selected.update(|s| { if c { s.insert(vnum); } else { s.remove(&vnum); } });
                                            }
                                        />
                                        <div class="flex size-10 shrink-0 items-center justify-center rounded border border-border bg-muted/40">
                                            {match it.icon_data_url.clone() {
                                                Some(u) => view! { <img src=u class="max-h-10 max-w-10 [image-rendering:pixelated]"/> }.into_any(),
                                                None => view! { <Icon name="Package" class="size-4 text-muted-foreground"/> }.into_any(),
                                            }}
                                        </div>
                                        <div class="min-w-0 flex-1">
                                            <p class="font-medium">
                                                {it.name.clone().unwrap_or_else(|| "(ohne Namen)".into())}
                                                " " <span class="text-xs font-normal text-muted-foreground">"#" {vnum} " · " {it.item_type.clone()}</span>
                                            </p>
                                            <p class="text-xs text-muted-foreground">
                                                {it.files.iter().map(|f| format!(
                                                    "{}{}{}",
                                                    role_label(&f.role),
                                                    if f.guessed { " (vermutet)" } else { "" },
                                                    if f.used_by > 1 { format!(" ·geteilt mit {}", f.used_by - 1) } else { String::new() }
                                                )).collect::<Vec<_>>().join(", ")}
                                            </p>
                                            {missing_name.then(|| view! { <p class="text-xs text-amber-600">"Kein Name im Paket."</p> })}
                                            {(!it.has_proto).then(|| view! { <p class="text-xs text-amber-600">"Keine item_proto-Zeile im Paket: nur Client-Einträge."</p> })}
                                            {it.warnings.iter().map(|w| view! { <p class="text-xs text-amber-600">"⚠ " {w.message.clone()}</p> }).collect_view()}
                                        </div>
                                    </label>
                                }
                            }).collect_view()}
                        </div>

                        {move || {
                            let g = guessed_files.get();
                            (!g.is_empty()).then(|| view! {
                                <div class="space-y-1 rounded-md border border-border p-3">
                                    <p class="text-sm font-medium">"Vermutete Dateien"</p>
                                    <p class="text-xs text-muted-foreground">
                                        "Diese Dateien sind nur nach Namen zugeordnet und werden nicht ungefragt übernommen. Anhaken, was mit soll."
                                    </p>
                                    {g.into_iter().map(|(path, (role, size, used))| {
                                        let p1 = path.clone();
                                        let p2 = path.clone();
                                        view! {
                                            <label class="flex items-center gap-2 text-xs">
                                                <input
                                                    type="checkbox"
                                                    prop:checked=move || guessed_ok.with(|g| g.contains(&p1))
                                                    on:change=move |e| {
                                                        let c = event_target_checked(&e);
                                                        let p = p2.clone();
                                                        guessed_ok.update(|g| { if c { g.insert(p); } else { g.remove(&p); } });
                                                    }
                                                />
                                                <span>{role_label(&role)}</span>
                                                <code class="break-all">{path.trim_start_matches("client/").to_string()}</code>
                                                <span class="text-muted-foreground">{bytes(size)} {(used > 1).then(|| format!(" · {used} Items"))}</span>
                                            </label>
                                        }
                                    }).collect_view()}
                                </div>
                            })
                        }}

                        {move || {
                            let f = flagged.get();
                            (!f.is_empty()).then(|| {
                                let assumptions = inspect.with(|i| i.as_ref().map(|i| i.assumptions.clone()).unwrap_or_default());
                                view! {
                                    <div class="space-y-2 rounded-md border border-amber-500/50 bg-amber-500/10 p-3 text-sm">
                                        <p class="font-medium">"Angenommene AntiFlags bitte bestätigen"</p>
                                        <p class="text-xs">
                                            "Diese Zuordnungen sind aus den Daten nicht belegt, sondern folgen dem Standard-Layout von Metin2. "
                                            "Sie bestimmen, ob Items z. B. getauscht, verkauft oder ins Lager gelegt werden dürfen."
                                        </p>
                                        <ul class="list-disc pl-5 text-xs">
                                            {assumptions.into_iter().filter(|a| f.contains(&a.name)).map(|a| view! {
                                                <li><code>{a.name}</code> " (Bit " {a.bit} ") - " {a.basis}</li>
                                            }).collect_view()}
                                        </ul>
                                        <label class="flex items-center gap-2">
                                            <input
                                                type="checkbox"
                                                prop:checked=move || confirm_flags.get()
                                                on:change=move |e| confirm_flags.set(event_target_checked(&e))
                                            />
                                            "Ich bestätige diese Annahmen für die gewählten Items."
                                        </label>
                                    </div>
                                }
                            })
                        }}

                        <div class="space-y-2 rounded-md border border-border p-3 text-sm">
                            <p class="font-medium">"Einspielen"</p>
                            <p class="text-xs text-muted-foreground">
                                "Client: Dateien kommen in den eingestellten Client-Ordner (Pfade wie im Paket), item_list.txt und item_names.txt werden um die Zeilen ergänzt."
                            </p>
                            <div class="flex flex-wrap gap-x-6 gap-y-1">
                                <label class="flex items-center gap-2">
                                    <input type="checkbox" prop:checked=move || write_db.get() on:change=move |e| {
                                        let c = event_target_checked(&e);
                                        write_db.set(c);
                                        save_setting(KEY_DB, if c { "1".into() } else { "0".into() });
                                    }/>
                                    "Server: SQL direkt in MySQL schreiben"
                                </label>
                                <label class="flex items-center gap-2">
                                    <input type="checkbox" prop:checked=move || write_files.get() on:change=move |e| {
                                        let c = event_target_checked(&e);
                                        write_files.set(c);
                                        save_setting(KEY_FILES, if c { "1".into() } else { "0".into() });
                                    }/>
                                    "Server: SQL-Dateien bereitstellen"
                                </label>
                            </div>
                            {move || write_files.get().then(|| view! {
                                <div class="flex flex-wrap items-center gap-2 text-xs">
                                    <span class="text-muted-foreground">"Ordner:"</span>
                                    <input
                                        class="w-80 rounded-md border border-border bg-background px-2 py-1"
                                        placeholder="leer = Ordner der ZIP-Datei"
                                        prop:value=move || out_dir.get()
                                        on:input=move |e| out_dir.set(event_target_value(&e))
                                        on:change=move |e| save_setting(KEY_OUT, event_target_value(&e))
                                    />
                                    <Button variant=ButtonVariant::Outline size=ButtonSize::Xs on_click=move |_| choose_out_dir()>"Wählen…"</Button>
                                </div>
                            })}
                            <label class="flex flex-wrap items-center gap-2">
                                "Vorhandene Vnum:"
                                <select
                                    class="rounded-md border border-border bg-background px-2 py-1 text-sm"
                                    prop:value=move || conflict_pref.get()
                                    on:change=move |e| {
                                        let v = event_target_value(&e);
                                        conflict_pref.set(v.clone());
                                        save_setting(KEY_CONFLICT, v);
                                    }
                                >
                                    <option value="ask">"pro Import fragen"</option>
                                    <option value="overwrite">"überschreiben"</option>
                                    <option value="keep">"behalten"</option>
                                </select>
                            </label>
                            {move || (!write_db.get() && !write_files.get()).then(|| view! {
                                <p class="text-xs text-amber-600">"Bitte mindestens eine Server-Möglichkeit wählen."</p>
                            })}
                        </div>

                        <div class="flex justify-end">
                            <Button
                                disabled=Signal::derive(move || !can_import.get())
                                on_click=move |_| start_check()
                            >
                                <Icon name="Upload" class="size-4"/>
                                {move || if checking.get() { "Prüfe…" } else if running.get() { "Spiele ein…" } else { "Einspielen…" }}
                            </Button>
                        </div>
                    </section>
                }
            })}

            {move || report.get().map(|r| {
                let id = r.backup_id.clone();
                view! {
                    <section class="space-y-2 rounded-lg border border-border p-4 text-sm">
                        <h2 class="text-base font-medium">"Bericht"</h2>
                        <ul class="list-disc space-y-0.5 pl-5">
                            <li>{r.items_selected} " Item(s) verarbeitet"</li>
                            <li>
                                "Client-Dateien: " {r.files_created} " neu, " {r.files_replaced} " ersetzt, " {r.files_unchanged} " unverändert"
                                {(!r.files_kept.is_empty()).then(|| format!(", {} behalten (vorhanden)", r.files_kept.len()))}
                            </li>
                            <li>
                                "item_list.txt: " {r.list_added} " ergänzt, " {r.list_replaced} " ersetzt, " {r.list_kept} " unverändert/behalten"
                            </li>
                            {r.db_written.clone().map(|d| view! {
                                <li>"MySQL: " {d.inserted} " neu, " {d.replaced} " überschrieben, " {d.kept} " behalten"</li>
                            })}
                            {(!r.server_files.is_empty()).then(|| view! {
                                <li>
                                    "SQL-/Server-Dateien: "
                                    {r.server_files.iter().map(|f| view! { <code class="mr-2 break-all text-xs">{f.clone()}</code> }).collect_view()}
                                </li>
                            })}
                        </ul>
                        {r.notes.iter().map(|n| view! { <p class="text-xs text-amber-600">"⚠ " {n.clone()}</p> }).collect_view()}
                        {(!r.files_kept.is_empty()).then(|| view! {
                            <p class="text-xs text-muted-foreground">"Behalten (schon vorhanden): " {r.files_kept.join(", ")}</p>
                        })}
                        <p class="text-xs text-muted-foreground">
                            "Hinweis: Die Client-Datei item_proto (Binärdatei) wird hier nicht erzeugt. Bitte wie gewohnt neu erzeugen und packen."
                        </p>
                        <p class="text-xs text-muted-foreground">"Backup: " <code class="break-all">{r.backup_dir.clone()}</code></p>
                        <Button variant=ButtonVariant::Destructive size=ButtonSize::Sm on_click=move |_| undo_confirm.set(Some(id.clone()))>
                            <Icon name="Undo2" class="size-3.5"/>
                            "Rückgängig machen"
                        </Button>
                    </section>
                }
            })}

            {move || {
                let h = history.get();
                (!h.is_empty()).then(|| view! {
                    <section class="space-y-2 rounded-lg border border-border p-4">
                        <h2 class="text-sm font-medium text-muted-foreground">"Eingespielte Pakete (Paket-Import)"</h2>
                        {h.into_iter().map(|r| {
                            let id = r.id.clone();
                            let undone = r.undone;
                            view! {
                                <div class="flex flex-wrap items-center justify-between gap-3 rounded-md border border-border p-2 text-sm">
                                    <div>
                                        <p class="font-medium">{r.collection.clone()} " " <span class="text-xs font-normal text-muted-foreground">"(" {r.package_file.clone()} ")"</span></p>
                                        <p class="text-xs text-muted-foreground">
                                            {de_datetime(&r.created_at)} " · " {r.item_count} " Item(s)"
                                            {r.db_applied.then_some(" · MySQL geschrieben")}
                                            {undone.then_some(" · rückgängig gemacht")}
                                        </p>
                                    </div>
                                    {(!undone).then(|| view! {
                                        <Button
                                            variant=ButtonVariant::Destructive
                                            size=ButtonSize::Sm
                                            disabled=Signal::derive(move || undoing.get())
                                            on_click=move |_| undo_confirm.set(Some(id.clone()))
                                        >
                                            <Icon name="Undo2" class="size-3.5"/>
                                            "Rückgängig machen"
                                        </Button>
                                    })}
                                </div>
                            }
                        }).collect_view()}
                    </section>
                })
            }}

            // Bestätigung vor dem Einspielen
            {move || preflight.get().map(|p| {
                let sel = selected.with(|s| s.len());
                let db_list = p.existing_in_db.clone().unwrap_or_default();
                let conflicts = !db_list.is_empty() || !p.existing_in_client_list.is_empty() || !p.differing_files.is_empty();
                let ask = conflict_pref.get() == "ask";
                let blocked = p.item_list_files == 0;
                view! {
                    <div class="fixed inset-0 z-50 flex items-center justify-center bg-black/50">
                        <div class="max-h-[85vh] w-[34rem] space-y-3 overflow-y-auto rounded-lg border border-border bg-card p-4">
                            <p class="text-sm font-medium">{sel} " Item(s) jetzt einspielen?"</p>
                            <ul class="list-disc space-y-1 pl-5 text-sm text-muted-foreground">
                                <li>"Vorher wird alles gesichert (Dateien, Textzeilen" {write_db.get().then_some(", item_proto-Zeilen")} ")"</li>
                                <li>"item_list.txt in " {p.item_list_files} " Sprache(n), item_names.txt in " {p.item_names_files}</li>
                                {write_db.get().then(|| view! { <li>"Server: direkt in MySQL (player.item_proto)"</li> })}
                                {write_files.get().then(|| view! { <li>"Server: SQL-Dateien werden bereitgestellt"</li> })}
                                <li>{p.identical_files} " Client-Datei(en) sind schon identisch vorhanden"</li>
                            </ul>
                            {(write_db.get() && p.existing_in_db.is_none()).then(|| view! {
                                <p class="text-sm text-destructive">"Keine MySQL-Verbindung. Bitte verbinden oder die MySQL-Option abwählen."</p>
                            })}
                            {p.notes.iter().map(|n| view! { <p class="text-xs text-amber-600">"⚠ " {n.clone()}</p> }).collect_view()}
                            {conflicts.then(|| view! {
                                <div class="space-y-1 rounded-md border border-amber-500/50 bg-amber-500/10 p-2 text-xs">
                                    <p class="font-medium">"Schon vorhanden:"</p>
                                    {(!db_list.is_empty()).then(|| view! { <p>"In der Datenbank: " {db_list.len()} " Vnum (" {join_vnums(&db_list)} ")"</p> })}
                                    {(!p.existing_in_client_list.is_empty()).then(|| view! {
                                        <p>"In item_list.txt: " {p.existing_in_client_list.len()} " Vnum (" {join_vnums(&p.existing_in_client_list)} ")"</p>
                                    })}
                                    {(!p.differing_files.is_empty()).then(|| view! {
                                        <p>"Dateien mit anderem Inhalt: " {p.differing_files.len()} " (" {p.differing_files.iter().take(4).cloned().collect::<Vec<_>>().join(", ")} ")"</p>
                                    })}
                                </div>
                            })}
                            {(conflicts && ask).then(|| view! {
                                <div class="space-y-1 text-sm">
                                    <label class="flex items-center gap-2">
                                        <input type="radio" name="m2pkg-conflict" prop:checked=move || dialog_conflict.get() == "keep"
                                            on:change=move |_| dialog_conflict.set("keep".into())/>
                                        "Vorhandenes behalten (nur Neues ergänzen)"
                                    </label>
                                    <label class="flex items-center gap-2">
                                        <input type="radio" name="m2pkg-conflict" prop:checked=move || dialog_conflict.get() == "overwrite"
                                            on:change=move |_| dialog_conflict.set("overwrite".into())/>
                                        "Vorhandenes überschreiben (Sicherung wird angelegt)"
                                    </label>
                                </div>
                            })}
                            {(conflicts && !ask).then(|| view! {
                                <p class="text-xs text-muted-foreground">
                                    "Laut Einstellung wird Vorhandenes " {if conflict_pref.get() == "overwrite" { "überschrieben" } else { "behalten" }} "."
                                </p>
                            })}
                            <div class="flex justify-end gap-2">
                                <Button variant=ButtonVariant::Outline on_click=move |_| preflight.set(None)>"Abbrechen"</Button>
                                <Button
                                    disabled=Signal::derive(move || blocked || (write_db.get() && preflight.with(|p| p.as_ref().is_some_and(|p| p.existing_in_db.is_none()))))
                                    on_click=move |_| run_import()
                                >
                                    "Jetzt einspielen"
                                </Button>
                            </div>
                        </div>
                    </div>
                }
            })}

            // Bestätigung vor dem Rückgängigmachen
            {move || undo_confirm.get().map(|id| {
                let id2 = id.clone();
                view! {
                    <div class="fixed inset-0 z-50 flex items-center justify-center bg-black/50">
                        <div class="w-[28rem] space-y-3 rounded-lg border border-border bg-card p-4">
                            <p class="text-sm font-medium">"Diesen Import rückgängig machen?"</p>
                            <ul class="list-disc space-y-1 pl-5 text-sm text-muted-foreground">
                                <li>"Neu angelegte Client-Dateien werden gelöscht, ersetzte aus dem Backup zurückgeholt"</li>
                                <li>"Die ergänzten Zeilen in item_list.txt/item_names.txt werden entfernt, ersetzte wiederhergestellt"</li>
                                <li>"Falls in MySQL geschrieben wurde: neue Zeilen werden gelöscht, überschriebene wiederhergestellt (MySQL-Verbindung nötig)"</li>
                                <li>"Bereitgestellte SQL-Dateien bleiben liegen"</li>
                            </ul>
                            <div class="flex justify-end gap-2">
                                <Button variant=ButtonVariant::Outline on_click=move |_| undo_confirm.set(None)>"Abbrechen"</Button>
                                <Button variant=ButtonVariant::Destructive on_click=move |_| run_undo(id2.clone())>"Rückgängig machen"</Button>
                            </div>
                        </div>
                    </div>
                }
            })}
        </div>
    }
}
