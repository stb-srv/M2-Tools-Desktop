//! Gilden-Verwaltung (Port von `src/features/guild-manager/GuildManager.tsx`).

use crate::components::{Button, ButtonSize, ButtonVariant, Icon, PageHeader, Skeleton};
use crate::features::shared::generic_row_editor::TableRows;
use crate::features::shared::GenericRowEditor;
use crate::util::log_activity;
use crate::store::navigation::{ModuleId, Section};
use crate::tauri::{invoke, invoke_unit};
use leptos::prelude::*;
use leptos::task::spawn_local;
use serde_json::{json, Value};
use std::collections::HashSet;

const MODULE: ModuleId = ModuleId::Section(Section::GuildManager);

// Reale Tabellennamen aus source/db/src/GuildManager.cpp verifiziert (guild:
// id/name/ladder_point/win/draw/loss/gold/level/master, guild_war_reservation/
// guild_war_bet für Kriege) - siehe src-tauri/src/db/guild.rs für den
// verifizierten Auflösen-Kaskade-Code. Welche Datenbank diese Tabellen
// enthält wurde NICHT live geprüft (Annahme: "player", wie player.player/
// item_proto/shop) - deshalb hier bewusst editierbar statt hart codiert.
const DEFAULT_DATABASE: &str = "player";
const GUILD_TABLE: &str = "guild";
const WAR_TABLE: &str = "guild_war_reservation";

type Row = Vec<Option<String>>;

fn find_column(rows: &TableRows, name: &str) -> Option<usize> {
    rows.columns.iter().position(|c| c == name)
}

fn cell(rows: &TableRows, row: &Row, name: &str) -> Option<String> {
    find_column(rows, name).and_then(|i| row.get(i).cloned().flatten())
}

/// Führt die Treffer beider Suchen (guild1/guild2) zusammen und entfernt
/// Dubletten anhand der `id`-Spalte (ohne id-Spalte: anhand der ganzen Zeile).
pub fn merge_wars(by_guild1: TableRows, by_guild2: TableRows) -> TableRows {
    let id_col = find_column(&by_guild1, "id");
    let mut seen = HashSet::new();
    let merged: Vec<Row> = by_guild1
        .rows
        .into_iter()
        .chain(by_guild2.rows)
        .filter(|row| {
            let key = match id_col {
                Some(i) => row.get(i).cloned().flatten().unwrap_or_default(),
                None => serde_json::to_string(row).unwrap_or_default(),
            };
            seen.insert(key)
        })
        .collect();
    let total = merged.len() as i64;
    TableRows { columns: by_guild1.columns, rows: merged, total_rows: total }
}

/// `Number(s)` als Kommando-Argument (NaN -> null wie bei `JSON.stringify`).
fn num_arg(s: &str) -> Value {
    let t = s.trim();
    if t.is_empty() {
        return Value::from(0);
    }
    match t.parse::<f64>() {
        Ok(n) if n.is_finite() && !t.chars().any(|c| c.is_ascii_alphabetic() && c != 'e' && c != 'E') => {
            if n.fract() == 0.0 && n.abs() < 9.007_199_254_740_992e15 {
                Value::from(n as i64)
            } else {
                Value::from(n)
            }
        }
        _ => Value::Null,
    }
}

fn cell_view(c: &Option<String>) -> AnyView {
    match c {
        Some(v) => v.clone().into_any(),
        None => view! { <span class="text-muted-foreground">"NULL"</span> }.into_any(),
    }
}

#[derive(Clone, Debug)]
struct WarsFor {
    #[allow(dead_code)]
    id: String,
    name: String,
    rows: TableRows,
}

#[component]
pub fn GuildManager() -> impl IntoView {
    let database = RwSignal::new(DEFAULT_DATABASE.to_string());
    let query = RwSignal::new(String::new());
    let result = RwSignal::new(None::<TableRows>);
    let searching = RwSignal::new(false);
    let search_error = RwSignal::new(None::<String>);

    let editing_id = RwSignal::new(None::<String>);
    let wars_for = RwSignal::new(None::<WarsFor>);
    let wars_loading = RwSignal::new(false);
    let wars_error = RwSignal::new(None::<String>);
    let disband_target = RwSignal::new(None::<(String, String)>);
    let disbanding = RwSignal::new(false);
    let disband_error = RwSignal::new(None::<String>);

    let run_search = move || {
        let (db, q) = (database.get_untracked(), query.get_untracked().trim().to_string());
        searching.set(true);
        search_error.set(None);
        spawn_local(async move {
            match invoke::<TableRows>("search_table_rows", &json!({ "database": db, "table": GUILD_TABLE, "column": "name", "query": q })).await {
                Ok(r) => result.set(Some(r)),
                Err(e) => search_error.set(Some(e)),
            }
            searching.set(false);
        });
    };

    let show_wars = move |id: String, name: String| {
        wars_error.set(None);
        wars_loading.set(true);
        wars_for.set(None);
        let db = database.get_untracked();
        spawn_local(async move {
            let res: Result<TableRows, String> = async {
                let g1 = invoke::<TableRows>("search_table_rows", &json!({ "database": db, "table": WAR_TABLE, "column": "guild1", "query": id })).await?;
                let g2 = invoke::<TableRows>("search_table_rows", &json!({ "database": db, "table": WAR_TABLE, "column": "guild2", "query": id })).await?;
                Ok(merge_wars(g1, g2))
            }
            .await;
            match res {
                Ok(rows) => wars_for.set(Some(WarsFor { id, name, rows })),
                Err(e) => wars_error.set(Some(e)),
            }
            wars_loading.set(false);
        });
    };

    let confirm_disband = move || {
        let Some((id, name)) = disband_target.get_untracked() else { return };
        let db = database.get_untracked();
        disbanding.set(true);
        disband_error.set(None);
        spawn_local(async move {
            match invoke_unit("disband_guild", &json!({ "database": db, "guildId": num_arg(&id) })).await {
                Ok(()) => {
                    log_activity(MODULE, "delete", &format!("Gilde \"{name}\" ({id}) aufgelöst"), Some(GUILD_TABLE), Some(&id));
                    disband_target.set(None);
                    result.update(|prev| {
                        if let Some(prev) = prev {
                            let idx = find_column(prev, "id");
                            prev.rows.retain(|r| idx.and_then(|i| r.get(i).cloned().flatten()) != Some(id.clone()));
                        }
                    });
                }
                Err(e) => disband_error.set(Some(e)),
            }
            disbanding.set(false);
        });
    };

    view! {
        <div class="space-y-4">
            <PageHeader module=Section::GuildManager title="Gilden-Verwaltung".to_string()/>
            <p class="max-w-4xl text-sm text-muted-foreground">
                "Bearbeitet die Gilden-Tabellen direkt in der Datenbank (generisch, wie Mob-Proto-Editor/ Account-Verwaltung - jede Spalte roh, ohne geratene Bedeutung). \"Auflösen\" führt exakt die Kaskade des echten Servers aus ("
                <code>"source/db/src/ClientManagerGuild.cpp::GuildDisband"</code>
                " , verifiziert): löscht die Gilde, ihre Rangstufen, Mitglieder und Kommentare, und setzt für jedes ehemalige Mitglied dieselbe Beitritts-Sperre-Quest-Flag wie der echte Server. Gildenkriege werden dabei bewusst nicht angefasst (der echte Server räumt die beim Auflösen ebenfalls nicht auf)."
            </p>

            <div class="flex items-center gap-2 text-xs text-muted-foreground">
                <span>"Datenbank:"</span>
                <input
                    prop:value=move || database.get()
                    on:input=move |e| database.set(event_target_value(&e))
                    title="Welche Datenbank die guild-Tabellen enthält - nicht live gegen den echten Server geprüft, Standard 'player' wie player.player/item_proto/shop"
                    class="w-28 rounded-md border border-border bg-background px-2 py-1"
                />
            </div>

            <section class="space-y-2 rounded-lg border border-border p-4">
                <h2 class="text-sm font-medium text-muted-foreground">"Gilde suchen"</h2>
                <div class="flex gap-2">
                    <input
                        prop:value=move || query.get()
                        on:input=move |e| query.set(event_target_value(&e))
                        on:keydown=move |e| {
                            if e.key() == "Enter" {
                                run_search()
                            }
                        }
                        placeholder="Gildenname (leer = alle)"
                        class="flex-1 rounded-md border border-border bg-background px-2 py-1 text-sm"
                    />
                    <Button variant=ButtonVariant::Outline on_click=move |_| run_search() disabled=searching>
                        <Icon name="Search" class="size-4"/>
                    </Button>
                </div>

                {move || search_error.get().map(|e| view! {
                    <p class="flex items-start gap-2 text-sm text-destructive">
                        <Icon name="AlertTriangle" class="mt-0.5 size-4 shrink-0"/>
                        <span>{e}</span>
                    </p>
                })}

                {move || result.get().map(|r| {
                    if r.rows.is_empty() {
                        return view! { <p class="text-sm text-muted-foreground">"Keine Treffer."</p> }.into_any();
                    }
                    view! {
                        <div class="overflow-x-auto rounded-md border border-border">
                            <table class="w-full text-xs">
                                <thead class="bg-muted/40">
                                    <tr>
                                        {r.columns.iter().map(|c| view! { <th class="px-2 py-1 text-left font-medium">{c.clone()}</th> }).collect_view()}
                                        <th></th>
                                    </tr>
                                </thead>
                                <tbody>
                                    {r.rows.iter().map(|row| {
                                        let id = cell(&r, row, "id").unwrap_or_default();
                                        let name = cell(&r, row, "name").unwrap_or_else(|| id.clone());
                                        let (id_edit, id_wars, name_wars, id_del, name_del) = (id.clone(), id.clone(), name.clone(), id, name);
                                        view! {
                                            <tr class="border-t border-border">
                                                {row.iter().map(|c| view! { <td class="max-w-32 truncate px-2 py-1">{cell_view(c)}</td> }).collect_view()}
                                                <td class="whitespace-nowrap px-2 py-1 text-right">
                                                    <Button size=ButtonSize::Sm variant=ButtonVariant::Outline on_click=move |_| editing_id.set(Some(id_edit.clone()))>
                                                        "Bearbeiten"
                                                    </Button>
                                                    " "
                                                    <Button size=ButtonSize::Sm variant=ButtonVariant::Outline on_click=move |_| show_wars(id_wars.clone(), name_wars.clone())>
                                                        <Icon name="Swords" class="size-3.5"/>
                                                    </Button>
                                                    " "
                                                    <Button
                                                        size=ButtonSize::Sm
                                                        variant=ButtonVariant::Destructive
                                                        on_click=move |_| disband_target.set(Some((id_del.clone(), name_del.clone())))
                                                    >
                                                        <Icon name="Trash2" class="size-3.5"/>
                                                    </Button>
                                                </td>
                                            </tr>
                                        }
                                    }).collect_view()}
                                </tbody>
                            </table>
                        </div>
                    }
                    .into_any()
                })}
            </section>

            {move || editing_id.get().map(|id| view! {
                <GenericRowEditor
                    database=database.get_untracked()
                    table=GUILD_TABLE
                    pk_value=id
                    on_close=move |_| editing_id.set(None)
                    activity_module=MODULE
                />
            })}

            <Show when=move || wars_loading.get() || wars_for.with(|w| w.is_some()) || wars_error.with(|e| e.is_some())>
                <div class="fixed inset-0 z-40 flex items-center justify-center bg-black/50">
                    <div class="max-h-[70vh] w-[36rem] space-y-3 overflow-y-auto rounded-lg border border-border bg-card p-4">
                        <div class="flex items-center justify-between">
                            <p class="text-sm font-medium">
                                "Gildenkriege"
                                {move || wars_for.with(|w| w.as_ref().map(|w| format!(" — {}", w.name)))}
                            </p>
                            <Button
                                variant=ButtonVariant::Ghost
                                size=ButtonSize::Sm
                                on_click=move |_| {
                                    wars_for.set(None);
                                    wars_error.set(None);
                                }
                            >
                                "Schließen"
                            </Button>
                        </div>
                        <Show when=move || wars_loading.get()>
                            <div class="space-y-1.5">
                                <Skeleton class="h-6 w-full"/>
                                <Skeleton class="h-6 w-full"/>
                                <Skeleton class="h-6 w-2/3"/>
                            </div>
                        </Show>
                        {move || wars_error.get().map(|e| view! {
                            <p class="flex items-start gap-2 text-sm text-destructive">
                                <Icon name="AlertTriangle" class="mt-0.5 size-4 shrink-0"/>
                                <span>{e}</span>
                            </p>
                        })}
                        {move || wars_for.get().map(|w| {
                            if w.rows.rows.is_empty() {
                                return view! { <p class="text-sm text-muted-foreground">"Keine Kriege gefunden."</p> }.into_any();
                            }
                            view! {
                                <table class="w-full text-xs">
                                    <thead class="bg-muted/40">
                                        <tr>
                                            {w.rows.columns.iter().map(|c| view! { <th class="px-2 py-1 text-left font-medium">{c.clone()}</th> }).collect_view()}
                                        </tr>
                                    </thead>
                                    <tbody>
                                        {w.rows.rows.iter().map(|row| view! {
                                            <tr class="border-t border-border">
                                                {row.iter().map(|c| view! { <td class="max-w-32 truncate px-2 py-1">{cell_view(c)}</td> }).collect_view()}
                                            </tr>
                                        }).collect_view()}
                                    </tbody>
                                </table>
                            }
                            .into_any()
                        })}
                    </div>
                </div>
            </Show>

            {move || disband_target.get().map(|(id, name)| view! {
                <div class="fixed inset-0 z-50 flex items-center justify-center bg-black/50">
                    <div class="w-96 space-y-3 rounded-lg border border-border bg-card p-4">
                        <p class="flex items-start gap-2 text-sm">
                            <Icon name="AlertTriangle" class="mt-0.5 size-4 shrink-0 text-destructive"/>
                            <span>
                                "Gilde " <strong>{name}</strong> " (" {id}
                                ") wirklich auflösen? Löscht die Gilde, ihre Rangstufen, Mitglieder und Kommentare unwiderruflich in der Live-Datenbank, ohne Backup (DB-Zeilen, keine Datei)."
                            </span>
                        </p>
                        {move || disband_error.get().map(|e| view! { <p class="text-sm text-destructive">{e}</p> })}
                        <div class="flex justify-end gap-2">
                            <Button variant=ButtonVariant::Outline on_click=move |_| disband_target.set(None) disabled=disbanding>
                                "Abbrechen"
                            </Button>
                            <Button variant=ButtonVariant::Destructive on_click=move |_| confirm_disband() disabled=disbanding>
                                {move || if disbanding.get() { "Löse auf…" } else { "Auflösen" }}
                            </Button>
                        </div>
                    </div>
                </div>
            })}
        </div>
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn rows(columns: &[&str], rows: Vec<Vec<Option<&str>>>) -> TableRows {
        TableRows {
            columns: columns.iter().map(|s| s.to_string()).collect(),
            rows: rows.into_iter().map(|r| r.into_iter().map(|c| c.map(str::to_string)).collect()).collect(),
            total_rows: 0,
        }
    }

    #[test]
    fn merges_wars_without_duplicates() {
        let a = rows(&["id", "guild1", "guild2"], vec![vec![Some("1"), Some("5"), Some("7")], vec![Some("2"), Some("5"), Some("5")]]);
        let b = rows(&["id", "guild1", "guild2"], vec![vec![Some("2"), Some("5"), Some("5")], vec![Some("3"), Some("9"), Some("5")]]);
        let m = merge_wars(a, b);
        assert_eq!(m.rows.len(), 3);
        assert_eq!(m.total_rows, 3);
    }

    #[test]
    fn merges_by_whole_row_without_id_column() {
        let a = rows(&["guild1"], vec![vec![Some("5")]]);
        let b = rows(&["guild1"], vec![vec![Some("5")], vec![None]]);
        assert_eq!(merge_wars(a, b).rows.len(), 2);
    }

    #[test]
    fn number_args() {
        assert_eq!(num_arg("12"), Value::from(12));
        assert_eq!(num_arg(""), Value::from(0));
        assert_eq!(num_arg("abc"), Value::Null);
    }
}
