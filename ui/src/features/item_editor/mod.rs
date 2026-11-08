//! PORT-TODO: aus src/features/item-editor portieren.

use crate::components::PageHeader;
use crate::i18n::t;
use crate::store::navigation::Section;
use leptos::prelude::*;

#[component]
pub fn ItemEditor() -> impl IntoView {
    view! {
        <div class="space-y-4">
            <PageHeader module=Section::ItemEditor title=Signal::derive(|| t("nav.itemEditor"))/>
            <p class="text-sm text-muted-foreground">"Dieses Modul wird gerade auf Rust portiert."</p>
        </div>
    }
}
