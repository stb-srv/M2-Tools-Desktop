//! PORT-TODO: Handbuch-Fenster aus src/features/manual/Manual.tsx portieren.
//! Wird als eigenes Fenster mit `index.html?view=manual#<modul-id>` geöffnet.

use leptos::prelude::*;

#[component]
pub fn Manual() -> impl IntoView {
    view! { <div class="p-6 text-sm">"Handbuch wird portiert."</div> }
}
