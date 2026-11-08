//! PORT-TODO: aus src/features/regen-editor portieren.

use crate::components::PageHeader;
use crate::i18n::t;
use crate::store::navigation::Section;
use leptos::prelude::*;

#[component]
pub fn RegenEditor() -> impl IntoView {
    view! {
        <div class="space-y-4">
            <PageHeader module=Section::RegenEditor title=Signal::derive(|| t("nav.regenEditor"))/>
            <p class="text-sm text-muted-foreground">"Dieses Modul wird gerade auf Rust portiert."</p>
        </div>
    }
}
