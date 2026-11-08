//! PORT-TODO: aus src/features/drop-generator portieren.

use crate::components::PageHeader;
use crate::i18n::t;
use crate::store::navigation::Section;
use leptos::prelude::*;

#[component]
pub fn DropGenerator() -> impl IntoView {
    view! {
        <div class="space-y-4">
            <PageHeader module=Section::DropGenerator title=Signal::derive(|| t("nav.dropGenerator"))/>
            <p class="text-sm text-muted-foreground">"Dieses Modul wird gerade auf Rust portiert."</p>
        </div>
    }
}
