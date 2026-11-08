//! PORT-TODO: aus src/features/server-control portieren.

use crate::components::PageHeader;
use crate::i18n::t;
use crate::store::navigation::Section;
use leptos::prelude::*;

#[component]
pub fn ServerControl() -> impl IntoView {
    view! {
        <div class="space-y-4">
            <PageHeader module=Section::ServerControl title=Signal::derive(|| t("nav.serverControl"))/>
            <p class="text-sm text-muted-foreground">"Dieses Modul wird gerade auf Rust portiert."</p>
        </div>
    }
}
