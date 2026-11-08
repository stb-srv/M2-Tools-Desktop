//! Datenbank-Explorer (Port von `src/features/db-explorer/DbExplorer.tsx`).

use crate::components::{Button, ButtonSize, ButtonVariant, Icon, PageHeader};
use crate::features::shared::generic_row_editor::{ColumnInfo, TableRows};
use crate::i18n::t;
use crate::util::format::de_int;
use crate::store::navigation::Section;
use crate::tauri::{invoke, invoke0};
use leptos::prelude::*;
use leptos::task::spawn_local;
use serde::Deserialize;
use serde_json::json;

#[derive(Clone, Debug, PartialEq, Deserialize)]
struct TableInfo {
    name: String,
    approx_rows: i64,
}

const PAGE_SIZE: i64 = 50;

pub fn total_pages(total_rows: i64) -> i64 {
    ((total_rows + PAGE_SIZE - 1) / PAGE_SIZE).max(1)
}

/// Standard-Suchspalte: Primärschlüssel, sonst die erste Spalte.
pub fn default_search_column(cols: &[ColumnInfo]) -> Option<String> {
    cols.iter().find(|c| c.is_primary_key).or_else(|| cols.first()).map(|c| c.name.clone())
}

#[component]
pub fn DbExplorer() -> impl IntoView {
    let connected = RwSignal::new(None::<bool>);
    let databases = RwSignal::new(Vec::<String>::new());
    let selected_db = RwSignal::new(None::<String>);
    let tables = RwSignal::new(Vec::<TableInfo>::new());
    let table_filter = RwSignal::new(String::new());
    let selected_table = RwSignal::new(None::<String>);

    let columns = RwSignal::new(Vec::<ColumnInfo>::new());
    let data = RwSignal::new(None::<TableRows>);
    let page = RwSignal::new(0i64);
    let loading = RwSignal::new(false);
    let error = RwSignal::new(None::<String>);

    let search_column = RwSignal::new(None::<String>);
    let search_query = RwSignal::new(String::new());
    let searching = RwSignal::new(false);

    // Nur echte Wertänderungen lösen die Effekte aus (wie React-Abhängigkeiten).
    let db_memo = Memo::new(move |_| selected_db.get());
    let table_memo = Memo::new(move |_| selected_table.get());
    let page_memo = Memo::new(move |_| page.get());

    spawn_local(async move {
        let res: Result<bool, String> = async {
            let ok = invoke0::<bool>("is_mysql_connected").await?;
            connected.try_set(Some(ok));
            if ok {
                let dbs = invoke0::<Vec<String>>("list_databases").await?;
                databases.try_set(dbs);
            }
            Ok(ok)
        }
        .await;
        if res.is_err() {
            connected.try_set(Some(false));
        }
    });

    Effect::new(move |_| {
        let Some(db) = db_memo.get() else { return };
        selected_table.set(None);
        data.set(None);
        spawn_local(async move {
            match invoke::<Vec<TableInfo>>("list_tables", &json!({ "database": db })).await {
                Ok(t) => {
                    tables.try_set(t);
                }
                Err(e) => {
                    error.try_set(Some(e));
                }
            }
        });
    });

    Effect::new(move |_| {
        let (Some(db), Some(table)) = (db_memo.get(), table_memo.get()) else { return };
        page.set(0);
        search_query.set(String::new());
        spawn_local(async move {
            match invoke::<Vec<ColumnInfo>>("get_table_columns", &json!({ "database": db, "table": table })).await {
                Ok(cols) => {
                    search_column.try_set(default_search_column(&cols));
                    columns.try_set(cols);
                }
                Err(e) => {
                    error.try_set(Some(e));
                }
            }
        });
    });

    let load_page = move |p: i64| {
        let (Some(db), Some(table)) = (selected_db.get_untracked(), selected_table.get_untracked()) else { return };
        loading.set(true);
        error.set(None);
        spawn_local(async move {
            match invoke::<TableRows>(
                "get_table_rows",
                &json!({ "database": db, "table": table, "limit": PAGE_SIZE, "offset": p * PAGE_SIZE }),
            )
            .await
            {
                Ok(r) => {
                    data.try_set(Some(r));
                }
                Err(e) => {
                    error.try_set(Some(e));
                }
            }
            loading.try_set(false);
        });
    };

    Effect::new(move |_| {
        let (db, table, p) = (db_memo.get(), table_memo.get(), page_memo.get());
        if db.is_none() || table.is_none() {
            return;
        }
        load_page(p);
    });

    let run_search = move || {
        let (Some(db), Some(table), Some(col)) = (selected_db.get_untracked(), selected_table.get_untracked(), search_column.get_untracked()) else {
            return;
        };
        let q = search_query.get_untracked().trim().to_string();
        if q.is_empty() {
            page.set(0);
            load_page(0);
            return;
        }
        searching.set(true);
        error.set(None);
        spawn_local(async move {
            match invoke::<TableRows>("search_table_rows", &json!({ "database": db, "table": table, "column": col, "query": q })).await {
                Ok(r) => {
                    data.try_set(Some(r));
                }
                Err(e) => {
                    error.try_set(Some(e));
                }
            }
            searching.try_set(false);
        });
    };

    let filtered_tables = move || {
        let f = table_filter.get().to_lowercase();
        tables.get().into_iter().filter(|t| t.name.to_lowercase().contains(&f)).collect::<Vec<_>>()
    };
    let pages = move || data.with(|d| d.as_ref().map(|d| total_pages(d.total_rows)).unwrap_or(1));
    let is_search_result = move || {
        data.with(|d| d.as_ref().is_some_and(|d| d.total_rows == d.rows.len() as i64)) && !search_query.with(|q| q.trim().is_empty())
    };

    view! {
        {move || if connected.get() == Some(false) {
            view! {
                <div class="space-y-2">
                    <PageHeader module=Section::DbExplorer title=Signal::derive(|| t("nav.dbExplorer"))/>
                    <p class="max-w-4xl text-sm text-muted-foreground">
                        "Keine aktive MySQL-Verbindung. Bitte unter Verbindungen einrichten."
                    </p>
                </div>
            }
            .into_any()
        } else {
            view! {
                <div class="flex h-full gap-4">
                    <div class="w-48 shrink-0 space-y-2">
                        <h2 class="text-xs font-medium text-muted-foreground">"Datenbank"</h2>
                        <div class="space-y-1">
                            <For each=move || databases.get() key=|d| d.clone() let:db>
                                {
                                    let (db_click, db_cls) = (db.clone(), db.clone());
                                    view! {
                                        <button
                                            on:click=move |_| selected_db.set(Some(db_click.clone()))
                                            class=move || if selected_db.with(|s| s.as_deref() == Some(db_cls.as_str())) {
                                                "w-full rounded-md px-2 py-1 text-left text-sm hover:bg-muted bg-muted font-medium"
                                            } else {
                                                "w-full rounded-md px-2 py-1 text-left text-sm hover:bg-muted "
                                            }
                                        >
                                            {db}
                                        </button>
                                    }
                                }
                            </For>
                        </div>
                    </div>

                    <div class="w-64 shrink-0 space-y-2">
                        <h2 class="text-xs font-medium text-muted-foreground">"Tabelle"</h2>
                        <Show when=move || selected_db.with(|s| s.is_some())>
                            <input
                                prop:value=move || table_filter.get()
                                on:input=move |e| table_filter.set(event_target_value(&e))
                                placeholder="Filtern…"
                                class="w-full rounded-md border border-border bg-background px-2 py-1 text-sm"
                            />
                        </Show>
                        <div class="max-h-full space-y-1 overflow-y-auto">
                            <For each=filtered_tables key=|t| t.name.clone() let:tbl>
                                {
                                    let (n_click, n_cls) = (tbl.name.clone(), tbl.name.clone());
                                    view! {
                                        <button
                                            on:click=move |_| selected_table.set(Some(n_click.clone()))
                                            class=move || if selected_table.with(|s| s.as_deref() == Some(n_cls.as_str())) {
                                                "flex w-full items-center justify-between rounded-md px-2 py-1 text-left text-sm hover:bg-muted bg-muted font-medium"
                                            } else {
                                                "flex w-full items-center justify-between rounded-md px-2 py-1 text-left text-sm hover:bg-muted "
                                            }
                                        >
                                            <span class="truncate">{tbl.name.clone()}</span>
                                            <span class="text-xs text-muted-foreground">{de_int(tbl.approx_rows)}</span>
                                        </button>
                                    }
                                }
                            </For>
                        </div>
                    </div>

                    <div class="min-w-0 flex-1 space-y-2">
                        {move || error.get().map(|e| view! { <p class="text-sm text-destructive">{e}</p> })}
                        <Show when=move || selected_table.with(|t| t.is_none())>
                            <p class="text-sm text-muted-foreground">"Datenbank und Tabelle links auswählen."</p>
                        </Show>

                        <Show when=move || selected_table.with(|t| t.is_some())>
                            <div class="flex items-center justify-between">
                                <h1 class="text-lg font-semibold">
                                    {move || selected_db.get().unwrap_or_default()} "." {move || selected_table.get().unwrap_or_default()}
                                </h1>
                                <div class="flex items-center gap-2">
                                    <select
                                        prop:value=move || search_column.get().unwrap_or_default()
                                        on:change=move |e| search_column.set(Some(event_target_value(&e)))
                                        class="rounded-md border border-border bg-background px-2 py-1 text-sm"
                                    >
                                        {move || columns.get().into_iter().map(|c| {
                                            let (n_sel, n_val) = (c.name.clone(), c.name.clone());
                                            view! {
                                                <option value=n_val selected=move || search_column.with(|s| s.as_deref() == Some(n_sel.as_str()))>
                                                    {c.name.clone()}
                                                </option>
                                            }
                                        }).collect_view()}
                                    </select>
                                    <input
                                        prop:value=move || search_query.get()
                                        on:input=move |e| search_query.set(event_target_value(&e))
                                        on:keydown=move |e| {
                                            if e.key() == "Enter" {
                                                run_search()
                                            }
                                        }
                                        placeholder="Suchen…"
                                        class="rounded-md border border-border bg-background px-2 py-1 text-sm"
                                    />
                                    <Button variant=ButtonVariant::Outline size=ButtonSize::Sm on_click=move |_| run_search() disabled=searching>
                                        <Icon name="Search" class="size-4"/>
                                    </Button>
                                </div>
                            </div>

                            <div class="overflow-auto rounded-lg border border-border">
                                <table class="w-full text-xs">
                                    <thead class="sticky top-0 bg-card">
                                        <tr>
                                            {move || {
                                                let cols = columns.get();
                                                let names: Vec<String> = data.with(|d| d.as_ref().map(|d| d.columns.clone()))
                                                    .unwrap_or_else(|| cols.iter().map(|c| c.name.clone()).collect());
                                                names.into_iter().map(|col| {
                                                    let is_pk = cols.iter().find(|c| c.name == col).is_some_and(|c| c.is_primary_key);
                                                    view! {
                                                        <th class="whitespace-nowrap border-b border-border px-2 py-1 text-left font-medium">
                                                            <span class="flex items-center gap-1">
                                                                {is_pk.then(|| view! { <Icon name="Key" class="size-3 text-muted-foreground"/> })}
                                                                {col}
                                                            </span>
                                                        </th>
                                                    }
                                                }).collect_view()
                                            }}
                                        </tr>
                                    </thead>
                                    <tbody>
                                        {move || data.get().map(|d| d.rows.into_iter().map(|row| view! {
                                            <tr class="hover:bg-muted/50">
                                                {row.into_iter().map(|cell| {
                                                    let title = cell.clone();
                                                    view! {
                                                        <td
                                                            class="max-w-[200px] truncate whitespace-nowrap border-b border-border px-2 py-1"
                                                            title=title
                                                        >
                                                            {match cell {
                                                                None => view! { <span class="text-muted-foreground italic">"NULL"</span> }.into_any(),
                                                                Some(v) => v.into_any(),
                                                            }}
                                                        </td>
                                                    }
                                                }).collect_view()}
                                            </tr>
                                        }).collect_view())}
                                    </tbody>
                                </table>
                                <Show when=move || loading.get()>
                                    <p class="p-2 text-sm text-muted-foreground">"Lädt…"</p>
                                </Show>
                                <Show when=move || data.with(|d| d.as_ref().is_some_and(|d| d.rows.is_empty()))>
                                    <p class="p-2 text-sm text-muted-foreground">"Keine Zeilen."</p>
                                </Show>
                            </div>

                            <Show when=move || !is_search_result()>
                                <div class="flex items-center justify-between text-sm">
                                    <span class="text-muted-foreground">
                                        {move || data.with(|d| d.as_ref().map(|d| de_int(d.total_rows)))}
                                        {move || format!(" Zeilen · Seite {}/{}", page.get() + 1, pages())}
                                    </span>
                                    <div class="flex gap-1">
                                        <Button
                                            variant=ButtonVariant::Outline
                                            size=ButtonSize::IconSm
                                            on_click=move |_| page.update(|p| *p = (*p - 1).max(0))
                                            disabled=Signal::derive(move || page.get() == 0)
                                        >
                                            <Icon name="ChevronLeft" class="size-4"/>
                                        </Button>
                                        <Button
                                            variant=ButtonVariant::Outline
                                            size=ButtonSize::IconSm
                                            on_click=move |_| {
                                                let last = pages() - 1;
                                                page.update(|p| *p = last.min(*p + 1))
                                            }
                                            disabled=Signal::derive(move || page.get() >= pages() - 1)
                                        >
                                            <Icon name="ChevronRight" class="size-4"/>
                                        </Button>
                                    </div>
                                </div>
                            </Show>
                        </Show>
                    </div>
                </div>
            }
            .into_any()
        }}
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn col(name: &str, pk: bool) -> ColumnInfo {
        ColumnInfo { name: name.into(), data_type: "int".into(), is_nullable: false, is_primary_key: pk }
    }

    #[test]
    fn picks_pk_or_first_column() {
        assert_eq!(default_search_column(&[col("a", false), col("b", true)]).as_deref(), Some("b"));
        assert_eq!(default_search_column(&[col("a", false)]).as_deref(), Some("a"));
        assert_eq!(default_search_column(&[]), None);
    }

    #[test]
    fn page_count() {
        assert_eq!(total_pages(0), 1);
        assert_eq!(total_pages(50), 1);
        assert_eq!(total_pages(51), 2);
    }
}
