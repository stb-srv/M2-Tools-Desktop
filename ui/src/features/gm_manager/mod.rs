//! GM-Verwaltung (Port von `src/features/gm-manager/GmManager.tsx`) - lebt als
//! Tab in der Account-/Spieler-Verwaltung.

use crate::components::{Button, ButtonSize, ButtonVariant, Icon, PageHeader};
use crate::features::shared::generic_row_editor::{ColumnInfo, TableRows};
use crate::features::shared::GenericRowEditor;
use crate::util::log_activity;
use crate::store::navigation::{EmbeddedModule, ModuleId};
use crate::tauri::{invoke, invoke_unit};
use leptos::prelude::*;
use leptos::task::spawn_local;
use serde::Deserialize;
use serde_json::json;
use std::collections::HashMap;

#[derive(Clone, Debug, PartialEq, Deserialize)]
struct AccountSummary {
    id: i64,
    login: String,
}

const MODULE: ModuleId = ModuleId::Embedded(EmbeddedModule::GmManager);

// common.gmlist/common.gmhost verified to exist on this core and gmlist's real
// columns (mID/mAccount/mName/mContactIP/mServerIP/mAuthority) verified
// against the DB core's own query (`source/db/src/ClientManager.cpp::
// __GetAdminInfo`) - but the exact column *types* were never live-checked
// against this specific database, so this goes through the same generic,
// schema-introspecting commands as Mob-Proto-Editor/Account-Verwaltung
// instead of a purpose-built struct, and refuses to render if the expected
// columns aren't actually there.
const DB: &str = "common";
const TABLE: &str = "gmlist";

// EGMLevels (source/common/length.h), lowest to highest. Only the 5 above
// GM_PLAYER are ever stored as a string in gmlist.mAuthority.
const AUTHORITY_LEVELS: [(&str, &str); 5] = [
    ("LOW_WIZARD", "Low Wizard (niedrigster GM-Rang)"),
    ("WIZARD", "Wizard"),
    ("HIGH_WIZARD", "High Wizard"),
    ("GOD", "God"),
    ("IMPLEMENTOR", "Implementor (höchster Rang, voller Zugriff)"),
];

const REQUIRED_COLUMNS: [&str; 3] = ["mAccount", "mName", "mAuthority"];

/// Fehlende Pflichtspalten (leer = Schema passt).
pub fn missing_columns(cols: &[ColumnInfo]) -> Vec<&'static str> {
    REQUIRED_COLUMNS.iter().copied().filter(|n| !cols.iter().any(|c| c.name == *n)).collect()
}

/// Werte für `insert_table_row` - nur Spalten, die es wirklich gibt.
pub fn new_gm_values(
    has_column: impl Fn(&str) -> bool,
    account: &str,
    name: &str,
    authority: &str,
    contact_ip: &str,
    server_ip: &str,
) -> Vec<(&'static str, Option<String>)> {
    let mut values = vec![];
    if has_column("mAccount") {
        values.push(("mAccount", Some(account.trim().to_string())));
    }
    if has_column("mName") {
        values.push(("mName", Some(name.trim().to_string())));
    }
    if has_column("mAuthority") {
        values.push(("mAuthority", Some(authority.to_string())));
    }
    if has_column("mContactIP") {
        values.push(("mContactIP", Some(contact_ip.trim().to_string())));
    }
    if has_column("mServerIP") {
        let s = server_ip.trim();
        values.push(("mServerIP", Some(if s.is_empty() { "ALL".to_string() } else { s.to_string() })));
    }
    values
}

fn focus_on_mount(node: NodeRef<leptos::html::Input>) {
    Effect::new(move |_| {
        if let Some(el) = node.get() {
            let _ = el.focus();
        }
    });
}

#[component]
fn AccountPicker(#[prop(into)] on_pick: Callback<String>) -> impl IntoView {
    let open = RwSignal::new(false);
    let query = RwSignal::new(String::new());
    let results = RwSignal::new(Vec::<AccountSummary>::new());
    let searching = RwSignal::new(false);

    let run_search = move || {
        let q = query.get_untracked().trim().to_string();
        searching.set(true);
        spawn_local(async move {
            let res = invoke::<Vec<AccountSummary>>("list_accounts", &json!({ "search": q, "limit": 20, "offset": 0 })).await;
            results.try_set(res.unwrap_or_default());
            searching.try_set(false);
        });
    };

    view! {
        <div class="flex flex-col gap-1">
            <Button variant=ButtonVariant::Outline size=ButtonSize::Sm on_click=move |_| open.update(|o| *o = !*o)>
                <Icon name="Search" class="size-3.5"/>
                " Account suchen…"
            </Button>
            <Show when=move || open.get()>
                {
                    let input_ref = NodeRef::<leptos::html::Input>::new();
                    focus_on_mount(input_ref);
                    view! {
                        <div class="w-72 space-y-1 rounded-md border border-border bg-card p-2 shadow-md">
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
                                    placeholder="Login…"
                                    class="flex-1 rounded-md border border-border bg-background px-2 py-1 text-xs"
                                />
                                <Button variant=ButtonVariant::Outline size=ButtonSize::Sm on_click=move |_| run_search() disabled=searching>
                                    <Icon name="Search" class="size-3.5"/>
                                </Button>
                            </div>
                            <div class="max-h-40 space-y-0.5 overflow-y-auto">
                                <For each=move || results.get() key=|a| a.id let:a>
                                    {
                                        let login = a.login.clone();
                                        view! {
                                            <button
                                                type="button"
                                                on:click=move |_| {
                                                    on_pick.run(login.clone());
                                                    open.set(false);
                                                }
                                                class="block w-full rounded-md px-2 py-1 text-left text-xs hover:bg-muted"
                                            >
                                                {a.login.clone()}
                                            </button>
                                        }
                                    }
                                </For>
                                <Show when=move || results.with(|r| r.is_empty()) && !searching.get()>
                                    <p class="p-1 text-xs text-muted-foreground">"Suchen, um Accounts zu finden."</p>
                                </Show>
                            </div>
                        </div>
                    }
                }
            </Show>
        </div>
    }
}

#[component]
fn Field(#[prop(into)] label: String, children: Children) -> impl IntoView {
    view! {
        <label class="flex flex-col gap-1 text-xs text-muted-foreground">
            {label}
            {children()}
        </label>
    }
}

#[component]
fn NewGmDialog(columns: Vec<ColumnInfo>, #[prop(into)] on_close: Callback<()>, #[prop(into)] on_created: Callback<()>) -> impl IntoView {
    let account = RwSignal::new(String::new());
    let name = RwSignal::new(String::new());
    let authority = RwSignal::new("LOW_WIZARD".to_string());
    let contact_ip = RwSignal::new(String::new());
    let server_ip = RwSignal::new("ALL".to_string());
    let busy = RwSignal::new(false);
    let error = RwSignal::new(None::<String>);
    let account_ref = NodeRef::<leptos::html::Input>::new();
    focus_on_mount(account_ref);

    let can_create = move || !account.with(|a| a.trim().is_empty()) && !name.with(|n| n.trim().is_empty());

    let submit = move || {
        let (acc, nm, auth) = (account.get_untracked(), name.get_untracked(), authority.get_untracked());
        let values = new_gm_values(
            |n| columns.iter().any(|c| c.name == n),
            &acc,
            &nm,
            &auth,
            &contact_ip.get_untracked(),
            &server_ip.get_untracked(),
        );
        busy.set(true);
        error.set(None);
        spawn_local(async move {
            match invoke_unit("insert_table_row", &json!({ "database": DB, "table": TABLE, "values": values })).await {
                Ok(()) => {
                    log_activity(MODULE, "create", &format!("GM '{nm}' ({auth}) für Account '{acc}' angelegt"), Some("gmlist"), Some(&nm));
                    on_created.run(());
                    on_close.run(());
                }
                Err(e) => {
                    error.try_set(Some(e));
                }
            }
            busy.try_set(false);
        });
    };
    let submit = StoredValue::new(submit);

    view! {
        <div class="fixed inset-0 z-50 flex items-center justify-center bg-black/50">
            <div class="w-96 space-y-3 rounded-lg border border-border bg-card p-4">
                <p class="text-sm font-medium">"Neuer GM"</p>
                <Field label="Account (login)">
                    <input
                        node_ref=account_ref
                        prop:value=move || account.get()
                        on:input=move |e| account.set(event_target_value(&e))
                        class="rounded-md border border-border bg-background px-2 py-1 text-sm"
                    />
                </Field>
                <AccountPicker on_pick=move |login: String| account.set(login)/>
                <Field label="Charaktername (exakt wie im Spiel, Groß-/Kleinschreibung zählt)">
                    <input
                        prop:value=move || name.get()
                        on:input=move |e| name.set(event_target_value(&e))
                        class="rounded-md border border-border bg-background px-2 py-1 text-sm"
                    />
                </Field>
                <Field label="Rang">
                    <select
                        prop:value=move || authority.get()
                        on:change=move |e| authority.set(event_target_value(&e))
                        class="rounded-md border border-border bg-background px-2 py-1 text-sm"
                    >
                        {AUTHORITY_LEVELS.iter().map(|(value, label)| {
                            let v = *value;
                            view! { <option value=v selected=move || authority.with(|a| a == v)>{*label}</option> }
                        }).collect_view()}
                    </select>
                </Field>
                <Field label="Server-IP (ALL = auf allen Channels gültig)">
                    <input
                        prop:value=move || server_ip.get()
                        on:input=move |e| server_ip.set(event_target_value(&e))
                        class="rounded-md border border-border bg-background px-2 py-1 text-sm"
                    />
                </Field>
                <Field label="Kontakt-IP (optional)">
                    <input
                        prop:value=move || contact_ip.get()
                        on:input=move |e| contact_ip.set(event_target_value(&e))
                        class="rounded-md border border-border bg-background px-2 py-1 text-sm"
                    />
                </Field>
                {move || error.get().map(|e| view! { <p class="text-sm text-destructive">{e}</p> })}
                <div class="flex justify-end gap-2">
                    <Button variant=ButtonVariant::Outline on_click=move |_| on_close.run(())>"Abbrechen"</Button>
                    <Button disabled=Signal::derive(move || !can_create() || busy.get()) on_click=move |_| submit.with_value(|f| f())>
                        {move || if busy.get() { "Lege an…" } else { "Anlegen" }}
                    </Button>
                </div>
            </div>
        </div>
    }
}

#[component]
pub fn GmManager() -> impl IntoView {
    let columns = RwSignal::new(None::<Vec<ColumnInfo>>);
    let schema_error = RwSignal::new(None::<String>);
    let rows = RwSignal::new(None::<TableRows>);
    let loading = RwSignal::new(false);
    let load_error = RwSignal::new(None::<String>);

    let editing_pk = RwSignal::new(None::<String>);
    let creating = RwSignal::new(false);
    let delete_target = RwSignal::new(None::<(String, String)>);
    let delete_error = RwSignal::new(None::<String>);

    let pk_column = Memo::new(move |_| {
        columns.with(|c| c.as_ref().and_then(|c| c.iter().find(|c| c.is_primary_key).map(|c| c.name.clone())))
    });
    let has_column = move |name: &str| columns.with(|c| c.as_ref().is_some_and(|c| c.iter().any(|c| c.name == name)));
    let col_index = Memo::new(move |_| {
        rows.with(|r| {
            r.as_ref()
                .map(|r| r.columns.iter().enumerate().map(|(i, c)| (c.clone(), i)).collect::<HashMap<_, _>>())
                .unwrap_or_default()
        })
    });

    let load_rows = move || {
        loading.set(true);
        load_error.set(None);
        spawn_local(async move {
            match invoke::<TableRows>("get_table_rows", &json!({ "database": DB, "table": TABLE, "limit": 500, "offset": 0 })).await {
                Ok(r) => {
                    rows.try_set(Some(r));
                }
                Err(e) => {
                    load_error.try_set(Some(e));
                }
            }
            loading.try_set(false);
        });
    };

    schema_error.set(None);
    spawn_local(async move {
        match invoke::<Vec<ColumnInfo>>("get_table_columns", &json!({ "database": DB, "table": TABLE })).await {
            Ok(cols) => {
                let missing = missing_columns(&cols);
                columns.try_set(Some(cols));
                if !missing.is_empty() {
                    schema_error.try_set(Some(format!(
                        "common.gmlist hat nicht die aus dem Server-Quellcode erwarteten Spalten (fehlt: {}) - dieses Modul wurde diese Session nicht live gegen diese Datenbank geprüft, nur gegen den Quellcode.",
                        missing.join(", ")
                    )));
                    return;
                }
                load_rows();
            }
            Err(e) => {
                schema_error.try_set(Some(e));
            }
        }
    });

    let confirm_delete = move || {
        let Some((pk, label)) = delete_target.get_untracked() else { return };
        let Some(pk_col) = pk_column.get_untracked() else { return };
        delete_error.set(None);
        spawn_local(async move {
            match invoke_unit("delete_table_row", &json!({ "database": DB, "table": TABLE, "pkColumn": pk_col, "pkValue": pk })).await {
                Ok(()) => {
                    log_activity(MODULE, "delete", &format!("GM-Rechte für '{label}' entzogen"), Some("gmlist"), Some(&pk));
                    delete_target.set(None);
                    load_rows();
                }
                Err(e) => delete_error.set(Some(e)),
            }
        });
    };

    view! {
        <div class="space-y-6 pb-10">
            <PageHeader module=EmbeddedModule::GmManager title="GM-Verwaltung".to_string()/>
            <p class="max-w-4xl text-sm text-muted-foreground">
                "Bearbeitet " <code>"common.gmlist"</code>
                " direkt - die Tabelle, aus der der DB-Kern beim Start bzw. bei " <code>"/reload a"</code>
                " die GM-Rechte lädt (" <code>"ClientManager.cpp::__GetAdminInfo"</code>
                "). Ein GM-Eintrag ist an einen konkreten Charakternamen gebunden, nicht nur an den Account."
            </p>
            <p class="flex items-start gap-2 rounded-lg border border-amber-500/40 bg-amber-500/10 p-3 text-sm text-amber-700 dark:text-amber-400">
                <Icon name="Info" class="mt-0.5 size-4 shrink-0"/>
                <span>
                    "Wirkt nach dem Ingame-Befehl " <code>"/reload a"</code>
                    " (kein Server-Neustart nötig, quellcode-verifiziert: " <code>"cmd_gm.cpp"</code>
                    " löst darüber ein " <code>"HEADER_GD_RELOAD_ADMIN"</code>
                    " zum DB-Kern aus). Der Rang wird beim Login rein über Charaktername + Account abgeglichen ("
                    <code>"gm_new_get_level"</code> " in " <code>"gm.cpp"</code>
                    ") - Server-IP wird vom DB-Kern bereits per SQL gefiltert (nur Einträge mit " <code>"ALL"</code>
                    " oder der passenden Channel-IP werden geladen), Kontakt-IP wird zwar gespeichert/geloggt, aber in der geprüften Rang-Prüfung selbst nicht ausgewertet."
                </span>
            </p>

            {move || schema_error.get().map(|e| view! {
                <p class="flex items-start gap-2 rounded-lg border border-destructive/40 bg-destructive/10 p-3 text-sm text-destructive">
                    <Icon name="AlertTriangle" class="mt-0.5 size-4 shrink-0"/>
                    {e}
                </p>
            })}

            <Show when=move || schema_error.with(|e| e.is_none())>
                <div class="flex items-center gap-2">
                    <Button variant=ButtonVariant::Outline on_click=move |_| load_rows() disabled=loading>
                        <Icon name="RefreshCw" class=Signal::derive(move || if loading.get() { "size-4 animate-spin".to_string() } else { "size-4 ".to_string() })/>
                        " Neu laden"
                    </Button>
                    <Button on_click=move |_| creating.set(true) disabled=Signal::derive(move || columns.with(|c| c.is_none()))>
                        <Icon name="Plus" class="size-4"/>
                        " Neuer GM…"
                    </Button>
                </div>
                {move || load_error.get().map(|e| view! { <p class="text-sm text-destructive">{e}</p> })}

                {move || rows.get().map(|r| {
                    let idx = col_index.get();
                    let pk = pk_column.get();
                    let has_server_ip = has_column("mServerIP");
                    let has_contact_ip = has_column("mContactIP");
                    let empty = r.rows.is_empty();
                    let get = move |row: &Vec<Option<String>>, col: &str| -> Option<String> {
                        idx.get(col).and_then(|i| row.get(*i).cloned().flatten())
                    };
                    view! {
                        <div class="overflow-x-auto rounded-md border border-border">
                            <table class="w-full text-xs">
                                <thead class="bg-muted/40">
                                    <tr>
                                        <th class="px-2 py-1 text-left font-medium">"Account"</th>
                                        <th class="px-2 py-1 text-left font-medium">"Charakter"</th>
                                        <th class="px-2 py-1 text-left font-medium">"Rang"</th>
                                        <th class="px-2 py-1 text-left font-medium">"Server-IP"</th>
                                        <th class="px-2 py-1 text-left font-medium">"Kontakt-IP"</th>
                                        <th></th>
                                    </tr>
                                </thead>
                                <tbody>
                                    {r.rows.iter().map(|row| {
                                        let pk_value = pk.as_ref().and_then(|p| get(row, p)).unwrap_or_default();
                                        let name = get(row, "mName").unwrap_or_default();
                                        let (pk_edit, pk_del, name_del) = (pk_value.clone(), pk_value.clone(), name.clone());
                                        let no_pk = pk.is_none();
                                        view! {
                                            <tr class="border-t border-border">
                                                <td class="px-2 py-1">{get(row, "mAccount")}</td>
                                                <td class="px-2 py-1 font-medium">{name}</td>
                                                <td class="px-2 py-1">{get(row, "mAuthority")}</td>
                                                <td class="px-2 py-1">{if has_server_ip { get(row, "mServerIP") } else { Some("—".to_string()) }}</td>
                                                <td class="px-2 py-1">
                                                    {if has_contact_ip {
                                                        get(row, "mContactIP").filter(|v| !v.is_empty()).unwrap_or_else(|| "—".into())
                                                    } else {
                                                        "—".to_string()
                                                    }}
                                                </td>
                                                <td class="whitespace-nowrap px-2 py-1 text-right">
                                                    <Button
                                                        size=ButtonSize::Sm
                                                        variant=ButtonVariant::Outline
                                                        on_click=move |_| editing_pk.set(Some(pk_edit.clone()))
                                                        disabled=no_pk
                                                    >
                                                        "Bearbeiten"
                                                    </Button>
                                                    " "
                                                    <Button
                                                        size=ButtonSize::Sm
                                                        variant=ButtonVariant::Outline
                                                        on_click=move |_| delete_target.set(Some((pk_del.clone(), name_del.clone())))
                                                        disabled=no_pk
                                                    >
                                                        <Icon name="Trash2" class="size-3.5"/>
                                                    </Button>
                                                </td>
                                            </tr>
                                        }
                                    }).collect_view()}
                                    {empty.then(|| view! {
                                        <tr>
                                            <td colspan=6 class="px-2 py-3 text-center text-muted-foreground">"Keine GM-Einträge."</td>
                                        </tr>
                                    })}
                                </tbody>
                            </table>
                        </div>
                    }
                })}
            </Show>

            {move || (creating.get()).then(|| columns.get()).flatten().map(|cols| view! {
                <NewGmDialog columns=cols on_close=move |_| creating.set(false) on_created=move |_| load_rows()/>
            })}

            {move || editing_pk.get().filter(|_| pk_column.with(|p| p.is_some())).map(|pk| view! {
                <GenericRowEditor
                    database=DB
                    table=TABLE
                    pk_value=pk
                    on_close=move |_| editing_pk.set(None)
                    on_saved=move |_| load_rows()
                    activity_module=MODULE
                />
            })}

            {move || delete_target.get().map(|(_, label)| view! {
                <div class="fixed inset-0 z-50 flex items-center justify-center bg-black/50">
                    <div class="w-96 space-y-3 rounded-lg border border-border bg-card p-4">
                        <p class="flex items-center gap-2 text-sm font-medium">
                            <Icon name="AlertTriangle" class="size-4 text-destructive"/>
                            {format!(" GM-Rechte für \"{label}\" wirklich entziehen?")}
                        </p>
                        {move || delete_error.get().map(|e| view! { <p class="text-sm text-destructive">{e}</p> })}
                        <div class="flex justify-end gap-2">
                            <Button variant=ButtonVariant::Outline on_click=move |_| delete_target.set(None)>"Abbrechen"</Button>
                            <Button variant=ButtonVariant::Destructive on_click=move |_| confirm_delete()>"Entziehen"</Button>
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

    fn col(name: &str) -> ColumnInfo {
        ColumnInfo { name: name.into(), data_type: "varchar".into(), is_nullable: true, is_primary_key: false }
    }

    #[test]
    fn detects_missing_columns() {
        assert_eq!(missing_columns(&[col("mAccount"), col("mName"), col("mAuthority")]), Vec::<&str>::new());
        assert_eq!(missing_columns(&[col("mName")]), vec!["mAccount", "mAuthority"]);
    }

    #[test]
    fn builds_insert_values_only_for_existing_columns() {
        let cols = ["mAccount", "mName", "mAuthority", "mServerIP"];
        let v = new_gm_values(|n| cols.contains(&n), " acc ", " Name ", "GOD", " 1.2.3.4 ", "  ");
        assert_eq!(
            v,
            vec![
                ("mAccount", Some("acc".to_string())),
                ("mName", Some("Name".to_string())),
                ("mAuthority", Some("GOD".to_string())),
                ("mServerIP", Some("ALL".to_string())),
            ]
        );
    }
}
