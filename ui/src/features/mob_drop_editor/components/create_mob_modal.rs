use super::super::types::MobDropGroup;
use super::shared::{js_number, Field, Modal};
use crate::components::Button;
use crate::tauri::invoke;
use leptos::prelude::*;
use leptos::task::spawn_local;
use serde_json::json;

#[component]
pub fn CreateMobModal(#[prop(into)] on_close: Callback<()>, #[prop(into)] on_create: Callback<MobDropGroup>) -> impl IntoView {
    let name = RwSignal::new(String::new());
    let name_preview = RwSignal::new(String::new());
    let mob_vnum = RwSignal::new(String::new());
    let drop_type = RwSignal::new("drop".to_string());

    Effect::new(move |_| {
        let n = name.get();
        if n.trim().is_empty() {
            name_preview.set(String::new());
            return;
        }
        spawn_local(async move {
            match invoke::<String>("sanitize_mob_drop_group_name", &json!({ "name": n })).await {
                Ok(p) => name_preview.set(p),
                Err(_) => name_preview.set(String::new()),
            }
        });
    });

    let submit = move || {
        let vnum = js_number(&mob_vnum.get_untracked());
        let preview = name_preview.get_untracked();
        if preview.is_empty() || !vnum.is_finite() || vnum <= 0.0 {
            return;
        }
        let t = drop_type.get_untracked();
        let t = t.trim();
        on_create.run(MobDropGroup {
            name: preview,
            mob_vnum: vnum,
            drop_type: if t.is_empty() { "drop".into() } else { t.to_string() },
            items: vec![],
        });
    };

    view! {
        <Modal on_close=on_close>
            <div class="space-y-3">
                <p class="text-sm font-medium">"Neuen Mob-Eintrag anlegen"</p>
                <Field label="Name">
                    <input
                        autofocus
                        prop:value=move || name.get()
                        on:input=move |e| name.set(event_target_value(&e))
                        placeholder="z.B. Wildhund"
                        class="w-full rounded-md border border-border bg-background px-2 py-1 text-sm"
                    />
                </Field>
                <Show when=move || !name.with(|n| n.trim().is_empty())>
                    <p class="text-xs text-muted-foreground">
                        "Wird gespeichert als: "
                        <code>{move || {
                            let p = name_preview.get();
                            if p.is_empty() { "…".to_string() } else { p }
                        }}</code>
                    </p>
                </Show>
                <Field label="Mob-VNUM">
                    <input
                        type="number"
                        prop:value=move || mob_vnum.get()
                        on:input=move |e| mob_vnum.set(event_target_value(&e))
                        class="w-full rounded-md border border-border bg-background px-2 py-1 text-sm"
                    />
                </Field>
                <Field label="Type">
                    <input
                        prop:value=move || drop_type.get()
                        on:input=move |e| drop_type.set(event_target_value(&e))
                        class="w-full rounded-md border border-border bg-background px-2 py-1 text-sm"
                    />
                </Field>
                <Button
                    on_click=move |_| submit()
                    disabled=Signal::derive(move || name_preview.with(|p| p.is_empty()) || mob_vnum.with(|v| v.is_empty()))
                >
                    "Anlegen"
                </Button>
            </div>
        </Modal>
    }
}
