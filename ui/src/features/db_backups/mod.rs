//! PORT-TODO: aus src/features/db-backups portieren.

use crate::components::PageHeader;
use crate::i18n::t;
use crate::store::navigation::EmbeddedModule;
use leptos::prelude::*;

#[component]
pub fn DbBackups() -> impl IntoView {
    view! {
        <div class="space-y-4">
            <PageHeader module=EmbeddedModule::DbBackups title=Signal::derive(|| t("nav.dbBackups"))/>
            <p class="text-sm text-muted-foreground">"Dieses Modul wird gerade auf Rust portiert."</p>
        </div>
    }
}
