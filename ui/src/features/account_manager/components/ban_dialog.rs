use super::super::shared::{focus_on_mount, num_arg, AccountSummary, MODULE};
use crate::components::{Button, ButtonVariant, Icon};
use crate::lib::log_activity;
use crate::tauri::invoke_unit;
use leptos::prelude::*;
use leptos::task::spawn_local;
use serde_json::{json, Value};

// Sperrt einen Account mit einer frei formulierten Nachricht - die landet
// wörtlich als Fehlermeldung am Login-Screen des Spielers (verifiziert:
// der Login-Server vergleicht `status` nur auf "OK", jeder andere Wert wird
// unverändert an den Client geschickt). Die Dauer ist eine reine
// M2Manager-Zeitsteuerung (siehe bans.rs) - kein Server-Cron, wirkt nur
// wenn die App läuft.
#[component]
pub fn BanDialog(
    account: AccountSummary,
    #[prop(into)] on_close: Callback<()>,
    #[prop(into)] on_banned: Callback<()>,
) -> impl IntoView {
    let message = RwSignal::new("Gesperrt - bitte kontaktiere den Support.".to_string());
    let days = RwSignal::new(String::new());
    let busy = RwSignal::new(false);
    let error = RwSignal::new(None::<String>);
    let textarea_ref = NodeRef::<leptos::html::Textarea>::new();
    focus_on_mount(textarea_ref);

    let (id, login) = (account.id, account.login.clone());
    let submit = move || {
        let login = login.clone();
        let msg = message.get_untracked().trim().to_string();
        let d = days.get_untracked().trim().to_string();
        busy.set(true);
        error.set(None);
        spawn_local(async move {
            let days_arg = if d.is_empty() { Value::Null } else { num_arg(&d) };
            match invoke_unit(
                "ban_account",
                &json!({ "accountId": id, "login": login, "message": msg, "days": days_arg }),
            )
            .await
            {
                Ok(()) => {
                    let duration = if d.is_empty() { "dauerhaft".to_string() } else { format!("{d} Tag(e)") };
                    log_activity(
                        MODULE,
                        "ban",
                        &format!("Account '{login}' gesperrt ({duration}): {msg}"),
                        Some("account"),
                        Some(&id.to_string()),
                    );
                    on_banned.run(());
                }
                Err(e) => error.set(Some(e)),
            }
            busy.try_set(false);
        });
    };
    let submit = StoredValue::new(submit);

    view! {
        <div class="fixed inset-0 z-50 flex items-center justify-center bg-black/50">
            <div class="w-96 space-y-3 rounded-lg border border-border bg-card p-4">
                <p class="text-sm font-medium">"Account sperren: " {account.login.clone()}</p>
                <label class="flex flex-col gap-1 text-xs text-muted-foreground">
                    "Nachricht (wird wörtlich am Login-Bildschirm angezeigt)"
                    <textarea
                        node_ref=textarea_ref
                        prop:value=move || message.get()
                        on:input=move |e| message.set(event_target_value(&e))
                        rows=3
                        class="rounded-md border border-border bg-background px-2 py-1 text-sm"
                    ></textarea>
                </label>
                <label class="flex flex-col gap-1 text-xs text-muted-foreground">
                    "Dauer in Tagen (leer = dauerhaft, keine automatische Entsperrung)"
                    <input
                        type="number"
                        min=1
                        prop:value=move || days.get()
                        on:input=move |e| days.set(event_target_value(&e))
                        placeholder="dauerhaft"
                        class="w-32 rounded-md border border-border bg-background px-2 py-1 text-sm"
                    />
                </label>
                <p class="flex items-start gap-2 text-xs text-muted-foreground">
                    <Icon name="AlertTriangle" class="mt-0.5 size-3.5 shrink-0"/>
                    "Automatische Entsperrung greift nur, solange/wann immer M2Manager läuft - es gibt keinen Server-seitigen Cron dafür."
                </p>
                {move || error.get().map(|e| view! { <p class="text-sm text-destructive">{e}</p> })}
                <div class="flex justify-end gap-2">
                    <Button variant=ButtonVariant::Outline on_click=move |_| on_close.run(())>"Abbrechen"</Button>
                    <Button
                        variant=ButtonVariant::Destructive
                        disabled=Signal::derive(move || message.with(|m| m.trim().is_empty()) || busy.get())
                        on_click=move |_| submit.with_value(|f| f())
                    >
                        {move || if busy.get() { "Sperre…" } else { "Sperren" }}
                    </Button>
                </div>
            </div>
        </div>
    }
}
