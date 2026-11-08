//! PORT-TODO: aus src/features/server-events portieren.

use crate::components::PageHeader;
use crate::i18n::t;
use crate::store::navigation::Section;
use leptos::prelude::*;

#[component]
pub fn ServerEventsPage() -> impl IntoView {
    view! {
        <div class="space-y-4">
            <PageHeader module=Section::ServerEvents title=Signal::derive(|| t("nav.serverEvents"))/>
            <p class="text-sm text-muted-foreground">"Dieses Modul wird gerade auf Rust portiert."</p>
        </div>
    }
}
