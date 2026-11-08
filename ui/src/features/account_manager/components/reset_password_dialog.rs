use super::super::shared::{focus_on_mount, AccountSummary, MODULE};
use crate::components::{Button, ButtonVariant, Icon};
use crate::util::log_activity;
use crate::tauri::invoke_unit;
use leptos::prelude::*;
use leptos::task::spawn_local;
use serde_json::json;

// Passwörter sind ein Einweg-Hash (MySQL PASSWORD()) - live gegen echte
// Accounts verifiziert (41-Zeichen "*..."-Hash), nicht umkehrbar. "Auslesen"
// ist daher technisch unmöglich; stattdessen wird ein neues Passwort
// gesetzt, genau wie es der echte Login-Check erwartet.
#[component]
pub fn ResetPasswordDialog(account: AccountSummary, #[prop(into)] on_close: Callback<()>) -> impl IntoView {
    let password = RwSignal::new(String::new());
    let busy = RwSignal::new(false);
    let error = RwSignal::new(None::<String>);
    let ok = RwSignal::new(false);
    let input_ref = NodeRef::<leptos::html::Input>::new();
    focus_on_mount(input_ref);

    let (id, login) = (account.id, account.login.clone());
    let submit = move || {
        let login = login.clone();
        let pw = password.get_untracked();
        busy.set(true);
        error.set(None);
        spawn_local(async move {
            match invoke_unit("reset_account_password", &json!({ "id": id, "newPassword": pw })).await {
                Ok(()) => {
                    ok.set(true);
                    log_activity(
                        MODULE,
                        "reset-password",
                        &format!("Passwort für Account '{login}' zurückgesetzt"),
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
            <div class="w-96 space-y-3 rounded-lg border border-border bg-card p-4">
                <p class="text-sm font-medium">"Passwort zurücksetzen: " {account.login.clone()}</p>
                <p class="text-xs text-muted-foreground">
                    "Passwörter sind als Einweg-Hash gespeichert und können nicht ausgelesen werden - hier lässt sich nur ein neues setzen."
                </p>
                <label class="flex flex-col gap-1 text-xs text-muted-foreground">
                    "Neues Passwort"
                    <input
                        node_ref=input_ref
                        type="text"
                        prop:value=move || password.get()
                        on:input=move |e| password.set(event_target_value(&e))
                        class="rounded-md border border-border bg-background px-2 py-1 text-sm"
                    />
                </label>
                {move || error.get().map(|e| view! { <p class="text-sm text-destructive">{e}</p> })}
                <Show when=move || ok.get()>
                    <p class="flex items-center gap-1 text-sm text-green-600">
                        <Icon name="CheckCircle2" class="size-4"/>
                        " Passwort gesetzt."
                    </p>
                </Show>
                <div class="flex justify-end gap-2">
                    <Button variant=ButtonVariant::Outline on_click=move |_| on_close.run(())>
                        {move || if ok.get() { "Schließen" } else { "Abbrechen" }}
                    </Button>
                    <Show when=move || !ok.get()>
                        <Button
                            disabled=Signal::derive(move || password.with(|p| p.is_empty()) || busy.get())
                            on_click=move |_| submit.with_value(|f| f())
                        >
                            {move || if busy.get() { "Setze…" } else { "Setzen" }}
                        </Button>
                    </Show>
                </div>
            </div>
        </div>
    }
}
