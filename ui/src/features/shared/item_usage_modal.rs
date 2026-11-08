//! "Wo wird dieses Item verwendet?" - Shops, Drops, Kisten, Cube, Quests.
//! PORT-TODO (Paket Items): Inhalt aus src/features/shared/ItemUsageModal.tsx übernehmen.

use leptos::prelude::*;

#[component]
pub fn ItemUsageModal(vnum: i64, #[prop(into)] on_close: Callback<()>) -> impl IntoView {
    view! {
        <div class="fixed inset-0 z-50 flex items-center justify-center bg-black/50">
            <div class="rounded-lg border border-border bg-card p-4 text-sm">
                "Verwendung von Item #" {vnum} " - wird portiert."
                <button class="ml-4 underline" on:click=move |_| on_close.run(())>"Schließen"</button>
            </div>
        </div>
    }
}
