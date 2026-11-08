//! PORT-TODO: aus src/features/box-editor portieren.

use crate::components::PageHeader;
use crate::i18n::t;
use crate::store::navigation::EmbeddedModule;
use leptos::prelude::*;

#[component]
pub fn BoxEditor() -> impl IntoView {
    view! {
        <div class="space-y-4">
            <PageHeader module=EmbeddedModule::BoxEditor title=Signal::derive(|| t("nav.boxEditor"))/>
            <p class="text-sm text-muted-foreground">"Dieses Modul wird gerade auf Rust portiert."</p>
        </div>
    }
}
