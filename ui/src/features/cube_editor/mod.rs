//! PORT-TODO: aus src/features/cube-editor portieren.

use crate::components::PageHeader;
use crate::i18n::t;
use crate::store::navigation::Section;
use leptos::prelude::*;

#[component]
pub fn CubeEditor() -> impl IntoView {
    view! {
        <div class="space-y-4">
            <PageHeader module=Section::CubeEditor title=Signal::derive(|| t("nav.cubeEditor"))/>
            <p class="text-sm text-muted-foreground">"Dieses Modul wird gerade auf Rust portiert."</p>
        </div>
    }
}
