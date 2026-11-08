use super::super::shared::{focus_on_mount, TableRows, PLAYER_TABLE};
use crate::components::{Button, ButtonSize, ButtonVariant, Icon};
use crate::tauri::invoke;
use leptos::prelude::*;
use leptos::task::spawn_local;
use serde_json::json;

#[derive(Clone, Debug, PartialEq)]
struct Hit {
    id: String,
    name: String,
}

fn hits_from(r: &TableRows) -> Vec<Hit> {
    let id_idx = r.columns.iter().position(|c| c == "id");
    let name_idx = r.columns.iter().position(|c| c == "name");
    r.rows
        .iter()
        .filter_map(|row| {
            // `row[-1]` ist in JS `undefined` (!== null) - die Zeile bliebe mit
            // id "undefined" stehen; ohne id-Spalte ist sie hier unbrauchbar.
            let id = match id_idx {
                Some(i) => row.get(i).cloned().flatten()?,
                None => "undefined".to_string(),
            };
            let name = name_idx.and_then(|i| row.get(i).cloned().flatten()).unwrap_or_default();
            Some(Hit { id, name })
        })
        .collect()
}

// Kleine inline Spieler-Suche (player.player.name -> id), damit man beim
// Item-Geben nicht erst zur Spieler-Suche weiter oben auf der Seite scrollen
// und sich die ID merken muss.
#[component]
pub fn MiniPlayerSearch(#[prop(into)] on_pick: Callback<(String, String)>) -> impl IntoView {
    let open = RwSignal::new(false);
    let query = RwSignal::new(String::new());
    let results = RwSignal::new(Vec::<Hit>::new());
    let searching = RwSignal::new(false);

    let run_search = move || {
        let q = query.get_untracked().trim().to_string();
        if q.is_empty() {
            return;
        }
        searching.set(true);
        spawn_local(async move {
            match invoke::<TableRows>(
                "search_table_rows",
                &json!({ "database": PLAYER_TABLE.database, "table": PLAYER_TABLE.table, "column": "name", "query": q }),
            )
            .await
            {
                Ok(r) => results.set(hits_from(&r)),
                Err(_) => results.set(vec![]),
            }
            searching.set(false);
        });
    };

    view! {
        <div class="flex flex-col gap-1">
            <Button variant=ButtonVariant::Outline size=ButtonSize::Sm on_click=move |_| open.update(|o| *o = !*o)>
                <Icon name="Search" class="size-3.5"/>
                " Spieler suchen…"
            </Button>
            <Show when=move || open.get()>
                {
                    let input_ref = NodeRef::<leptos::html::Input>::new();
                    focus_on_mount(input_ref);
                    view! {
                        <div class="w-64 space-y-1 rounded-md border border-border bg-card p-2 shadow-md">
                            <div class="flex gap-1">
                                <input
                                    node_ref=input_ref
                                    prop:value=move || query.get()
                                    on:input=move |e| query.set(event_target_value(&e))
                                    on:keydown=move |e| {
                                        if e.key() == "Enter" {
                                            run_search()
                                        }
                                    }
                                    placeholder="Charaktername…"
                                    class="flex-1 rounded-md border border-border bg-background px-2 py-1 text-sm"
                                />
                                <Button size=ButtonSize::Sm variant=ButtonVariant::Outline on_click=move |_| run_search() disabled=searching>
                                    <Icon name="Search" class="size-3.5"/>
                                </Button>
                            </div>
                            <For each=move || results.get() key=|r| r.id.clone() let:r>
                                {
                                    let (id, name) = (r.id.clone(), r.name.clone());
                                    view! {
                                        <button
                                            class="block w-full rounded px-2 py-1 text-left text-sm hover:bg-muted"
                                            on:click=move |_| {
                                                on_pick.run((id.clone(), name.clone()));
                                                open.set(false);
                                                results.set(vec![]);
                                                query.set(String::new());
                                            }
                                        >
                                            {r.name.clone()}
                                            " "
                                            <span class="text-muted-foreground">"#"{r.id.clone()}</span>
                                        </button>
                                    }
                                }
                            </For>
                        </div>
                    }
                }
            </Show>
        </div>
    }
}
