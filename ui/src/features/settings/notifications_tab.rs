use super::shared::{load_setting, save_setting, save_setting_with_feedback, ConnStatusIcon, SettingRow, SettingsCard, TestState};
use crate::components::{Button, ButtonSize, ButtonVariant};
use crate::tauri::invoke_unit;
use leptos::prelude::*;
use leptos::task::spawn_local;
use serde_json::json;

#[component]
pub fn NotificationsTab() -> impl IntoView {
    let webhook_url = RwSignal::new(String::new());
    let stored = RwSignal::new(String::new());
    let test_state = RwSignal::new(TestState::Idle);
    let test_error = RwSignal::new(None::<String>);

    spawn_local(async move {
        if let Ok(v) = load_setting("webhook_url").await {
            let v = v.unwrap_or_default();
            webhook_url.set(v.clone());
            stored.set(v);
        }
    });

    let commit = move |_| {
        let value = webhook_url.get_untracked().trim().to_string();
        if value == stored.get_untracked() {
            return;
        }
        spawn_local(async move {
            if save_setting_with_feedback("webhook_url", &value, "Webhook-URL").await {
                stored.set(value);
            }
        });
    };

    let test_webhook = move |_| {
        test_state.set(TestState::Testing);
        test_error.set(None);
        let url = webhook_url.get_untracked().trim().to_string();
        spawn_local(async move {
            let res: Result<(), String> = async {
                save_setting("webhook_url", &url).await?;
                stored.set(url.clone());
                invoke_unit("send_test_webhook", &json!({})).await
            }
            .await;
            match res {
                Ok(()) => test_state.set(TestState::Ok),
                Err(e) => {
                    test_state.set(TestState::Error);
                    test_error.set(Some(e));
                }
            }
        });
    };

    view! {
        <div class="space-y-4">
            <SettingsCard title="Webhook (Discord o.ä.)".to_string() icon="Webhook">
                <SettingRow
                    label="Webhook-URL".to_string()
                    hint=Some("Meldet fehlgeschlagene Datenbank-Backups und (solange die App offen ist) wenn die überwachten Server-Prozesse verschwinden. Leer lassen schaltet Benachrichtigungen ab.")
                >
                    <input
                        prop:value=move || webhook_url.get()
                        on:input=move |e| webhook_url.set(event_target_value(&e))
                        on:blur=commit
                        placeholder="https://discord.com/api/webhooks/…"
                        class="w-full max-w-2xl rounded-md border border-border bg-background px-3 py-2 text-sm"
                    />
                    <div class="flex items-center gap-2 pt-1">
                        <Button
                            variant=ButtonVariant::Outline
                            size=ButtonSize::Sm
                            on_click=test_webhook
                            disabled=Signal::derive(move || {
                                webhook_url.with(|u| u.trim().is_empty()) || test_state.get() == TestState::Testing
                            })
                        >
                            {move || if test_state.get() == TestState::Testing { "Sende…" } else { "Testnachricht senden" }}
                        </Button>
                        <ConnStatusIcon state=test_state/>
                    </div>
                    {move || test_error.get().map(|e| view! { <p class="text-sm text-destructive">{e}</p> })}
                </SettingRow>
            </SettingsCard>
        </div>
    }
}
