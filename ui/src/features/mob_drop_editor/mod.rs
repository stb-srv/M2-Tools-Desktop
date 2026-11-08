//! PORT-TODO: aus src/features/mob-drop-editor portieren.

use crate::components::PageHeader;
use crate::i18n::t;
use crate::store::navigation::EmbeddedModule;
use leptos::prelude::*;

#[component]
pub fn MobDropEditor() -> impl IntoView {
    view! {
        <div class="space-y-4">
            <PageHeader module=EmbeddedModule::MobDropEditor title=Signal::derive(|| t("nav.mobDropEditor"))/>
            <p class="text-sm text-muted-foreground">"Dieses Modul wird gerade auf Rust portiert."</p>
        </div>
    }
}
