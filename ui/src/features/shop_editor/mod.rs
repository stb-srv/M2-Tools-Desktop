//! Shop Editor: Shops und ihre Waren im Spielraster bearbeiten, mit
//! 3D-Vorschau des Händler-NPCs (Port von `src/features/shop-editor`).

mod logic;

use crate::components::{Button, ButtonSize, ButtonVariant, Icon, PageHeader, PasswordInput};
use crate::features::model_viewer::{Gr2Canvas, Gr2ModelInfo};
use crate::features::shared::{EntityBrowser, EntityKind, EntityRow, RowRenderer};
use crate::i18n::t;
use crate::util::connection::auto_connect_mysql;
use crate::util::{log_activity, open_manual};
use crate::store::navigation::{ModuleId, Section};
use crate::tauri::{invoke, invoke0, invoke_unit};
use leptos::prelude::*;
use leptos::task::spawn_local;
use logic::{js_number, js_number_json, place_items, update_columns, update_rows, ShopItem, MAX_SLOTS};
use serde::Deserialize;
use serde_json::{json, Value};
use std::collections::{HashMap, HashSet};
use std::sync::Arc;

#[derive(Clone, Debug, PartialEq, Deserialize)]
struct ShopSummary {
    vnum: i64,
    name: String,
    npc_vnum: i64,
    #[allow(dead_code)]
    npc_name: String,
    npc_folder: Option<String>,
    item_count: i64,
}

#[derive(Clone, Debug, PartialEq)]
enum CellSelection {
    Empty,
    Filled(ShopItem),
}

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
enum Mode {
    Global,
    ProShop,
}

const MODULE: Section = Section::ShopEditor;

fn module_id() -> ModuleId {
    MODULE.into()
}

#[component]
pub fn ShopEditor() -> impl IntoView {
    let connected = RwSignal::new(false);
    let checking_connection = RwSignal::new(true);
    let connecting = RwSignal::new(false);
    let conn_error = RwSignal::new(None::<String>);
    let host = RwSignal::new(String::new());
    let port = RwSignal::new("3306".to_string());
    let username = RwSignal::new(String::new());
    let password = RwSignal::new(String::new());

    let shops = RwSignal::new(Vec::<ShopSummary>::new());
    let shop_search = RwSignal::new(String::new());
    let selected_shop = RwSignal::new(None::<ShopSummary>);
    let shop_items = RwSignal::new(Vec::<ShopItem>::new());
    let error = RwSignal::new(None::<String>);
    let renaming = RwSignal::new(false);
    let name_draft = RwSignal::new(String::new());

    let columns = RwSignal::new(5i64);
    let rows = RwSignal::new(8i64);
    let mode = RwSignal::new(Mode::Global);
    let max_value = RwSignal::new(200i64);
    let icons = RwSignal::new(HashMap::<i64, Option<String>>::new());
    let icons_requested = StoredValue::new(HashSet::<i64>::new());

    let npc_model = RwSignal::new(None::<Arc<Gr2ModelInfo>>);
    let npc_model_error = RwSignal::new(None::<String>);
    let npc_model_loading = RwSignal::new(false);

    let cell = RwSignal::new(None::<CellSelection>);

    let sync_confirm = RwSignal::new(false);
    let syncing = RwSignal::new(false);
    let sync_result = RwSignal::new(None::<i64>);

    let delete_confirm = RwSignal::new(None::<ShopSummary>);
    let creating = RwSignal::new(false);
    let new_shop_name = RwSignal::new(String::new());
    let new_shop_npc_vnum = RwSignal::new(String::new());

    // ------------------------------------------------------------ Aktionen

    let refresh_shops = move || async move {
        error.set(None);
        match invoke0::<Vec<ShopSummary>>("list_shops").await {
            Ok(r) => shops.set(r),
            Err(e) => error.set(Some(e)),
        }
    };

    let select_shop = move |shop: ShopSummary| async move {
        let vnum = shop.vnum;
        selected_shop.set(Some(shop));
        renaming.set(false);
        error.set(None);
        match invoke::<Vec<ShopItem>>("get_shop_items", &json!({ "shopVnum": vnum })).await {
            Ok(items) => shop_items.set(items),
            Err(e) => error.set(Some(e)),
        }
    };

    // Beim Mounten: meist ist die Verbindung schon da (App-Start verbindet
    // automatisch) - sonst das manuelle Formular vorbelegen.
    spawn_local(async move {
        if auto_connect_mysql().await {
            connected.set(true);
            refresh_shops().await;
            checking_connection.set(false);
            return;
        }
        let get = |key: &'static str| async move {
            invoke::<Option<String>>("get_setting", &json!({ "key": key })).await
        };
        if let (Ok(h), Ok(p), Ok(u)) = (get("mysql_host").await, get("mysql_port").await, get("mysql_username").await) {
            if let Some(h) = h.filter(|s| !s.is_empty()) {
                host.set(h);
            }
            if let Some(p) = p.filter(|s| !s.is_empty()) {
                port.set(p);
            }
            if let Some(u) = u.filter(|s| !s.is_empty()) {
                username.set(u);
            }
        }
        checking_connection.set(false);
    });

    // NPC-Modell des gewählten Shops laden.
    let model_key = Memo::new(move |_| {
        selected_shop.with(|s| s.as_ref().map(|s| (s.vnum, s.npc_vnum, s.npc_folder.clone())))
    });
    let model_request = StoredValue::new(0u64);
    Effect::new(move |_| {
        let key = model_key.get();
        let id = model_request.get_value() + 1;
        model_request.set_value(id);
        let Some((_, npc_vnum, folder)) = key else {
            npc_model.set(None);
            npc_model_error.set(None);
            return;
        };
        npc_model_loading.set(true);
        npc_model_error.set(None);
        npc_model.set(None);
        spawn_local(async move {
            let res: Result<Gr2ModelInfo, String> = async {
                let (dll_path, gr2_path) = invoke::<(String, String)>(
                    "locate_npc_model",
                    &json!({ "npcVnum": npc_vnum, "folder": folder }),
                )
                .await?;
                invoke::<Gr2ModelInfo>("load_gr2_model", &json!({ "grannyDllPath": dll_path, "gr2Path": gr2_path })).await
            }
            .await;
            if model_request.get_value() != id {
                return;
            }
            match res {
                Ok(m) => npc_model.set(Some(Arc::new(m))),
                Err(e) => npc_model_error.set(Some(e)),
            }
            npc_model_loading.set(false);
        });
    });

    let scope_vnum = move || -> Option<i64> {
        if mode.get_untracked() == Mode::ProShop {
            selected_shop.with_untracked(|s| s.as_ref().map(|s| s.vnum))
        } else {
            None
        }
    };

    let max_key = Memo::new(move |_| (mode.get(), selected_shop.with(|s| s.as_ref().map(|s| s.vnum))));
    Effect::new(move |_| {
        let (m, vnum) = max_key.get();
        if vnum.is_none() && m == Mode::ProShop {
            return;
        }
        let shop_vnum = if m == Mode::ProShop { vnum } else { None };
        spawn_local(async move {
            if let Ok(v) = invoke::<i64>("get_shop_default_max", &json!({ "shopVnum": shop_vnum })).await {
                max_value.set(v);
            }
        });
    });

    let ensure_icons = move |vnums: Vec<i64>| {
        let mut seen = HashSet::new();
        for vnum in vnums {
            if !seen.insert(vnum) {
                continue;
            }
            if icons.with_untracked(|i| i.contains_key(&vnum)) || icons_requested.with_value(|r| r.contains(&vnum)) {
                continue;
            }
            icons_requested.update_value(|r| {
                r.insert(vnum);
            });
            spawn_local(async move {
                let res = invoke::<Option<String>>("get_item_icon", &json!({ "vnum": vnum })).await;
                icons.update(|i| {
                    i.insert(vnum, res.ok().flatten());
                });
            });
        }
    };

    Effect::new(move |_| {
        let vnums: Vec<i64> = shop_items.with(|items| items.iter().map(|i| i.item_vnum).collect());
        ensure_icons(vnums);
    });

    let set_columns = move |next: i64| {
        let (c, r) = update_columns(next, rows.get_untracked());
        columns.set(c);
        rows.set(r);
    };
    let set_rows = move |next: i64| {
        let (c, r) = update_rows(next, columns.get_untracked());
        columns.set(c);
        rows.set(r);
    };

    let handle_connect = move || {
        connecting.set(true);
        conn_error.set(None);
        let (h, p, u, pw) = (host.get_untracked(), port.get_untracked(), username.get_untracked(), password.get_untracked());
        spawn_local(async move {
            let res: Result<(), String> = async {
                let config = json!({ "host": h, "port": js_number_json(js_number(&p)), "username": u, "database": Value::Null });
                invoke_unit("connect_mysql", &json!({ "config": config, "password": pw })).await?;
                invoke_unit("set_setting", &json!({ "key": "mysql_host", "value": h })).await?;
                invoke_unit("set_setting", &json!({ "key": "mysql_port", "value": p })).await?;
                invoke_unit("set_setting", &json!({ "key": "mysql_username", "value": u })).await?;
                invoke_unit("store_credential", &json!({ "account": "mysql_password", "secret": pw })).await?;
                connected.set(true);
                refresh_shops().await;
                Ok(())
            }
            .await;
            if let Err(e) = res {
                conn_error.set(Some(e));
            }
            connecting.set(false);
        });
    };

    let change_count = move |item: ShopItem, delta: i64| {
        let Some(shop) = selected_shop.get_untracked() else { return };
        let previous_count = item.count;
        let next_count = (item.count + delta).max(1);
        let vnum = item.item_vnum;
        shop_items.update(|items| {
            for i in items.iter_mut().filter(|i| i.item_vnum == vnum) {
                i.count = next_count;
            }
        });
        spawn_local(async move {
            let res = invoke_unit(
                "update_shop_item_count",
                &json!({ "shopVnum": shop.vnum, "itemVnum": vnum, "count": next_count }),
            )
            .await;
            match res {
                Ok(()) => log_activity(
                    module_id(),
                    "update-count",
                    &format!("Stückzahl von Item {vnum} in Shop '{}' auf {next_count} gesetzt", shop.name),
                    Some("shop"),
                    Some(&shop.vnum.to_string()),
                ),
                Err(e) => {
                    error.set(Some(e));
                    // Optimistische Änderung zurücknehmen - sonst zeigt das
                    // Raster eine nie gespeicherte Stückzahl an.
                    shop_items.update(|items| {
                        for i in items.iter_mut().filter(|i| i.item_vnum == vnum) {
                            i.count = previous_count;
                        }
                    });
                }
            }
        });
    };

    let remove_item = move |item: ShopItem| {
        let Some(shop) = selected_shop.get_untracked() else { return };
        let vnum = item.item_vnum;
        spawn_local(async move {
            match invoke_unit("remove_shop_item", &json!({ "shopVnum": shop.vnum, "itemVnum": vnum })).await {
                Ok(()) => {
                    log_activity(
                        module_id(),
                        "remove-item",
                        &format!("Item {vnum} aus Shop '{}' entfernt", shop.name),
                        Some("shop"),
                        Some(&shop.vnum.to_string()),
                    );
                    shop_items.update(|items| items.retain(|i| i.item_vnum != vnum));
                    cell.set(None);
                }
                Err(e) => error.set(Some(e)),
            }
        });
    };

    let add_item = move |item: EntityRow| {
        let Some(shop) = selected_shop.get_untracked() else { return };
        let count = max_value.get_untracked();
        spawn_local(async move {
            let res = invoke_unit(
                "add_shop_item",
                &json!({ "shopVnum": shop.vnum, "itemVnum": item.vnum, "count": count }),
            )
            .await;
            match res {
                Ok(()) => {
                    log_activity(
                        module_id(),
                        "add-item",
                        &format!("Item {} zu Shop '{}' hinzugefügt", item.vnum, shop.name),
                        Some("shop"),
                        Some(&shop.vnum.to_string()),
                    );
                    select_shop(shop).await;
                    cell.set(None);
                }
                Err(e) => error.set(Some(e)),
            }
        });
    };

    let confirm_delete = move || {
        let Some(del) = delete_confirm.get_untracked() else { return };
        spawn_local(async move {
            match invoke_unit("delete_shop", &json!({ "shopVnum": del.vnum })).await {
                Ok(()) => {
                    log_activity(
                        module_id(),
                        "delete",
                        &format!("Shop '{}' gelöscht", del.name),
                        Some("shop"),
                        Some(&del.vnum.to_string()),
                    );
                    shops.update(|s| s.retain(|s| s.vnum != del.vnum));
                    if selected_shop.with_untracked(|s| s.as_ref().map(|s| s.vnum)) == Some(del.vnum) {
                        selected_shop.set(None);
                        shop_items.set(vec![]);
                    }
                }
                Err(e) => error.set(Some(e)),
            }
            delete_confirm.set(None);
        });
    };

    let save_max_value = move |next: i64| {
        max_value.set(next);
        let shop_vnum = scope_vnum();
        spawn_local(async move {
            if let Err(e) = invoke_unit("set_shop_default_max", &json!({ "shopVnum": shop_vnum, "value": next })).await {
                error.set(Some(e));
            }
        });
    };

    let confirm_sync = move || {
        syncing.set(true);
        sync_result.set(None);
        let shop_vnum = scope_vnum();
        let count = max_value.get_untracked();
        let pro = mode.get_untracked() == Mode::ProShop;
        spawn_local(async move {
            match invoke::<i64>("sync_shop_stack_sizes", &json!({ "shopVnum": shop_vnum, "count": count })).await {
                Ok(affected) => {
                    let scope = match selected_shop.get_untracked() {
                        Some(s) if pro => format!("Shop '{}'", s.name),
                        _ => "alle Shops".to_string(),
                    };
                    log_activity(
                        module_id(),
                        "sync-stack-sizes",
                        &format!("Stückzahlen für {scope} synchronisiert ({affected} Zeile(n))"),
                        Some("shop"),
                        None,
                    );
                    sync_result.set(Some(affected));
                    refresh_shops().await;
                    if let Some(s) = selected_shop.get_untracked() {
                        select_shop(s).await;
                    }
                }
                Err(e) => error.set(Some(e)),
            }
            syncing.set(false);
            sync_confirm.set(false);
        });
    };

    let save_rename = move || {
        let Some(shop) = selected_shop.get_untracked() else {
            renaming.set(false);
            return;
        };
        let draft = name_draft.get_untracked();
        if draft.trim().is_empty() || draft == shop.name {
            renaming.set(false);
            return;
        }
        let new_name = draft.trim().to_string();
        spawn_local(async move {
            match invoke_unit("rename_shop", &json!({ "shopVnum": shop.vnum, "name": new_name })).await {
                Ok(()) => {
                    log_activity(
                        module_id(),
                        "rename",
                        &format!("Shop '{}' umbenannt zu '{new_name}'", shop.name),
                        Some("shop"),
                        Some(&shop.vnum.to_string()),
                    );
                    let updated = ShopSummary { name: new_name, ..shop };
                    selected_shop.set(Some(updated.clone()));
                    shops.update(|s| {
                        for s in s.iter_mut().filter(|s| s.vnum == updated.vnum) {
                            *s = updated.clone();
                        }
                    });
                }
                Err(e) => error.set(Some(e)),
            }
            renaming.set(false);
        });
    };

    let submit_new_shop = move || {
        let name = new_shop_name.get_untracked();
        let npc_vnum = js_number(&new_shop_npc_vnum.get_untracked());
        if name.trim().is_empty() || !npc_vnum.is_finite() {
            return;
        }
        let name = name.trim().to_string();
        spawn_local(async move {
            match invoke_unit("create_shop", &json!({ "name": name, "npcVnum": js_number_json(npc_vnum) })).await {
                Ok(()) => {
                    log_activity(
                        module_id(),
                        "create",
                        &format!("Shop '{name}' angelegt (NPC {npc_vnum})"),
                        Some("shop"),
                        None,
                    );
                    refresh_shops().await;
                    creating.set(false);
                    new_shop_name.set(String::new());
                    new_shop_npc_vnum.set(String::new());
                }
                Err(e) => error.set(Some(e)),
            }
        });
    };

    // ------------------------------------------------------------ Ansichten

    let connect_form = move || {
        view! {
            <div class="max-w-md space-y-4">
                <PageHeader module=MODULE title=Signal::derive(|| t("nav.shopEditor"))/>
                <p class="max-w-4xl text-sm text-muted-foreground">
                    "Verbinde dich mit der MySQL-Datenbank, um Shops zu bearbeiten."
                </p>
                <div class="space-y-2">
                    <input
                        prop:value=move || host.get()
                        on:input=move |e| host.set(event_target_value(&e))
                        placeholder="Host"
                        class="w-full rounded-md border border-border bg-background px-2 py-1 text-sm"
                    />
                    <input
                        prop:value=move || port.get()
                        on:input=move |e| port.set(event_target_value(&e))
                        placeholder="Port"
                        class="w-full rounded-md border border-border bg-background px-2 py-1 text-sm"
                    />
                    <input
                        prop:value=move || username.get()
                        on:input=move |e| username.set(event_target_value(&e))
                        placeholder="Benutzername"
                        class="w-full rounded-md border border-border bg-background px-2 py-1 text-sm"
                    />
                    <PasswordInput value=password placeholder="Passwort"/>
                </div>
                {move || conn_error.get().map(|e| view! { <p class="text-sm text-destructive">{e}</p> })}
                <Button
                    on_click=move |_| handle_connect()
                    disabled=Signal::derive(move || connecting.get() || host.with(|h| h.is_empty()) || username.with(|u| u.is_empty()))
                >
                    {move || if connecting.get() { "Verbinde…" } else { "Verbinden" }}
                </Button>
            </div>
        }
    };

    let placement = Memo::new(move |_| shop_items.with(|items| place_items(items, columns.get(), rows.get())));

    let icon_src = move |vnum: i64| icons.with(|i| i.get(&vnum).cloned().flatten());

    let shop_list = move || {
        let q = shop_search.get().to_lowercase();
        let sel = selected_shop.with(|s| s.as_ref().map(|s| s.vnum));
        shops
            .get()
            .into_iter()
            .filter(|s| s.name.to_lowercase().contains(&q))
            .map(|shop| {
                let class = format!(
                    "cursor-pointer rounded-md border border-border p-2 text-sm hover:bg-muted {}",
                    if sel == Some(shop.vnum) { "bg-muted" } else { "" }
                );
                let info = format!("ID: {} · NPC: {} · {} Gegenstände", shop.vnum, shop.npc_vnum, shop.item_count);
                let name = shop.name.clone();
                view! {
                    <div on:click=move |_| spawn_local(select_shop(shop.clone())) class=class>
                        <div class="font-medium">{name}</div>
                        <div class="text-xs text-muted-foreground">{info}</div>
                    </div>
                }
            })
            .collect_view()
    };

    let grid = move || {
        let p = placement.get();
        let placed = p
            .placed_items
            .into_iter()
            .map(|pi| {
                let item = pi.item.clone();
                let item_click = item.clone();
                let item_ctx = item.clone();
                let vnum = item.item_vnum;
                let title = format!("{} #{} (1x{})", item.item_name, item.item_vnum, item.size);
                let style = format!("grid-column: {}; grid-row: {} / span {}", pi.col, pi.row, pi.span);
                let alt = item.item_name.clone();
                view! {
                    <button
                        title=title
                        on:click=move |_| cell.set(Some(CellSelection::Filled(item_click.clone())))
                        on:contextmenu=move |e| {
                            e.prevent_default();
                            remove_item(item_ctx.clone());
                        }
                        style=style
                        class="relative flex w-12 flex-col items-center justify-center rounded-md border border-border bg-muted/40 hover:bg-muted"
                    >
                        {move || match icon_src(vnum) {
                            Some(src) => view! {
                                <img src=src alt=alt.clone() class="max-h-full w-8 object-contain [image-rendering:pixelated]"/>
                            }.into_any(),
                            None => view! { <span class="text-xs font-medium">{vnum}</span> }.into_any(),
                        }}
                        <span class="absolute bottom-0 right-0.5 text-[9px] leading-none text-muted-foreground">
                            {item.count}
                        </span>
                    </button>
                }
            })
            .collect_view();
        let empty = p
            .empty_cells
            .into_iter()
            .map(|(row, col)| {
                view! {
                    <button
                        on:click=move |_| cell.set(Some(CellSelection::Empty))
                        style=format!("grid-column: {col}; grid-row: {row}")
                        class="flex size-12 items-center justify-center rounded-md border border-dashed border-border text-muted-foreground hover:bg-muted/40"
                    >
                        <Icon name="Plus" class="size-4"/>
                    </button>
                }
            })
            .collect_view();
        (placed, empty)
    };

    let model_phase = Memo::new(move |_| {
        if npc_model_loading.get() {
            0
        } else if npc_model_error.with(|e| e.is_some()) {
            1
        } else {
            2
        }
    });
    let model_panel = move || match model_phase.get() {
        0 => view! {
            <div class="flex h-full items-center justify-center text-sm text-muted-foreground">"Lädt Modell…"</div>
        }
        .into_any(),
        1 => view! {
            <div class="flex h-full flex-col items-center justify-center gap-2 p-4 text-center">
                <Icon name="AlertTriangle" class="size-6 text-destructive"/>
                <p class="text-sm font-medium text-destructive">"Modell konnte nicht geladen werden"</p>
                <p class="text-xs text-muted-foreground">{move || npc_model_error.get()}</p>
            </div>
        }
        .into_any(),
        _ => view! { <Gr2Canvas model=npc_model class="h-full w-full"/> }.into_any(),
    };

    let selected_panel = move || {
        let sel_field = move |f: fn(&ShopSummary) -> String| move || selected_shop.with(|s| s.as_ref().map(f).unwrap_or_default());
        view! {
            <div class="flex h-full flex-col gap-3">
                <div class="flex items-start justify-between">
                    <div>
                        {move || if renaming.get() {
                            // `autoFocus` wie im TS: beim Einblenden fokussieren.
                            let rename_ref = NodeRef::<leptos::html::Input>::new();
                            rename_ref.on_load(|el| {
                                let _ = el.focus();
                            });
                            view! {
                                <input
                                    node_ref=rename_ref
                                    autofocus
                                    prop:value=move || name_draft.get()
                                    on:input=move |e| name_draft.set(event_target_value(&e))
                                    on:blur=move |_| save_rename()
                                    on:keydown=move |e| {
                                        if e.key() == "Enter" {
                                            save_rename()
                                        }
                                    }
                                    class="rounded-md border border-border bg-background px-2 py-1 text-xl font-semibold"
                                />
                            }
                            .into_any()
                        } else {
                            view! {
                                <h1
                                    class="cursor-text text-xl font-semibold"
                                    on:click=move |_| {
                                        name_draft.set(selected_shop.with_untracked(|s| s.as_ref().map(|s| s.name.clone()).unwrap_or_default()));
                                        renaming.set(true);
                                    }
                                >
                                    {sel_field(|s| s.name.clone())}
                                </h1>
                            }
                            .into_any()
                        }}
                        <p class="text-sm text-muted-foreground">
                            "ID: " {sel_field(|s| s.vnum.to_string())} " · NPC: " {sel_field(|s| s.npc_vnum.to_string())}
                            " · Gegenstände: " {move || shop_items.with(|i| i.len())}
                        </p>
                    </div>
                    <Button
                        variant=ButtonVariant::Destructive
                        on_click=move |_| delete_confirm.set(selected_shop.get_untracked())
                    >
                        <Icon name="Trash2" class="size-4"/>
                        "Shop löschen"
                    </Button>
                </div>

                <div class="flex flex-wrap items-center gap-4 rounded-md border border-border p-2 text-sm">
                    <div class="flex overflow-hidden rounded-md border border-border">
                        <button
                            on:click=move |_| mode.set(Mode::Global)
                            class=move || format!("px-3 py-1 {}", if mode.get() == Mode::Global { "bg-primary text-primary-foreground" } else { "" })
                        >
                            "Global"
                        </button>
                        <button
                            on:click=move |_| mode.set(Mode::ProShop)
                            class=move || format!("px-3 py-1 {}", if mode.get() == Mode::ProShop { "bg-primary text-primary-foreground" } else { "" })
                        >
                            "Pro Shop"
                        </button>
                    </div>
                    <Stepper label="Spalten" value=columns on_change=Callback::new(set_columns) min=1 max=MAX_SLOTS step=1/>
                    <Stepper label="Zeilen" value=rows on_change=Callback::new(set_rows) min=1 max=MAX_SLOTS step=1/>
                    <Stepper label="Max" value=max_value on_change=Callback::new(save_max_value) min=1 max=9999 step=10/>
                    <Button
                        variant=ButtonVariant::Outline
                        size=ButtonSize::Sm
                        on_click=move |_| {
                            sync_result.set(None);
                            sync_confirm.set(true);
                        }
                    >
                        <Icon name="RefreshCw" class="size-3.5"/>
                        "Bestandsware synchronisieren"
                    </Button>
                    {move || sync_result.get().map(|n| view! {
                        <span class="text-xs text-muted-foreground">{format!("{n} Gegenstände aktualisiert")}</span>
                    })}
                    <span class="text-xs text-muted-foreground">
                        {move || format!(
                            "{}/{} Slots (max. {MAX_SLOTS})",
                            placement.with(|p| p.used_slots),
                            columns.get() * rows.get()
                        )}
                    </span>
                    {move || {
                        let n = placement.with(|p| p.overflow_items.len());
                        (n > 0).then(|| view! {
                            <span class="flex items-center gap-1 text-xs text-destructive">
                                <Icon name="AlertTriangle" class="size-3.5"/>
                                {format!("{n} Gegenstand/Gegenstände passen nicht ins Raster")}
                            </span>
                        })
                    }}
                </div>

                <div class="flex min-h-0 flex-1 gap-3">
                    <div
                        class="grid content-start gap-1.5 overflow-y-auto"
                        style=move || format!(
                            "grid-template-columns: repeat({}, 48px); grid-template-rows: repeat({}, 48px)",
                            columns.get(),
                            rows.get()
                        )
                    >
                        {grid}
                    </div>

                    <div class="w-64 shrink-0 overflow-hidden rounded-lg border border-border">
                        {model_panel}
                    </div>
                </div>

                <p class="text-xs text-muted-foreground">
                    "Leere Zelle klicken zum Hinzufügen · Gegenstand klicken zum Bearbeiten · Rechtsklick zum Entfernen"
                </p>
            </div>
        }
    };

    let cell_modal = move || {
        cell.get().map(|c| {
            let content = match c {
                CellSelection::Filled(item) => {
                    let vnum = item.item_vnum;
                    let alt = item.item_name.clone();
                    let (minus, plus, remove) = (item.clone(), item.clone(), item.clone());
                    view! {
                        <div class="space-y-3">
                            <div class="flex items-center gap-2">
                                {move || icon_src(vnum).map(|src| view! {
                                    <img src=src alt=alt.clone() class="size-8 object-contain [image-rendering:pixelated]"/>
                                })}
                                <p class="text-sm font-medium">
                                    {item.item_name.clone()} " "
                                    <span class="text-muted-foreground">"#"{vnum}</span>
                                </p>
                            </div>
                            <div class="flex items-center gap-2">
                                <Button variant=ButtonVariant::Outline size=ButtonSize::IconSm on_click=move |_| change_count(minus.clone(), -1)>
                                    <Icon name="Minus" class="size-3.5"/>
                                </Button>
                                <span class="w-10 text-center">{item.count}</span>
                                <Button variant=ButtonVariant::Outline size=ButtonSize::IconSm on_click=move |_| change_count(plus.clone(), 1)>
                                    <Icon name="Plus" class="size-3.5"/>
                                </Button>
                            </div>
                            <Button variant=ButtonVariant::Destructive on_click=move |_| remove_item(remove.clone())>
                                "Entfernen"
                            </Button>
                        </div>
                    }
                    .into_any()
                }
                CellSelection::Empty => {
                    let leading: RowRenderer = Arc::new(move |r: &EntityRow| {
                        let vnum = r.vnum;
                        (move || icon_src(vnum).map(|src| view! {
                            <img src=src alt="" class="size-6 shrink-0 object-contain [image-rendering:pixelated]"/>
                        }))
                        .into_any()
                    });
                    view! {
                        <EntityBrowser
                            kind=EntityKind::Item
                            pick_label="Hinzufügen"
                            auto_focus=true
                            max_height_class="max-h-64"
                            on_pick=Callback::new(add_item)
                            on_rows_change=Callback::new(move |rows: Vec<EntityRow>| ensure_icons(rows.iter().map(|r| r.vnum).collect()))
                            render_leading=leading
                        />
                    }
                    .into_any()
                }
            };
            view! { <Modal on_close=Callback::new(move |_| cell.set(None))>{content}</Modal> }
        })
    };

    let create_modal = move || {
        creating.get().then(|| {
            view! {
                <Modal on_close=Callback::new(move |_| creating.set(false))>
                    <div class="space-y-3">
                        <p class="text-sm font-medium">"Neuen Shop anlegen"</p>
                        <input
                            prop:value=move || new_shop_name.get()
                            on:input=move |e| new_shop_name.set(event_target_value(&e))
                            placeholder="Shop-Name"
                            class="w-full rounded-md border border-border bg-background px-2 py-1 text-sm"
                        />
                        <input
                            prop:value=move || new_shop_npc_vnum.get()
                            on:input=move |e| new_shop_npc_vnum.set(event_target_value(&e))
                            placeholder="NPC-VNUM"
                            class="w-full rounded-md border border-border bg-background px-2 py-1 text-sm"
                        />
                        <Button
                            on_click=move |_| submit_new_shop()
                            disabled=Signal::derive(move || {
                                new_shop_name.with(|n| n.trim().is_empty()) || new_shop_npc_vnum.with(|v| v.is_empty())
                            })
                        >
                            "Erstellen"
                        </Button>
                    </div>
                </Modal>
            }
        })
    };

    let sync_dialog = move || {
        sync_confirm.get().then(|| {
            view! {
                <div class="fixed inset-0 flex items-center justify-center bg-black/50">
                    <div class="w-96 space-y-3 rounded-lg border border-border bg-card p-4">
                        <p class="text-sm">
                            "Alle vorhandenen Gegenstände "
                            {move || if mode.get() == Mode::ProShop {
                                view! {
                                    "in "
                                    <strong>{move || selected_shop.with(|s| s.as_ref().map(|s| s.name.clone()))}</strong>
                                }
                                .into_any()
                            } else {
                                "in allen Shops".into_any()
                            }}
                            " auf Stapelgröße " <strong>{move || max_value.get()}</strong>
                            " setzen? Das überschreibt die aktuellen Mengen und kann nicht rückgängig gemacht werden."
                        </p>
                        <div class="flex justify-end gap-2">
                            <Button
                                variant=ButtonVariant::Outline
                                on_click=move |_| sync_confirm.set(false)
                                disabled=syncing
                            >
                                {move || t("common.cancel")}
                            </Button>
                            <Button on_click=move |_| confirm_sync() disabled=syncing>
                                {move || if syncing.get() { "Synchronisiere…" } else { "Synchronisieren" }}
                            </Button>
                        </div>
                    </div>
                </div>
            }
        })
    };

    let delete_dialog = move || {
        delete_confirm.get().map(|d| {
            view! {
                <div class="fixed inset-0 flex items-center justify-center bg-black/50">
                    <div class="w-80 space-y-3 rounded-lg border border-border bg-card p-4">
                        <p class="text-sm">
                            "Shop " <strong>{d.name.clone()}</strong>
                            " wirklich löschen? Das kann nicht rückgängig gemacht werden."
                        </p>
                        <div class="flex justify-end gap-2">
                            <Button variant=ButtonVariant::Outline on_click=move |_| delete_confirm.set(None)>
                                {move || t("common.cancel")}
                            </Button>
                            <Button variant=ButtonVariant::Destructive on_click=move |_| confirm_delete()>
                                {move || t("common.delete")}
                            </Button>
                        </div>
                    </div>
                </div>
            }
        })
    };

    let main_view = move || {
        view! {
            <div class="flex h-full gap-4">
                <div class="flex w-72 shrink-0 flex-col gap-2">
                    <div class="flex gap-2">
                        <div class="relative flex-1">
                            <Icon name="Search" class="absolute left-2 top-1/2 size-3.5 -translate-y-1/2 text-muted-foreground"/>
                            <input
                                prop:value=move || shop_search.get()
                                on:input=move |e| shop_search.set(event_target_value(&e))
                                placeholder="Shops suchen…"
                                class="w-full rounded-md border border-border bg-background py-1 pl-7 pr-2 text-sm"
                            />
                        </div>
                        <Button variant=ButtonVariant::Outline size=ButtonSize::Icon on_click=move |_| creating.set(true)>
                            <Icon name="Plus" class="size-4"/>
                        </Button>
                        <Button
                            variant=ButtonVariant::Ghost
                            size=ButtonSize::Icon
                            title="Hilfe zu diesem Modul"
                            on_click=move |_| open_manual(Some(module_id()))
                        >
                            <Icon name="HelpCircle" class="size-4"/>
                        </Button>
                    </div>
                    <div class="flex-1 space-y-1 overflow-y-auto">{shop_list}</div>
                </div>

                <div class="min-w-0 flex-1">
                    {move || error.get().map(|e| view! { <p class="mb-2 text-sm text-destructive">{e}</p> })}
                    <Show when=move || selected_shop.with(|s| s.is_none())>
                        <p class="text-sm text-muted-foreground">"Wähle links einen Shop aus."</p>
                    </Show>
                    <Show when=move || selected_shop.with(|s| s.is_some())>{selected_panel}</Show>
                </div>

                {cell_modal}
                {create_modal}
                {sync_dialog}
                {delete_dialog}
            </div>
        }
    };

    let phase = Memo::new(move |_| {
        if checking_connection.get() {
            0
        } else if !connected.get() {
            1
        } else {
            2
        }
    });

    move || {
        let p = phase.get();
        untrack(move || match p {
            0 => view! { <p class="text-sm text-muted-foreground">"Prüfe Verbindung…"</p> }.into_any(),
            1 => connect_form().into_any(),
            _ => main_view().into_any(),
        })
    }
}

#[component]
fn Stepper(
    label: &'static str,
    value: RwSignal<i64>,
    on_change: Callback<i64>,
    min: i64,
    max: i64,
    step: i64,
) -> impl IntoView {
    view! {
        <div class="flex items-center gap-1">
            <span class="text-muted-foreground">{label} ":"</span>
            <Button
                variant=ButtonVariant::Outline
                size=ButtonSize::IconSm
                on_click=move |_| on_change.run((value.get_untracked() - step).max(min))
            >
                <Icon name="Minus" class="size-3"/>
            </Button>
            <span class="w-8 text-center">{move || value.get()}</span>
            <Button
                variant=ButtonVariant::Outline
                size=ButtonSize::IconSm
                on_click=move |_| on_change.run((value.get_untracked() + step).min(max))
            >
                <Icon name="Plus" class="size-3"/>
            </Button>
        </div>
    }
}

#[component]
fn Modal(on_close: Callback<()>, children: Children) -> impl IntoView {
    view! {
        <div class="fixed inset-0 flex items-center justify-center bg-black/50">
            <div class="w-96 rounded-lg border border-border bg-card p-4">
                <div class="mb-2 flex justify-end">
                    <button on:click=move |_| on_close.run(()) class="text-muted-foreground hover:text-foreground">
                        <Icon name="X" class="size-4"/>
                    </button>
                </div>
                {children()}
            </div>
        </div>
    }
}
