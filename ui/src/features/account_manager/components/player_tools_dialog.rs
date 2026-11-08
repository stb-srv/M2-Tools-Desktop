use super::super::shared::{js_error, num_arg, ColumnInfo, TableRows, MODULE};
use crate::components::{Button, ButtonSize, ButtonVariant, Icon};
use crate::lib::log_activity;
use crate::tauri::{invoke, invoke_unit};
use leptos::prelude::*;
use leptos::task::spawn_local;
use serde_json::json;
use std::collections::BTreeMap;

const POSITION_FIELDS: [&str; 3] = ["map_index", "x", "y"];

type Position = BTreeMap<&'static str, Option<String>>;

fn changed_fields(position: &Option<Position>, original: &Option<Position>) -> Vec<&'static str> {
    match (position, original) {
        (Some(p), Some(o)) => POSITION_FIELDS.iter().copied().filter(|f| p.get(f) != o.get(f)).collect(),
        _ => vec![],
    }
}

// Fokussierter Dialog statt des vollen GenericRowEditor - deckt gezielt zwei
// Support-Aufgaben ab: Yang gutschreiben/abziehen (atomar serverseitig, s.
// db/account.rs::adjust_player_gold) und einen festhängenden Spieler per
// direktem Positions-Update entbuggen (x/y/map_index sind die real beim
// Login geladenen Spalten, siehe db/src/ClientManagerPlayer.cpp - wirkt nur,
// während der Spieler offline ist, siehe Hinweis oben auf der Seite).
#[component]
pub fn PlayerToolsDialog(
    #[prop(into)] player_id: String,
    #[prop(into)] player_label: String,
    #[prop(into)] on_close: Callback<()>,
) -> impl IntoView {
    let gold_delta = RwSignal::new(String::new());
    let gold_busy = RwSignal::new(false);
    let gold_error = RwSignal::new(None::<String>);
    let gold_new_value = RwSignal::new(None::<i64>);

    let pk_column = RwSignal::new(None::<String>);
    let position = RwSignal::new(None::<Position>);
    let position_original = RwSignal::new(None::<Position>);
    let pos_load_error = RwSignal::new(None::<String>);
    let pos_saving = RwSignal::new(false);
    let pos_save_error = RwSignal::new(None::<String>);
    let pos_save_ok = RwSignal::new(false);

    {
        let player_id = player_id.clone();
        spawn_local(async move {
            let res: Result<(String, Position), String> = async {
                let cols = invoke::<Vec<ColumnInfo>>("get_table_columns", &json!({ "database": "player", "table": "player" })).await?;
                let pk = cols
                    .iter()
                    .find(|c| c.is_primary_key)
                    .ok_or_else(|| js_error("player.player hat keinen Primärschlüssel."))?;
                let row = invoke::<Option<TableRows>>(
                    "get_table_row",
                    &json!({ "database": "player", "table": "player", "pkColumn": pk.name, "pkValue": player_id }),
                )
                .await?
                .ok_or_else(|| js_error("Spieler nicht gefunden."))?;
                let first = row.rows.first().cloned().unwrap_or_default();
                let values = POSITION_FIELDS
                    .iter()
                    .map(|f| {
                        let v = row.columns.iter().position(|c| c == f).and_then(|i| first.get(i).cloned().flatten());
                        (*f, v)
                    })
                    .collect();
                Ok((pk.name.clone(), values))
            }
            .await;
            match res {
                Ok((pk, values)) => {
                    pk_column.try_set(Some(pk));
                    position.try_set(Some(values.clone()));
                    position_original.try_set(Some(values));
                }
                Err(e) => {
                    pos_load_error.try_set(Some(e));
                }
            }
        });
    }

    let apply_gold = {
        let (player_id, player_label) = (player_id.clone(), player_label.clone());
        move || {
            let d = gold_delta.get_untracked();
            if d.trim().is_empty() {
                return;
            }
            let (player_id, player_label) = (player_id.clone(), player_label.clone());
            gold_busy.set(true);
            gold_error.set(None);
            spawn_local(async move {
                match invoke::<i64>("adjust_player_gold", &json!({ "playerId": num_arg(&player_id), "delta": num_arg(&d) })).await {
                    Ok(v) => {
                        log_activity(
                            MODULE,
                            "adjust-gold",
                            &format!("Yang für Spieler #{player_id} ({player_label}) um {d} angepasst (neu: {v})"),
                            Some("player"),
                            Some(&player_id),
                        );
                        gold_new_value.try_set(Some(v));
                        gold_delta.try_set(String::new());
                    }
                    Err(e) => {
                        gold_error.try_set(Some(e));
                    }
                }
                gold_busy.try_set(false);
            });
        }
    };
    let apply_gold = StoredValue::new(apply_gold);

    let position_changed = Memo::new(move |_| position.with(|p| position_original.with(|o| changed_fields(p, o))));

    let save_position = {
        let (player_id, player_label) = (player_id.clone(), player_label.clone());
        move || {
            let Some(pk) = pk_column.get_untracked() else { return };
            let changed = position_changed.get_untracked();
            if changed.is_empty() {
                return;
            }
            let pos = position.get_untracked().unwrap_or_default();
            let changes: Vec<(&str, Option<String>)> = changed.iter().map(|f| (*f, pos.get(f).cloned().flatten())).collect();
            let (player_id, player_label) = (player_id.clone(), player_label.clone());
            pos_saving.set(true);
            pos_save_error.set(None);
            spawn_local(async move {
                match invoke_unit(
                    "update_table_row",
                    &json!({ "database": "player", "table": "player", "pkColumn": pk, "pkValue": player_id, "changes": changes }),
                )
                .await
                {
                    Ok(()) => {
                        let detail = changed
                            .iter()
                            .map(|f| format!("{f}={}", pos.get(f).cloned().flatten().unwrap_or_else(|| "null".into())))
                            .collect::<Vec<_>>()
                            .join(", ");
                        log_activity(
                            MODULE,
                            "unstick",
                            &format!("Position für Spieler #{player_id} ({player_label}) gesetzt ({detail})"),
                            Some("player"),
                            Some(&player_id),
                        );
                        position_original.try_set(Some(pos));
                        pos_save_ok.try_set(true);
                    }
                    Err(e) => {
                        pos_save_error.try_set(Some(e));
                    }
                }
                pos_saving.try_set(false);
            });
        }
    };
    let save_position = StoredValue::new(save_position);

    let title = if player_label.is_empty() { format!("#{player_id}") } else { player_label.clone() };

    view! {
        <div class="fixed inset-0 z-50 flex items-center justify-center bg-black/50">
            <div class="w-96 space-y-4 rounded-lg border border-border bg-card p-4">
                <div class="flex items-center justify-between">
                    <p class="text-sm font-medium">"Werkzeuge: " {title}</p>
                    <Button variant=ButtonVariant::Ghost size=ButtonSize::Sm on_click=move |_| on_close.run(())>
                        "Schließen"
                    </Button>
                </div>

                <section class="space-y-2 rounded-md border border-border p-3">
                    <p class="text-xs font-medium text-muted-foreground">"Yang gutschreiben/abziehen"</p>
                    <div class="flex gap-2">
                        <input
                            type="number"
                            placeholder="z.B. 100000 oder -50000"
                            prop:value=move || gold_delta.get()
                            on:input=move |e| gold_delta.set(event_target_value(&e))
                            class="flex-1 rounded-md border border-border bg-background px-2 py-1 text-sm"
                        />
                        <Button
                            size=ButtonSize::Sm
                            disabled=Signal::derive(move || gold_delta.with(|d| d.trim().is_empty()) || gold_busy.get())
                            on_click=move |_| apply_gold.with_value(|f| f())
                        >
                            {move || if gold_busy.get() { "…" } else { "Anwenden" }}
                        </Button>
                    </div>
                    {move || gold_error.get().map(|e| view! { <p class="text-sm text-destructive">{e}</p> })}
                    {move || gold_new_value.get().map(|v| view! {
                        <p class="flex items-center gap-1 text-sm text-green-600">
                            <Icon name="CheckCircle2" class="size-4"/>
                            " Neuer Yang-Betrag: "
                            {v}
                        </p>
                    })}
                </section>

                <section class="space-y-2 rounded-md border border-border p-3">
                    <p class="text-xs font-medium text-muted-foreground">
                        "Position setzen (entbuggen - wirkt nur, wenn der Spieler offline ist)"
                    </p>
                    {move || pos_load_error.get().map(|e| view! { <p class="text-sm text-destructive">{e}</p> })}
                    <Show when=move || position.with(|p| p.is_none()) && pos_load_error.with(|e| e.is_none())>
                        <p class="text-sm text-muted-foreground">"Lade…"</p>
                    </Show>
                    <Show when=move || position.with(|p| p.is_some())>
                        <div class="grid grid-cols-3 gap-2">
                            {POSITION_FIELDS.iter().map(|f| {
                                let f = *f;
                                view! {
                                    <label class="flex flex-col gap-0.5 text-xs">
                                        <span class="font-mono text-muted-foreground">{f}</span>
                                        <input
                                            type="number"
                                            prop:value=move || position.with(|p| p.as_ref().and_then(|p| p.get(f).cloned().flatten()).unwrap_or_default())
                                            on:input=move |e| {
                                                let v = event_target_value(&e);
                                                position.update(|p| {
                                                    if let Some(p) = p {
                                                        p.insert(f, Some(v));
                                                    }
                                                });
                                                pos_save_ok.set(false);
                                            }
                                            class="rounded-md border border-border bg-background px-2 py-1 text-sm"
                                        />
                                    </label>
                                }
                            }).collect_view()}
                        </div>
                        {move || pos_save_error.get().map(|e| view! { <p class="text-sm text-destructive">{e}</p> })}
                        <Show when=move || pos_save_ok.get()>
                            <p class="flex items-center gap-1 text-sm text-green-600">
                                <Icon name="CheckCircle2" class="size-4"/>
                                " Gespeichert."
                            </p>
                        </Show>
                        <Button
                            size=ButtonSize::Sm
                            disabled=Signal::derive(move || position_changed.with(|c| c.is_empty()) || pos_saving.get())
                            on_click=move |_| save_position.with_value(|f| f())
                        >
                            {move || if pos_saving.get() { "Speichere…" } else { "Position speichern" }}
                        </Button>
                    </Show>
                </section>
            </div>
        </div>
    }
}
