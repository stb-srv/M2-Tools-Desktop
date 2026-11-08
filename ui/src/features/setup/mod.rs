//! PORT-TODO: aus src/features/setup/SetupWizard.tsx portieren.

use leptos::prelude::*;

/// Erststart-Assistent. `on_complete` wird nach erfolgreicher Einrichtung aufgerufen.
#[component]
pub fn SetupWizard(#[prop(into)] on_complete: Callback<()>) -> impl IntoView {
    view! {
        <div class="flex h-screen items-center justify-center bg-background text-foreground">
            <div class="space-y-3 text-center">
                <p>"Einrichtungsassistent wird portiert."</p>
                <button class="underline" on:click=move |_| on_complete.run(())>"Überspringen"</button>
            </div>
        </div>
    }
}
