//! PORT-TODO: aus src/features/broadcast portieren.

use crate::components::PageHeader;
use crate::i18n::t;
use crate::store::navigation::Section;
use leptos::prelude::*;

#[component]
pub fn BroadcastSystem() -> impl IntoView {
    view! {
        <div class="space-y-4">
            <PageHeader module=Section::BroadcastSystem title=Signal::derive(|| t("nav.broadcastSystem"))/>
            <p class="text-sm text-muted-foreground">"Dieses Modul wird gerade auf Rust portiert."</p>
        </div>
    }
}
