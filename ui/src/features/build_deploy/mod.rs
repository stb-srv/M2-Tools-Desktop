//! PORT-TODO: aus src/features/build-deploy portieren.

use crate::components::PageHeader;
use crate::i18n::t;
use crate::store::navigation::Section;
use leptos::prelude::*;

#[component]
pub fn BuildDeploy() -> impl IntoView {
    view! {
        <div class="space-y-4">
            <PageHeader module=Section::BuildDeploy title=Signal::derive(|| t("nav.buildDeploy"))/>
            <p class="text-sm text-muted-foreground">"Dieses Modul wird gerade auf Rust portiert."</p>
        </div>
    }
}
