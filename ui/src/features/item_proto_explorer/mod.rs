//! PORT-TODO: aus src/features/item-proto-explorer portieren.

use crate::components::PageHeader;
use crate::i18n::t;
use crate::store::navigation::Section;
use leptos::prelude::*;

#[component]
pub fn ItemProtoExplorer() -> impl IntoView {
    view! {
        <div class="space-y-4">
            <PageHeader module=Section::ItemProtoExplorer title=Signal::derive(|| t("nav.itemProtoExplorer"))/>
            <p class="text-sm text-muted-foreground">"Dieses Modul wird gerade auf Rust portiert."</p>
        </div>
    }
}
