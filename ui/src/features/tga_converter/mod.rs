//! PORT-TODO: aus src/features/tga-converter portieren.

use crate::components::PageHeader;
use crate::i18n::t;
use crate::store::navigation::Section;
use leptos::prelude::*;

#[component]
pub fn TgaConverter() -> impl IntoView {
    view! {
        <div class="space-y-4">
            <PageHeader module=Section::TgaConverter title=Signal::derive(|| t("nav.tgaConverter"))/>
            <p class="text-sm text-muted-foreground">"Dieses Modul wird gerade auf Rust portiert."</p>
        </div>
    }
}
