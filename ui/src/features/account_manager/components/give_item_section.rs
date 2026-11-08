use super::super::shared::{ColumnInfo, ITEM_TABLE, MODULE};
use super::MiniPlayerSearch;
use crate::components::{toast, Button, ButtonSize, ButtonVariant, Icon};
use crate::features::shared::{EntityBrowser, EntityKind, EntityRow};
use crate::lib::log_activity;
use crate::tauri::{invoke, invoke_unit};
use leptos::prelude::*;
use leptos::task::spawn_local;
use serde_json::json;

/// Ein Spaltenname-Muster (Ersatz für die Regex-Liste im alten Frontend,
/// alle Muster sind case-insensitiv).
type Pattern = fn(&str) -> bool;

fn is_exact_vnum(n: &str) -> bool {
    n == "vnum"
}
fn has_item_vnum(n: &str) -> bool {
    n.contains("item_vnum")
}
fn has_vnum(n: &str) -> bool {
    n.contains("vnum")
}
fn has_owner(n: &str) -> bool {
    n.contains("owner")
}
fn has_player_id(n: &str) -> bool {
    n.contains("player_id")
}
/// `/char.*id/i`
fn has_char_then_id(n: &str) -> bool {
    n.find("char").is_some_and(|i| n[i + 4..].contains("id"))
}
fn has_account_id(n: &str) -> bool {
    n.contains("account_id")
}

const VNUM_PATTERNS: [Pattern; 3] = [is_exact_vnum, has_item_vnum, has_vnum];
const OWNER_PATTERNS: [Pattern; 4] = [has_owner, has_player_id, has_char_then_id, has_account_id];

// Errät anhand des Namensmusters eine wahrscheinliche Spalte aus der echten,
// live geholten Spaltenliste - nur als Ausfüllhilfe für die Picker unten,
// niemals als Ersatz für die generische Eingabe.
pub fn guess_column(columns: &[ColumnInfo], patterns: &[Pattern]) -> Option<String> {
    for p in patterns {
        if let Some(hit) = columns.iter().find(|c| p(&c.name.to_lowercase())) {
            return Some(hit.name.clone());
        }
    }
    None
}

fn copy_to_clipboard(value: &str) {
    if let Some(w) = web_sys::window() {
        // Fehler werden wie im alten Frontend verschluckt.
        let _ = w.navigator().clipboard().write_text(value);
    }
}

type Values = Vec<(String, String)>;

fn set_value(values: &mut Values, col: &str, v: String) {
    match values.iter_mut().find(|(k, _)| k == col) {
        Some(e) => e.1 = v,
        None => values.push((col.to_string(), v)),
    }
}

// Generic insert/delete on player.item - deliberately not core-specific
// (window/pos/count semantics for items were never verified against a real
// server, unlike item_proto). The user fills in every column themselves;
// this just saves writing raw SQL and adds a confirmation step. Die
// Item-/Spieler-Picker unten sind reine Ausfüllhilfen (siehe guess_column) -
// finden sie keine passende Spalte, wird der Wert stattdessen in die
// Zwischenablage kopiert statt eine falsche Spalte zu raten.
#[component]
pub fn GiveItemSection() -> impl IntoView {
    let columns = RwSignal::new(None::<Vec<ColumnInfo>>);
    let load_error = RwSignal::new(None::<String>);
    let values = RwSignal::new(Values::new());
    let open = RwSignal::new(false);
    let confirm_open = RwSignal::new(false);
    let saving = RwSignal::new(false);
    let picker_note = RwSignal::new(None::<String>);

    let delete_id = RwSignal::new(String::new());
    let delete_column = RwSignal::new("id".to_string());
    let delete_confirm = RwSignal::new(false);
    let delete_busy = RwSignal::new(false);

    let apply_picked_value = move |patterns: &[Pattern], value: String, label: String| {
        let Some(cols) = columns.get_untracked() else { return };
        match guess_column(&cols, patterns) {
            Some(col) => {
                values.update(|v| set_value(v, &col, value));
                picker_note.set(Some(format!("{label} in Spalte \"{col}\" übernommen.")));
            }
            None => {
                copy_to_clipboard(&value);
                picker_note.set(Some(format!(
                    "Keine passende Spalte gefunden - \"{value}\" in die Zwischenablage kopiert, bitte manuell einfügen."
                )));
            }
        }
    };

    let open_form = move || {
        open.set(true);
        spawn_local(async move {
            match invoke::<Vec<ColumnInfo>>("get_table_columns", &json!({ "database": ITEM_TABLE.database, "table": ITEM_TABLE.table })).await {
                Ok(cols) => {
                    values.set(cols.iter().map(|c| (c.name.clone(), String::new())).collect());
                    columns.set(Some(cols));
                }
                Err(e) => load_error.set(Some(e)),
            }
        });
    };

    let submit_insert = move || {
        confirm_open.set(false);
        let entries: Vec<(String, Option<String>)> = values
            .get_untracked()
            .into_iter()
            .filter(|(_, v)| !v.trim().is_empty())
            .map(|(k, v)| (k, Some(v)))
            .collect();
        saving.set(true);
        spawn_local(async move {
            match invoke_unit(
                "insert_table_row",
                &json!({ "database": ITEM_TABLE.database, "table": ITEM_TABLE.table, "values": entries }),
            )
            .await
            {
                Ok(()) => {
                    toast::success("Zeile eingefügt.");
                    log_activity(
                        MODULE,
                        "give-item",
                        &format!("Neue Zeile in player.item eingefügt ({} Spalte(n) gesetzt)", entries.len()),
                        Some("item"),
                        None,
                    );
                }
                Err(e) => toast::error(&e),
            }
            saving.set(false);
        });
    };

    let submit_delete = move || {
        delete_confirm.set(false);
        let (col, id) = (delete_column.get_untracked(), delete_id.get_untracked());
        delete_busy.set(true);
        spawn_local(async move {
            match invoke_unit(
                "delete_table_row",
                &json!({ "database": ITEM_TABLE.database, "table": ITEM_TABLE.table, "pkColumn": col, "pkValue": id }),
            )
            .await
            {
                Ok(()) => {
                    toast::success("Zeile gelöscht.");
                    log_activity(MODULE, "remove-item", &format!("Zeile in player.item gelöscht ({col}={id})"), Some("item"), Some(&id));
                }
                Err(e) => toast::error(&e),
            }
            delete_busy.set(false);
        });
    };

    view! {
        <section class="space-y-2 rounded-lg border border-border p-4">
            <h2 class="text-sm font-medium text-muted-foreground">"Item geben / entfernen (player.item)"</h2>
            <p class="text-xs text-muted-foreground">
                "Window/Position/Zähler-Bedeutung von " <code>"player.item"</code>
                " ist core-spezifisch und nicht verifiziert - trage die Spalten so ein, wie sie auf diesem Server erwartet werden (z.B. Besitzer-Spalte mit der ID aus der Spieler-Suche oben füllen)."
            </p>

            <div class="flex gap-2">
                <Button variant=ButtonVariant::Outline size=ButtonSize::Sm on_click=move |_| open_form()>
                    <Icon name="Plus" class="size-3.5"/>
                    "Neue Item-Zeile einfügen…"
                </Button>
            </div>

            <Show when=move || open.get()>
                <div class="space-y-2 rounded-md border border-border p-3">
                    {move || load_error.get().map(|e| view! { <p class="text-sm text-destructive">{e}</p> })}
                    <Show when=move || columns.with(|c| c.is_none()) && load_error.with(|e| e.is_none())>
                        <p class="text-sm text-muted-foreground">"Lade Spalten…"</p>
                    </Show>
                    <Show when=move || columns.with(|c| c.is_some())>
                        <div class="flex flex-wrap items-center gap-2 rounded-md border border-border bg-muted/30 p-2">
                            <EntityBrowser
                                kind=EntityKind::Item
                                pick_label="Vnum übernehmen"
                                on_pick=move |row: EntityRow| {
                                    apply_picked_value(&VNUM_PATTERNS, row.vnum.to_string(), format!("Item-Vnum {}", row.vnum))
                                }
                            />
                            <MiniPlayerSearch on_pick=move |(id, name): (String, String)| {
                                apply_picked_value(&OWNER_PATTERNS, id, format!("Spieler-ID von \"{name}\""))
                            }/>
                        </div>
                        {move || picker_note.get().map(|n| view! { <p class="text-xs text-muted-foreground">{n}</p> })}
                        <div class="grid grid-cols-2 gap-2">
                            {move || columns.get().unwrap_or_default().into_iter().map(|c| {
                                let name = c.name.clone();
                                let name2 = c.name.clone();
                                view! {
                                    <label class="flex flex-col gap-0.5 text-xs">
                                        <span class="font-mono text-muted-foreground">{c.name.clone()}</span>
                                        <input
                                            prop:value=move || values.with(|v| v.iter().find(|(k, _)| *k == name).map(|(_, v)| v.clone()).unwrap_or_default())
                                            on:input=move |e| {
                                                let v = event_target_value(&e);
                                                values.update(|vals| set_value(vals, &name2, v));
                                            }
                                            class="rounded-md border border-border bg-background px-2 py-1 text-sm"
                                        />
                                    </label>
                                }
                            }).collect_view()}
                        </div>
                        <Button size=ButtonSize::Sm on_click=move |_| confirm_open.set(true) disabled=saving>
                            {move || if saving.get() { "Füge ein…" } else { "Einfügen" }}
                        </Button>
                    </Show>
                </div>
            </Show>

            <div class="flex flex-wrap items-end gap-2 rounded-md border border-border p-3">
                <label class="flex flex-col gap-0.5 text-xs text-muted-foreground">
                    "Primärschlüssel-Spalte"
                    <input
                        prop:value=move || delete_column.get()
                        on:input=move |e| delete_column.set(event_target_value(&e))
                        class="w-32 rounded-md border border-border bg-background px-2 py-1 text-sm"
                    />
                </label>
                <label class="flex flex-col gap-0.5 text-xs text-muted-foreground">
                    "Wert der zu löschenden Zeile"
                    <input
                        prop:value=move || delete_id.get()
                        on:input=move |e| delete_id.set(event_target_value(&e))
                        class="w-40 rounded-md border border-border bg-background px-2 py-1 text-sm"
                    />
                </label>
                <Button
                    variant=ButtonVariant::Destructive
                    size=ButtonSize::Sm
                    on_click=move |_| delete_confirm.set(true)
                    disabled=Signal::derive(move || delete_id.with(|d| d.trim().is_empty()) || delete_busy.get())
                >
                    <Icon name="Trash2" class="size-3.5"/>
                    {move || if delete_busy.get() { "Lösche…" } else { "Item-Zeile löschen" }}
                </Button>
            </div>

            <Show when=move || confirm_open.get()>
                <div class="fixed inset-0 z-50 flex items-center justify-center bg-black/50">
                    <div class="w-96 space-y-3 rounded-lg border border-border bg-card p-4">
                        <p class="flex items-start gap-2 text-sm">
                            <Icon name="AlertTriangle" class="mt-0.5 size-4 shrink-0 text-destructive"/>
                            <span>"Eine neue Zeile wird direkt in player.item eingefügt, ohne Rückfrage danach."</span>
                        </p>
                        <div class="flex justify-end gap-2">
                            <Button variant=ButtonVariant::Outline on_click=move |_| confirm_open.set(false)>"Abbrechen"</Button>
                            <Button on_click=move |_| submit_insert()>"Einfügen"</Button>
                        </div>
                    </div>
                </div>
            </Show>

            <Show when=move || delete_confirm.get()>
                <div class="fixed inset-0 z-50 flex items-center justify-center bg-black/50">
                    <div class="w-96 space-y-3 rounded-lg border border-border bg-card p-4">
                        <p class="flex items-start gap-2 text-sm">
                            <Icon name="AlertTriangle" class="mt-0.5 size-4 shrink-0 text-destructive"/>
                            <span>
                                "Zeile mit " <code>{move || delete_column.get()}</code> " = " <code>{move || delete_id.get()}</code>
                                " wird endgültig aus player.item gelöscht."
                            </span>
                        </p>
                        <div class="flex justify-end gap-2">
                            <Button variant=ButtonVariant::Outline on_click=move |_| delete_confirm.set(false)>"Abbrechen"</Button>
                            <Button variant=ButtonVariant::Destructive on_click=move |_| submit_delete()>"Löschen"</Button>
                        </div>
                    </div>
                </div>
            </Show>
        </section>
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn col(name: &str) -> ColumnInfo {
        ColumnInfo { name: name.into(), data_type: "int".into(), is_nullable: false, is_primary_key: false }
    }

    #[test]
    fn guesses_columns_in_pattern_order() {
        let cols = vec![col("id"), col("owner_id"), col("Item_Vnum"), col("vnum")];
        assert_eq!(guess_column(&cols, &VNUM_PATTERNS).as_deref(), Some("vnum"));
        assert_eq!(guess_column(&cols, &OWNER_PATTERNS).as_deref(), Some("owner_id"));
        let cols = vec![col("charID"), col("x")];
        assert_eq!(guess_column(&cols, &OWNER_PATTERNS).as_deref(), Some("charID"));
        assert_eq!(guess_column(&[col("x")], &VNUM_PATTERNS), None);
    }
}
