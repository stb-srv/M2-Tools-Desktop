//! PORT-TODO: aus src/features/mob-proto-editor portieren.

use crate::components::PageHeader;
use crate::i18n::t;
use crate::store::navigation::Section;
use leptos::prelude::*;

#[component]
pub fn MobProtoEditor() -> impl IntoView {
    view! {
        <div class="space-y-4">
            <PageHeader module=Section::MobProtoEditor title=Signal::derive(|| t("nav.mobProtoEditor"))/>
            <p class="text-sm text-muted-foreground">"Dieses Modul wird gerade auf Rust portiert."</p>
        </div>
    }
}
