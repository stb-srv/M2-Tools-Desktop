//! Kleine Formular-Bausteine des Quest-Dialogs (aus components/shared.tsx).

use crate::components::{Button, ButtonSize, ButtonVariant, Icon};
use leptos::prelude::*;

#[component]
pub fn Field(#[prop(into)] label: Signal<String>, children: Children) -> impl IntoView {
    view! {
        <label class="flex flex-col gap-1 text-xs text-muted-foreground">
            {move || label.get()}
            {children()}
        </label>
    }
}

#[component]
pub fn VnumInput(
    #[prop(into)] value: Signal<String>,
    #[prop(into)] on_change: Callback<String>,
    #[prop(into)] on_pick: Callback<()>,
) -> impl IntoView {
    view! {
        <div class="flex gap-1">
            <input
                type="number"
                prop:value=move || value.get()
                on:input=move |e| on_change.run(event_target_value(&e))
                class="w-24 rounded-md border border-border bg-background px-2 py-1 text-sm"
            />
            <Button variant=ButtonVariant::Outline size=ButtonSize::IconSm on_click=move |_| on_pick.run(()) title="Suchen…">
                <Icon name="Search" class="size-3.5"/>
            </Button>
        </div>
    }
}

#[component]
pub fn Modal(#[prop(into)] on_close: Callback<()>, #[prop(optional)] wide: bool, children: Children) -> impl IntoView {
    view! {
        <div class="fixed inset-0 flex items-center justify-center bg-black/50">
            <div class=if wide { "rounded-lg border border-border bg-card p-4 w-[48rem]" } else { "rounded-lg border border-border bg-card p-4 w-96" }>
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
