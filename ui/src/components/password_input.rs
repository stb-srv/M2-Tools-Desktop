use super::Icon;
use leptos::prelude::*;

/// Passwortfeld mit Auge-Knopf zum Ein-/Ausblenden.
#[component]
pub fn PasswordInput(
    value: RwSignal<String>,
    #[prop(into, optional)] placeholder: MaybeProp<String>,
    #[prop(into, optional)] class: MaybeProp<String>,
) -> impl IntoView {
    let visible = RwSignal::new(false);
    view! {
        <div class=move || format!("relative {}", class.get().unwrap_or_default())>
            <input
                type=move || if visible.get() { "text" } else { "password" }
                prop:value=move || value.get()
                on:input=move |e| value.set(event_target_value(&e))
                placeholder=move || placeholder.get()
                class="w-full rounded-md border border-border bg-background px-3 py-2 pr-8 text-sm"
            />
            <button
                type="button"
                tabindex="-1"
                on:click=move |_| visible.update(|v| *v = !*v)
                class="absolute right-2 top-1/2 -translate-y-1/2 text-muted-foreground hover:text-foreground"
            >
                <Icon name=Signal::derive(move || if visible.get() { "EyeOff" } else { "Eye" }) class="size-3.5"/>
            </button>
        </div>
    }
}
