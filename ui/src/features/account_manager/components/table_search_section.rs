use super::super::shared::{js_error, ColumnInfo, TableRows, TableTarget, MODULE};
use super::PlayerToolsDialog;
use crate::components::{Button, ButtonSize, ButtonVariant, Icon};
use crate::features::shared::GenericRowEditor;
use crate::tauri::invoke;
use leptos::prelude::*;
use leptos::task::spawn_local;
use serde_json::json;

type Row = Vec<Option<String>>;

async fn resolve_pk(target: TableTarget, result: Option<TableRows>, row: Row) -> Result<String, String> {
    let Some(result) = result else { return Err(js_error("Kein Suchergebnis.")) };
    let cols = invoke::<Vec<ColumnInfo>>("get_table_columns", &json!({ "database": target.database, "table": target.table })).await?;
    let pk = cols.iter().find(|c| c.is_primary_key).ok_or_else(|| js_error("Diese Tabelle hat keinen Primärschlüssel."))?;
    let index = result.columns.iter().position(|c| *c == pk.name);
    match index.and_then(|i| row.get(i).cloned()) {
        Some(Some(v)) => Ok(v),
        Some(None) => Err(js_error("Primärschlüssel-Wert ist NULL.")),
        // `row[-1]` ist in JS `undefined` - wird ungeprüft als pkValue weitergereicht.
        None => Ok(String::new()),
    }
}

#[component]
pub fn TableSearchSection(
    #[prop(into)] title: String,
    target: TableTarget,
    #[prop(into)] default_column: String,
    #[prop(into)] placeholder: String,
    /// Zeigt einen "Werkzeuge"-Button (Yang gutschreiben, Position setzen) je
    /// Treffer - nur sinnvoll, wenn `target` player.player ist.
    #[prop(optional)]
    player_tools: bool,
) -> impl IntoView {
    let column = RwSignal::new(default_column);
    let query = RwSignal::new(String::new());
    let result = RwSignal::new(None::<TableRows>);
    let searching = RwSignal::new(false);
    let search_error = RwSignal::new(None::<String>);
    let editing_pk = RwSignal::new(None::<String>);
    let tools_target = RwSignal::new(None::<(String, String)>);

    let run_search = move || {
        let q = query.get_untracked().trim().to_string();
        if q.is_empty() {
            return;
        }
        let col = column.get_untracked();
        searching.set(true);
        search_error.set(None);
        spawn_local(async move {
            match invoke::<TableRows>(
                "search_table_rows",
                &json!({ "database": target.database, "table": target.table, "column": col, "query": q }),
            )
            .await
            {
                Ok(r) => result.set(Some(r)),
                Err(e) => search_error.set(Some(e)),
            }
            searching.set(false);
        });
    };

    let pick_row = move |row: Row| {
        let res = result.get_untracked();
        spawn_local(async move {
            match resolve_pk(target, res, row).await {
                Ok(pk) => editing_pk.set(Some(pk)),
                Err(e) => search_error.set(Some(e)),
            }
        });
    };

    let open_tools = move |row: Row| {
        let Some(res) = result.get_untracked() else { return };
        let label = res
            .columns
            .iter()
            .position(|c| c == "name")
            .map(|i| row.get(i).cloned().flatten().unwrap_or_default())
            .unwrap_or_default();
        spawn_local(async move {
            match resolve_pk(target, Some(res), row).await {
                Ok(pk) => tools_target.set(Some((pk, label))),
                Err(e) => search_error.set(Some(e)),
            }
        });
    };

    view! {
        <section class="space-y-2 rounded-lg border border-border p-4">
            <h2 class="text-sm font-medium text-muted-foreground">{title}</h2>
            <div class="flex gap-2">
                <input
                    prop:value=move || column.get()
                    on:input=move |e| column.set(event_target_value(&e))
                    title="Suchspalte"
                    class="w-32 rounded-md border border-border bg-background px-2 py-1 text-sm"
                />
                <input
                    prop:value=move || query.get()
                    on:input=move |e| query.set(event_target_value(&e))
                    on:keydown=move |e| {
                        if e.key() == "Enter" {
                            run_search()
                        }
                    }
                    placeholder=placeholder
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
                let col_count = r.columns.len();
                view! {
                    <div class="overflow-x-auto rounded-md border border-border">
                        <table class="w-full text-xs">
                            <thead class="bg-muted/40">
                                <tr>
                                    {r.columns.iter().take(6).map(|c| view! {
                                        <th class="px-2 py-1 text-left font-medium">{c.clone()}</th>
                                    }).collect_view()}
                                    <th></th>
                                </tr>
                            </thead>
                            <tbody>
                                {r.rows.into_iter().map(|row| {
                                    let row_pick = row.clone();
                                    let row_tools = row.clone();
                                    view! {
                                        <tr class="border-t border-border">
                                            {row.iter().take(6).map(|cell| view! {
                                                <td class="max-w-32 truncate px-2 py-1">
                                                    {match cell {
                                                        Some(v) => v.clone().into_any(),
                                                        None => view! { <span class="text-muted-foreground">"NULL"</span> }.into_any(),
                                                    }}
                                                </td>
                                            }).collect_view()}
                                            <td class="whitespace-nowrap px-2 py-1 text-right">
                                                <Button size=ButtonSize::Sm variant=ButtonVariant::Outline on_click=move |_| pick_row(row_pick.clone())>
                                                    "Bearbeiten"
                                                </Button>
                                                " "
                                                {player_tools.then(|| view! {
                                                    <Button size=ButtonSize::Sm variant=ButtonVariant::Outline on_click=move |_| open_tools(row_tools.clone())>
                                                        <Icon name="Wrench" class="size-3.5"/>
                                                        "Werkzeuge"
                                                    </Button>
                                                })}
                                            </td>
                                        </tr>
                                    }
                                }).collect_view()}
                            </tbody>
                        </table>
                        {(col_count > 6).then(|| view! {
                            <p class="border-t border-border p-1.5 text-[10px] text-muted-foreground">
                                "Nur die ersten 6 von " {col_count} " Spalten angezeigt - \"Bearbeiten\" zeigt alle."
                            </p>
                        })}
                    </div>
                }
                .into_any()
            })}

            {move || editing_pk.get().map(|pk| view! {
                <GenericRowEditor
                    database=target.database
                    table=target.table
                    pk_value=pk
                    on_close=move |_| editing_pk.set(None)
                    activity_module=MODULE
                />
            })}

            {move || tools_target.get().map(|(pk, label)| view! {
                <PlayerToolsDialog player_id=pk player_label=label on_close=move |_| tools_target.set(None)/>
            })}
        </section>
    }
}
