//! Schema-unabhängiger Zeilen-Editor auf Basis derselben Introspektion wie
//! der Datenbank-Explorer (`get_table_columns` / `get_table_row` /
//! `update_table_row`). Für Tabellen ohne eigenes, geprüftes Schema
//! (mob_proto, account, player, gmlist, guild): jede echte Spalte mit
//! passendem Eingabefeld, Primärschlüssel automatisch erkannt.

use crate::components::{Button, ButtonSize, ButtonVariant, Icon, Skeleton};
use crate::lib::{log_activity, use_save_shortcut};
use crate::store::navigation::{report_section_dirty, ModuleId, Section};
use crate::tauri::invoke;
use leptos::prelude::*;
use leptos::task::spawn_local;
use serde::Deserialize;
use serde_json::json;
use std::collections::BTreeMap;

#[derive(Clone, Debug, Deserialize, PartialEq)]
pub struct ColumnInfo {
    pub name: String,
    pub data_type: String,
    pub is_nullable: bool,
    pub is_primary_key: bool,
}

#[derive(Clone, Debug, Deserialize)]
pub struct TableRows {
    pub columns: Vec<String>,
    pub rows: Vec<Vec<Option<String>>>,
    pub total_rows: i64,
}

type Values = BTreeMap<String, Option<String>>;

const NUMERIC_TYPE_PREFIXES: [&str; 8] = ["int", "tinyint", "smallint", "mediumint", "bigint", "decimal", "float", "double"];

pub fn is_numeric_type(data_type: &str) -> bool {
    let d = data_type.to_lowercase();
    NUMERIC_TYPE_PREFIXES.iter().any(|p| d.starts_with(p))
}

fn changed(values: &Values, original: &Values) -> Vec<String> {
    values.iter().filter(|(k, v)| original.get(*k) != Some(*v)).map(|(k, _)| k.clone()).collect()
}

#[component]
pub fn GenericRowEditor(
    #[prop(into)] database: String,
    #[prop(into)] table: String,
    #[prop(into)] pk_value: String,
    #[prop(into)] on_close: Callback<()>,
    #[prop(into, optional)] on_saved: Option<Callback<()>>,
    /// Wenn gesetzt, wird ein erfolgreiches Speichern ins Änderungsprotokoll geschrieben.
    #[prop(into, optional)] activity_module: Option<ModuleId>,
) -> impl IntoView {
    let columns = RwSignal::new(None::<Vec<ColumnInfo>>);
    let load_error = RwSignal::new(None::<String>);
    let values = RwSignal::new(Values::new());
    let original = RwSignal::new(Values::new());
    let last_saved = RwSignal::new(None::<Values>);
    let saving = RwSignal::new(false);
    let save_error = RwSignal::new(None::<String>);
    let save_ok = RwSignal::new(false);
    let confirm_open = RwSignal::new(false);

    let pk_column = Memo::new(move |_| {
        columns.with(|c| c.as_ref().map(|c| c.iter().filter(|c| c.is_primary_key).map(|c| c.name.clone()).collect::<Vec<_>>()).unwrap_or_default())
    });
    let changed_columns = Memo::new(move |_| values.with(|v| original.with(|o| changed(v, o))));

    {
        let (database, table, pk_value) = (database.clone(), table.clone(), pk_value.clone());
        spawn_local(async move {
            let res: Result<(Vec<ColumnInfo>, Values), String> = async {
                let cols = invoke::<Vec<ColumnInfo>>("get_table_columns", &json!({ "database": database, "table": table })).await?;
                let pk: Vec<&ColumnInfo> = cols.iter().filter(|c| c.is_primary_key).collect();
                if pk.len() != 1 {
                    return Err(if pk.is_empty() {
                        "Diese Tabelle hat keinen Primärschlüssel - generischer Editor nicht möglich.".to_string()
                    } else {
                        "Diese Tabelle hat einen zusammengesetzten Primärschlüssel - generischer Editor unterstützt das nicht.".to_string()
                    });
                }
                let row = invoke::<Option<TableRows>>(
                    "get_table_row",
                    &json!({ "database": database, "table": table, "pkColumn": pk[0].name, "pkValue": pk_value }),
                )
                .await?
                .ok_or_else(|| "Zeile nicht gefunden.".to_string())?;
                let first = row.rows.first().cloned().unwrap_or_default();
                let vals = row.columns.iter().enumerate().map(|(i, n)| (n.clone(), first.get(i).cloned().flatten())).collect();
                Ok((cols, vals))
            }
            .await;
            match res {
                Ok((cols, vals)) => {
                    columns.set(Some(cols));
                    values.set(vals.clone());
                    original.set(vals);
                }
                Err(e) => load_error.set(Some(e)),
            }
        });
    }

    if let Some(m) = activity_module {
        let section = m.host();
        Effect::new(move |_| report_section_dirty(section, !changed_columns.with(|c| c.is_empty())));
        on_cleanup(move || report_section_dirty(section, false));
    }

    use_save_shortcut(
        activity_module.map(|m| m.host()).unwrap_or(Section::Dashboard),
        move || activity_module.is_some() && !changed_columns.with_untracked(|c| c.is_empty()) && !saving.get_untracked(),
        move || confirm_open.set(true),
    );

    let save = {
        let (database, table, pk_value) = (database.clone(), table.clone(), pk_value.clone());
        move || {
            let pk = pk_column.get_untracked();
            let changed_cols = changed_columns.get_untracked();
            if pk.len() != 1 || changed_cols.is_empty() {
                return;
            }
            confirm_open.set(false);
            let vals = values.get_untracked();
            let changes: Vec<(String, Option<String>)> = changed_cols.iter().map(|n| (n.clone(), vals.get(n).cloned().flatten())).collect();
            let (database, table, pk_value) = (database.clone(), table.clone(), pk_value.clone());
            saving.set(true);
            save_error.set(None);
            spawn_local(async move {
                let res = invoke::<serde_json::Value>(
                    "update_table_row",
                    &json!({ "database": database, "table": table, "pkColumn": pk[0], "pkValue": pk_value, "changes": changes }),
                )
                .await;
                match res {
                    Ok(_) => {
                        last_saved.set(Some(original.get_untracked()));
                        original.set(vals);
                        save_ok.set(true);
                        if let Some(m) = activity_module {
                            log_activity(
                                m,
                                "update",
                                &format!("{table} ({pk_value}) aktualisiert: {}", changed_cols.join(", ")),
                                Some(&table),
                                Some(&pk_value),
                            );
                        }
                        if let Some(cb) = on_saved {
                            cb.run(());
                        }
                    }
                    Err(e) => save_error.set(Some(e)),
                }
                saving.set(false);
            });
        }
    };
    let save = StoredValue::new(save);

    let rollback = {
        let (database, table, pk_value) = (database.clone(), table.clone(), pk_value.clone());
        move || {
            let Some(prev) = last_saved.get_untracked() else { return };
            let pk = pk_column.get_untracked();
            if pk.len() != 1 {
                return;
            }
            let orig = original.get_untracked();
            let changes: Vec<(String, Option<String>)> =
                prev.iter().filter(|(k, v)| orig.get(*k) != Some(*v)).map(|(k, v)| (k.clone(), v.clone())).collect();
            if changes.is_empty() {
                return;
            }
            let (database, table, pk_value) = (database.clone(), table.clone(), pk_value.clone());
            saving.set(true);
            spawn_local(async move {
                let res = invoke::<serde_json::Value>(
                    "update_table_row",
                    &json!({ "database": database, "table": table, "pkColumn": pk[0], "pkValue": pk_value, "changes": changes }),
                )
                .await;
                match res {
                    Ok(_) => {
                        values.set(prev.clone());
                        original.set(prev);
                        last_saved.set(None);
                        save_ok.set(false);
                    }
                    Err(e) => save_error.set(Some(e)),
                }
                saving.set(false);
            });
        }
    };
    let rollback = StoredValue::new(rollback);

    let title = format!("{database}.{table} — {pk_value}");
    let target = format!("{database}.{table}");

    view! {
        <div class="fixed inset-0 z-40 flex items-center justify-center bg-black/50">
            <div class="flex max-h-[85vh] w-[42rem] flex-col gap-3 rounded-lg border border-border bg-card p-4">
                <div class="flex items-center justify-between">
                    <p class="text-sm font-medium">{title}</p>
                    <Button variant=ButtonVariant::Ghost size=ButtonSize::Sm on_click=move |_| on_close.run(())>
                        "Schließen"
                    </Button>
                </div>

                {move || load_error.get().map(|e| view! {
                    <p class="flex items-start gap-2 text-sm text-destructive">
                        <Icon name="AlertTriangle" class="mt-0.5 size-4 shrink-0"/>
                        <span>{e}</span>
                    </p>
                })}
                <Show when=move || columns.with(|c| c.is_none()) && load_error.with(|e| e.is_none())>
                    <div class="grid grid-cols-2 gap-3">
                        {(0..8).map(|_| view! {
                            <div class="space-y-1">
                                <Skeleton class="h-3 w-16"/>
                                <Skeleton class="h-7 w-full"/>
                            </div>
                        }).collect_view()}
                    </div>
                </Show>

                {move || columns.get().map(|cols| view! {
                    <p class="text-xs text-muted-foreground">
                        "Generischer Editor - jede Spalte roh, ohne bekannte Spaltensemantik. Text-Spalten mit binärer Kollation (z.B. "
                        <code>"locale_name"</code>
                        ") werden Windows-1252-kodiert gespeichert, passend zur restigen App."
                    </p>
                    <div class="min-h-0 flex-1 space-y-2 overflow-y-auto pr-1">
                        {cols.into_iter().map(|col| {
                            let name = col.name.clone();
                            let name2 = col.name.clone();
                            view! {
                                <label class="flex items-center gap-2 text-sm">
                                    <span
                                        class=if col.is_primary_key { "w-40 shrink-0 truncate font-mono text-xs font-semibold" } else { "w-40 shrink-0 truncate font-mono text-xs text-muted-foreground" }
                                        title=col.data_type.clone()
                                    >
                                        {col.name.clone()}
                                        {col.is_primary_key.then_some(" (PK)")}
                                    </span>
                                    <input
                                        type=if is_numeric_type(&col.data_type) { "number" } else { "text" }
                                        prop:value=move || values.with(|v| v.get(&name).cloned().flatten().unwrap_or_default())
                                        disabled=col.is_primary_key
                                        on:input=move |e| {
                                            let val = event_target_value(&e);
                                            values.update(|v| { v.insert(name2.clone(), Some(val)); });
                                            save_ok.set(false);
                                        }
                                        class="w-full rounded-md border border-border bg-background px-2 py-1 text-sm disabled:opacity-60"
                                    />
                                </label>
                            }
                        }).collect_view()}
                    </div>

                    {move || save_error.get().map(|e| view! { <p class="text-sm text-destructive">{e}</p> })}
                    <Show when=move || save_ok.get()>
                        <p class="flex items-center gap-1 text-sm text-green-600">
                            <Icon name="CheckCircle2" class="size-4"/>
                            " Gespeichert."
                        </p>
                    </Show>

                    <div class="flex items-center justify-between">
                        <span class="text-xs text-muted-foreground">
                            {move || {
                                let n = changed_columns.with(|c| c.len());
                                if n > 0 { format!("{n} Feld(er) geändert") } else { "Keine Änderungen".to_string() }
                            }}
                        </span>
                        <div class="flex gap-2">
                            <Show when=move || last_saved.with(|l| l.is_some())>
                                <Button variant=ButtonVariant::Outline size=ButtonSize::Sm on_click=move |_| rollback.with_value(|f| f()) disabled=saving>
                                    <Icon name="Undo2" class="size-3.5"/>
                                    "Letztes Speichern rückgängig"
                                </Button>
                            </Show>
                            <Button
                                size=ButtonSize::Sm
                                on_click=move |_| confirm_open.set(true)
                                disabled=Signal::derive(move || changed_columns.with(|c| c.is_empty()) || saving.get())
                            >
                                {move || if saving.get() { "Speichere…" } else { "Speichern" }}
                            </Button>
                        </div>
                    </div>
                })}

                <Show when=move || confirm_open.get()>
                    <div class="fixed inset-0 z-50 flex items-center justify-center bg-black/50">
                        <div class="w-96 space-y-3 rounded-lg border border-border bg-card p-4">
                            <p class="flex items-start gap-2 text-sm">
                                <Icon name="AlertTriangle" class="mt-0.5 size-4 shrink-0 text-destructive"/>
                                <span>
                                    {move || changed_columns.with(|c| c.len())}
                                    " Spalte(n) in "
                                    <code>{target.clone()}</code>
                                    " werden direkt in der Live-Datenbank überschrieben, ohne Backup (DB-Zeilen, keine Datei)."
                                </span>
                            </p>
                            <div class="flex justify-end gap-2">
                                <Button variant=ButtonVariant::Outline on_click=move |_| confirm_open.set(false)>"Abbrechen"</Button>
                                <Button on_click=move |_| save.with_value(|f| f())>"Speichern"</Button>
                            </div>
                        </div>
                    </div>
                </Show>
            </div>
        </div>
    }
}
