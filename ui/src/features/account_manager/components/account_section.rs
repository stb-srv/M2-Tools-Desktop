use super::super::shared::{focus_on_mount, AccountSummary, BanRecord, ColumnInfo, MODULE};
use super::{BanDialog, CurrencyDialog, ResetPasswordDialog};
use crate::components::{Button, ButtonSize, ButtonVariant, Icon};
use crate::features::shared::generic_row_editor::is_numeric_type;
use crate::features::shared::GenericRowEditor;
use crate::util::format::de_datetime;
use crate::util::log_activity;
use crate::tauri::{invoke, invoke0, invoke_unit};
use leptos::prelude::*;
use leptos::task::spawn_local;
use serde_json::{json, Value};

const CURRENCY_EXCLUDED_COLUMNS: [&str; 2] = ["id", "empire"];

const EMPIRE_LABELS: [(i64, &str); 4] = [
    (0, "Noch nicht gewählt"),
    (1, "Shinsoo (Rot)"),
    (2, "Chunjo (Gelb)"),
    (3, "Jinno (Blau)"),
];

const PAGE_SIZE: i64 = 20;

fn empire_label(empire: i64) -> String {
    EMPIRE_LABELS.iter().find(|(v, _)| *v == empire).map(|(_, l)| l.to_string()).unwrap_or_else(|| empire.to_string())
}

/// Zusatzwährungs-Spalten: numerisch, kein Primärschlüssel, nicht id/empire.
pub fn currency_columns(cols: Vec<ColumnInfo>) -> Vec<ColumnInfo> {
    cols.into_iter()
        .filter(|c| !c.is_primary_key && !CURRENCY_EXCLUDED_COLUMNS.contains(&c.name.as_str()) && is_numeric_type(&c.data_type))
        .collect()
}

pub fn total_pages(total: i64) -> i64 {
    ((total + PAGE_SIZE - 1) / PAGE_SIZE).max(1)
}

// Eigene, gezielte Commands statt des generischen DB-Explorer-Zeilen-CRUD -
// Passwörter müssen über MySQLs eigene PASSWORD()-Funktion gesetzt werden
// (verifiziert gegen den echten Server-Quellcode: der Login-Check vergleicht
// PASSWORD(eingegebenes_pw) mit der gespeicherten Spalte), das kann der
// generische insert_table_row/update_table_row-Pfad nicht leisten - deshalb
// hier ein eigener kleiner Abschnitt statt TableSearchSection.
#[component]
pub fn AccountSection() -> impl IntoView {
    let accounts = RwSignal::new(None::<Vec<AccountSummary>>);
    let total = RwSignal::new(0i64);
    let page = RwSignal::new(0i64);
    let search = RwSignal::new(String::new());
    let loading = RwSignal::new(false);
    let load_error = RwSignal::new(None::<String>);

    let editing_pk = RwSignal::new(None::<String>);
    let reset_target = RwSignal::new(None::<AccountSummary>);

    let bans = RwSignal::new(Vec::<BanRecord>::new());
    let ban_target = RwSignal::new(None::<AccountSummary>);
    let currency_cols = RwSignal::new(Vec::<ColumnInfo>::new());
    let currency_target = RwSignal::new(None::<AccountSummary>);

    let creating = RwSignal::new(false);
    let new_login = RwSignal::new(String::new());
    let new_password = RwSignal::new(String::new());
    let new_empire = RwSignal::new("0".to_string());
    let create_busy = RwSignal::new(false);
    let create_error = RwSignal::new(None::<String>);

    // Nur die Antwort der jeweils letzten Anfrage zählt.
    let request_id = StoredValue::new(0u64);

    let load = move || {
        let (s, p) = (search.get_untracked(), page.get_untracked());
        let id = request_id.get_value() + 1;
        request_id.set_value(id);
        loading.set(true);
        load_error.set(None);
        spawn_local(async move {
            let res: Result<(Vec<AccountSummary>, i64), String> = async {
                let rows = invoke::<Vec<AccountSummary>>(
                    "list_accounts",
                    &json!({ "search": s, "limit": PAGE_SIZE, "offset": p * PAGE_SIZE }),
                )
                .await?;
                let count = invoke::<i64>("count_accounts", &json!({ "search": s })).await?;
                Ok((rows, count))
            }
            .await;
            if request_id.try_get_value() != Some(id) {
                return;
            }
            match res {
                Ok((rows, count)) => {
                    accounts.set(Some(rows));
                    total.set(count);
                }
                Err(e) => load_error.set(Some(e)),
            }
            loading.set(false);
        });
    };

    Effect::new(move |_| {
        page.track();
        load();
    });

    let load_bans = move || async move {
        match invoke0::<Vec<BanRecord>>("list_account_bans").await {
            Ok(b) => {
                bans.try_set(b);
            }
            Err(e) => {
                load_error.try_set(Some(e));
            }
        }
    };

    // Einmalig beim Öffnen: fällige Sperren automatisch aufheben (kein
    // Server-Cron, siehe bans.rs) und die reale Spaltenliste von
    // account.account holen - falls eine Zusatzwährungs-Spalte (z.B.
    // "Drachenmünzen") existiert, taucht sie hier auf, sonst bleibt die
    // Guthaben-Aktion pro Account unsichtbar statt eine geratene Spalte zu
    // zeigen.
    spawn_local(async move {
        let _ = invoke0::<Value>("process_due_bans").await;
        load_bans().await;
        if let Ok(cols) = invoke::<Vec<ColumnInfo>>("get_table_columns", &json!({ "database": "account", "table": "account" })).await {
            currency_cols.try_set(currency_columns(cols));
        }
    });

    // Seite 0 + neu laden (ein `page.set(0)` auf Seite 0 würde nichts auslösen).
    let reload_first_page = move || {
        if page.get_untracked() != 0 {
            page.set(0);
        } else {
            load();
        }
    };

    let do_unban = move |account_id: i64, ban_id: Option<i64>, login: String| {
        spawn_local(async move {
            match invoke_unit("unban_account", &json!({ "accountId": account_id, "banId": ban_id })).await {
                Ok(()) => {
                    log_activity(MODULE, "unban", &format!("Account '{login}' entsperrt"), Some("account"), Some(&account_id.to_string()));
                    load();
                    load_bans().await;
                }
                Err(e) => load_error.set(Some(e)),
            }
        });
    };

    let unban = move |a: AccountSummary| {
        let active_ban = bans.with_untracked(|b| b.iter().find(|b| b.account_id == a.id && b.active).map(|b| b.id));
        do_unban(a.id, active_ban, a.login);
    };

    let submit_create = move || {
        let login = new_login.get_untracked().trim().to_string();
        let pw = new_password.get_untracked();
        let emp = new_empire.get_untracked();
        let empire = if emp == "0" { Value::Null } else { super::super::shared::num_arg(&emp) };
        create_busy.set(true);
        create_error.set(None);
        spawn_local(async move {
            match invoke_unit("create_account", &json!({ "login": login, "password": pw, "empire": empire })).await {
                Ok(()) => {
                    log_activity(MODULE, "create", &format!("Account '{login}' angelegt"), Some("account"), Some(&login));
                    creating.set(false);
                    new_login.set(String::new());
                    new_password.set(String::new());
                    new_empire.set("0".into());
                    reload_first_page();
                }
                Err(e) => create_error.set(Some(e)),
            }
            create_busy.set(false);
        });
    };

    let can_create = move || !new_login.with(|l| l.trim().is_empty()) && !new_password.with(|p| p.is_empty());

    view! {
        <section class="space-y-2 rounded-lg border border-border p-4">
            <div class="flex items-center justify-between">
                <h2 class="text-sm font-medium text-muted-foreground">"Accounts"</h2>
                <Button size=ButtonSize::Sm on_click=move |_| creating.set(true)>
                    <Icon name="Plus" class="size-3.5"/>
                    "Neuer Account…"
                </Button>
            </div>

            <div class="flex gap-2">
                <input
                    prop:value=move || search.get()
                    on:input=move |e| search.set(event_target_value(&e))
                    on:keydown=move |e| {
                        if e.key() == "Enter" {
                            reload_first_page()
                        }
                    }
                    placeholder="Login suchen (leer = alle anzeigen)…"
                    class="flex-1 rounded-md border border-border bg-background px-2 py-1 text-sm"
                />
                <Button variant=ButtonVariant::Outline on_click=move |_| reload_first_page() disabled=loading>
                    <Icon name="Search" class="size-4"/>
                </Button>
            </div>

            {move || load_error.get().map(|e| view! {
                <p class="flex items-start gap-2 text-sm text-destructive">
                    <Icon name="AlertTriangle" class="mt-0.5 size-4 shrink-0"/>
                    <span>{e}</span>
                </p>
            })}

            {move || {
                let active: Vec<BanRecord> = bans.with(|b| b.iter().filter(|b| b.active).cloned().collect());
                (!active.is_empty()).then(|| view! {
                    <div class="space-y-1 rounded-md border border-amber-500/40 bg-amber-500/10 p-2">
                        <p class="text-xs font-medium text-amber-700 dark:text-amber-400">"Aktive Sperren"</p>
                        {active.into_iter().map(|b| {
                            let (account_id, ban_id, login) = (b.account_id, b.id, b.login.clone());
                            view! {
                                <div class="flex items-center justify-between gap-2 text-xs">
                                    <span>
                                        <span class="font-medium">{b.login.clone()}</span>
                                        " — "
                                        {b.reason.clone()}
                                        " · "
                                        {match &b.unban_at {
                                            Some(u) => format!("automatische Entsperrung: {}", de_datetime(u)),
                                            None => "dauerhaft".to_string(),
                                        }}
                                    </span>
                                    <Button
                                        size=ButtonSize::Sm
                                        variant=ButtonVariant::Outline
                                        on_click=move |_| do_unban(account_id, Some(ban_id), login.clone())
                                    >
                                        <Icon name="LockOpen" class="size-3.5"/>
                                        "Jetzt entsperren"
                                    </Button>
                                </div>
                            }
                        }).collect_view()}
                    </div>
                })
            }}

            {move || accounts.get().map(|list| {
                let empty = list.is_empty();
                view! {
                    <div class="overflow-x-auto rounded-md border border-border">
                        <table class="w-full text-xs">
                            <thead class="bg-muted/40">
                                <tr>
                                    <th class="px-2 py-1 text-left font-medium">"ID"</th>
                                    <th class="px-2 py-1 text-left font-medium">"Login"</th>
                                    <th class="px-2 py-1 text-left font-medium">"E-Mail"</th>
                                    <th class="px-2 py-1 text-left font-medium">"Status"</th>
                                    <th class="px-2 py-1 text-left font-medium">"Reich"</th>
                                    <th class="px-2 py-1 text-left font-medium">"Erstellt"</th>
                                    <th class="px-2 py-1 text-left font-medium">"Zuletzt gespielt"</th>
                                    <th></th>
                                </tr>
                            </thead>
                            <tbody>
                                {list.into_iter().map(|a| {
                                    let (a_edit, a_reset, a_ban, a_unban, a_cur) = (a.clone(), a.clone(), a.clone(), a.clone(), a.clone());
                                    let status_ok = a.status == "OK";
                                    view! {
                                        <tr class="border-t border-border">
                                            <td class="px-2 py-1">{a.id}</td>
                                            <td class="px-2 py-1 font-medium">{a.login.clone()}</td>
                                            <td class="max-w-32 truncate px-2 py-1">{if a.email.is_empty() { "—".to_string() } else { a.email.clone() }}</td>
                                            <td class="px-2 py-1">{a.status.clone()}</td>
                                            <td class="px-2 py-1">{empire_label(a.empire)}</td>
                                            <td class="px-2 py-1">{a.create_time.clone()}</td>
                                            <td class="px-2 py-1">{a.last_play.clone().unwrap_or_else(|| "—".into())}</td>
                                            <td class="whitespace-nowrap px-2 py-1 text-right">
                                                <Button size=ButtonSize::Sm variant=ButtonVariant::Outline on_click=move |_| editing_pk.set(Some(a_edit.id.to_string()))>
                                                    "Bearbeiten"
                                                </Button>
                                                " "
                                                <Button size=ButtonSize::Sm variant=ButtonVariant::Outline on_click=move |_| reset_target.set(Some(a_reset.clone()))>
                                                    <Icon name="KeyRound" class="size-3.5"/>
                                                    "Passwort"
                                                </Button>
                                                " "
                                                {if status_ok {
                                                    view! {
                                                        <Button size=ButtonSize::Sm variant=ButtonVariant::Outline on_click=move |_| ban_target.set(Some(a_ban.clone()))>
                                                            <Icon name="Lock" class="size-3.5"/>
                                                            "Sperren"
                                                        </Button>
                                                    }.into_any()
                                                } else {
                                                    view! {
                                                        <Button size=ButtonSize::Sm variant=ButtonVariant::Outline on_click=move |_| unban(a_unban.clone())>
                                                            <Icon name="LockOpen" class="size-3.5"/>
                                                            "Entsperren"
                                                        </Button>
                                                    }.into_any()
                                                }}
                                                " "
                                                {move || {
                                                    let a_cur = a_cur.clone();
                                                    currency_cols.with(|c| !c.is_empty()).then(move || view! {
                                                        <Button size=ButtonSize::Sm variant=ButtonVariant::Outline on_click=move |_| currency_target.set(Some(a_cur.clone()))>
                                                            <Icon name="Coins" class="size-3.5"/>
                                                            "Guthaben"
                                                        </Button>
                                                    })
                                                }}
                                            </td>
                                        </tr>
                                    }
                                }).collect_view()}
                                {empty.then(|| view! {
                                    <tr>
                                        <td colspan=8 class="px-2 py-3 text-center text-muted-foreground">"Keine Accounts gefunden."</td>
                                    </tr>
                                })}
                            </tbody>
                        </table>
                        <div class="flex items-center justify-between border-t border-border p-1.5 text-[10px] text-muted-foreground">
                            <span>
                                {move || {
                                    let t = total.get();
                                    format!(
                                        "{t} Account{} - Seite {} von {}",
                                        if t == 1 { "" } else { "s" },
                                        page.get() + 1,
                                        total_pages(t)
                                    )
                                }}
                            </span>
                            <div class="flex gap-1">
                                <Button
                                    variant=ButtonVariant::Ghost
                                    size=ButtonSize::IconSm
                                    disabled=Signal::derive(move || page.get() == 0)
                                    on_click=move |_| page.update(|p| *p -= 1)
                                >
                                    <Icon name="ChevronLeft" class="size-3.5"/>
                                </Button>
                                <Button
                                    variant=ButtonVariant::Ghost
                                    size=ButtonSize::IconSm
                                    disabled=Signal::derive(move || page.get() + 1 >= total_pages(total.get()))
                                    on_click=move |_| page.update(|p| *p += 1)
                                >
                                    <Icon name="ChevronRight" class="size-3.5"/>
                                </Button>
                            </div>
                        </div>
                    </div>
                }
            })}

            {move || editing_pk.get().map(|pk| view! {
                <GenericRowEditor
                    database="account"
                    table="account"
                    pk_value=pk
                    on_close=move |_| editing_pk.set(None)
                    on_saved=move |_| load()
                    activity_module=MODULE
                />
            })}

            {move || reset_target.get().map(|a| view! {
                <ResetPasswordDialog account=a on_close=move |_| reset_target.set(None)/>
            })}

            {move || ban_target.get().map(|a| view! {
                <BanDialog
                    account=a
                    on_close=move |_| ban_target.set(None)
                    on_banned=move |_| {
                        ban_target.set(None);
                        load();
                        spawn_local(load_bans());
                    }
                />
            })}

            {move || currency_target.get().map(|a| view! {
                <CurrencyDialog account=a columns=currency_cols.get_untracked() on_close=move |_| currency_target.set(None)/>
            })}

            <Show when=move || creating.get()>
                {
                    let login_ref = NodeRef::<leptos::html::Input>::new();
                    focus_on_mount(login_ref);
                    view! {
                        <div class="fixed inset-0 z-50 flex items-center justify-center bg-black/50">
                            <div class="w-96 space-y-3 rounded-lg border border-border bg-card p-4">
                                <p class="text-sm font-medium">"Neuer Account"</p>
                                <label class="flex flex-col gap-1 text-xs text-muted-foreground">
                                    "Login"
                                    <input
                                        node_ref=login_ref
                                        prop:value=move || new_login.get()
                                        on:input=move |e| new_login.set(event_target_value(&e))
                                        class="rounded-md border border-border bg-background px-2 py-1 text-sm"
                                    />
                                </label>
                                <label class="flex flex-col gap-1 text-xs text-muted-foreground">
                                    "Passwort"
                                    <input
                                        type="text"
                                        prop:value=move || new_password.get()
                                        on:input=move |e| new_password.set(event_target_value(&e))
                                        class="rounded-md border border-border bg-background px-2 py-1 text-sm"
                                    />
                                </label>
                                <label class="flex flex-col gap-1 text-xs text-muted-foreground">
                                    "Reich (optional, sonst wählt der Spieler beim ersten Login)"
                                    <select
                                        prop:value=move || new_empire.get()
                                        on:change=move |e| new_empire.set(event_target_value(&e))
                                        class="rounded-md border border-border bg-background px-2 py-1 text-sm"
                                    >
                                        {EMPIRE_LABELS.iter().map(|(v, l)| {
                                            let v = v.to_string();
                                            let v2 = v.clone();
                                            view! { <option value=v selected=move || new_empire.with(|e| *e == v2)>{*l}</option> }
                                        }).collect_view()}
                                    </select>
                                </label>
                                {move || create_error.get().map(|e| view! { <p class="text-sm text-destructive">{e}</p> })}
                                <div class="flex justify-end gap-2">
                                    <Button variant=ButtonVariant::Outline on_click=move |_| creating.set(false)>"Abbrechen"</Button>
                                    <Button
                                        disabled=Signal::derive(move || !can_create() || create_busy.get())
                                        on_click=move |_| submit_create()
                                    >
                                        {move || if create_busy.get() { "Lege an…" } else { "Anlegen" }}
                                    </Button>
                                </div>
                            </div>
                        </div>
                    }
                }
            </Show>
        </section>
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn col(name: &str, ty: &str, pk: bool) -> ColumnInfo {
        ColumnInfo { name: name.into(), data_type: ty.into(), is_nullable: false, is_primary_key: pk }
    }

    #[test]
    fn currency_columns_only_numeric_non_pk() {
        let cols = vec![
            col("id", "int", true),
            col("login", "varchar", false),
            col("empire", "tinyint", false),
            col("cash", "INT(11)", false),
            col("coins", "bigint", false),
        ];
        let names: Vec<String> = currency_columns(cols).into_iter().map(|c| c.name).collect();
        assert_eq!(names, vec!["cash", "coins"]);
    }

    #[test]
    fn pages() {
        assert_eq!(total_pages(0), 1);
        assert_eq!(total_pages(20), 1);
        assert_eq!(total_pages(21), 2);
        assert_eq!(empire_label(2), "Chunjo (Gelb)");
        assert_eq!(empire_label(9), "9");
    }
}
