use super::super::shared::{focus_on_mount, num_arg, AccountSummary, ColumnInfo, MODULE};
use crate::components::{Button, ButtonVariant, Icon};
use crate::util::log_activity;
use crate::tauri::invoke;
use leptos::prelude::*;
use leptos::task::spawn_local;
use serde_json::json;

// Additive Guthaben-Anpassung einer numerischen account.account-Spalte, die
// zur Laufzeit real gefunden wurde (siehe AccountSection - keine geratenen
// Spaltennamen wie "coins"/"cash", das Backend validiert zusätzlich erneut
// gegen die echte Spaltenliste).
#[component]
pub fn CurrencyDialog(account: AccountSummary, columns: Vec<ColumnInfo>, #[prop(into)] on_close: Callback<()>) -> impl IntoView {
    let column = RwSignal::new(columns.first().map(|c| c.name.clone()).unwrap_or_default());
    let delta = RwSignal::new(String::new());
    let busy = RwSignal::new(false);
    let error = RwSignal::new(None::<String>);
    let new_value = RwSignal::new(None::<i64>);
    let input_ref = NodeRef::<leptos::html::Input>::new();
    focus_on_mount(input_ref);

    let (id, login) = (account.id, account.login.clone());
    let submit = move || {
        let col = column.get_untracked();
        let d = delta.get_untracked();
        if col.is_empty() || d.trim().is_empty() {
            return;
        }
        let login = login.clone();
        busy.set(true);
        error.set(None);
        new_value.set(None);
        spawn_local(async move {
            match invoke::<i64>(
                "adjust_account_numeric_column",
                &json!({ "accountId": id, "column": col, "delta": num_arg(&d) }),
            )
            .await
            {
                Ok(v) => {
                    new_value.set(Some(v));
                    log_activity(
                        MODULE,
                        "adjust-currency",
                        &format!("Guthaben-Spalte '{col}' für Account '{login}' um {d} angepasst (neu: {v})"),
                        Some("account"),
                        Some(&id.to_string()),
                    );
                }
                Err(e) => error.set(Some(e)),
            }
            busy.set(false);
        });
    };
    let submit = StoredValue::new(submit);

    view! {
        <div class="fixed inset-0 z-50 flex items-center justify-center bg-black/50">
            <div class="w-80 space-y-3 rounded-lg border border-border bg-card p-4">
                <p class="text-sm font-medium">"Guthaben anpassen: " {account.login.clone()}</p>
                <label class="flex flex-col gap-1 text-xs text-muted-foreground">
                    "Spalte"
                    <select
                        prop:value=move || column.get()
                        on:change=move |e| column.set(event_target_value(&e))
                        class="rounded-md border border-border bg-background px-2 py-1 text-sm"
                    >
                        {columns.iter().map(|c| {
                            let name = c.name.clone();
                            let (n2, value) = (name.clone(), name.clone());
                            view! { <option value=value selected=move || column.with(|v| *v == n2)>{name}</option> }
                        }).collect_view()}
                    </select>
                </label>
                <label class="flex flex-col gap-1 text-xs text-muted-foreground">
                    "Änderung (negativ zum Abziehen)"
                    <input
                        node_ref=input_ref
                        type="number"
                        prop:value=move || delta.get()
                        on:input=move |e| delta.set(event_target_value(&e))
                        class="rounded-md border border-border bg-background px-2 py-1 text-sm"
                    />
                </label>
                {move || error.get().map(|e| view! { <p class="text-sm text-destructive">{e}</p> })}
                {move || new_value.get().map(|v| view! {
                    <p class="flex items-center gap-1 text-sm text-green-600">
                        <Icon name="CheckCircle2" class="size-4"/>
                        " Neuer Wert: "
                        {v}
                    </p>
                })}
                <div class="flex justify-end gap-2">
                    <Button variant=ButtonVariant::Outline on_click=move |_| on_close.run(())>
                        {move || if new_value.with(|v| v.is_some()) { "Schließen" } else { "Abbrechen" }}
                    </Button>
                    <Show when=move || new_value.with(|v| v.is_none())>
                        <Button
                            disabled=Signal::derive(move || {
                                column.with(|c| c.is_empty()) || delta.with(|d| d.trim().is_empty()) || busy.get()
                            })
                            on_click=move |_| submit.with_value(|f| f())
                        >
                            {move || if busy.get() { "Speichere…" } else { "Anwenden" }}
                        </Button>
                    </Show>
                </div>
            </div>
        </div>
    }
}
