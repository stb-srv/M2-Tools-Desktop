//! PORT-TODO: aus src/features/gm-manager portieren.

use crate::components::PageHeader;
use crate::i18n::t;
use crate::store::navigation::EmbeddedModule;
use leptos::prelude::*;

#[component]
pub fn GmManager() -> impl IntoView {
    view! {
        <div class="space-y-4">
            <PageHeader module=EmbeddedModule::GmManager title=Signal::derive(|| t("nav.gmManager"))/>
            <p class="text-sm text-muted-foreground">"Dieses Modul wird gerade auf Rust portiert."</p>
        </div>
    }
}
