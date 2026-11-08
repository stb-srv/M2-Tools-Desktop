//! PORT-TODO: aus src/features/model-viewer portieren.

use crate::components::PageHeader;
use crate::i18n::t;
use crate::store::navigation::Section;
use leptos::prelude::*;

#[component]
pub fn ModelViewer() -> impl IntoView {
    view! {
        <div class="space-y-4">
            <PageHeader module=Section::ModelViewer title=Signal::derive(|| t("nav.modelViewer"))/>
            <p class="text-sm text-muted-foreground">"Dieses Modul wird gerade auf Rust portiert."</p>
        </div>
    }
}
