//! PORT-TODO: aus src/features/module-importer portieren.

use crate::components::PageHeader;
use crate::i18n::t;
use crate::store::navigation::Section;
use leptos::prelude::*;

#[component]
pub fn ModuleImporter() -> impl IntoView {
    view! {
        <div class="space-y-4">
            <PageHeader module=Section::ModuleImporter title=Signal::derive(|| t("nav.moduleImporter"))/>
            <p class="text-sm text-muted-foreground">"Dieses Modul wird gerade auf Rust portiert."</p>
        </div>
    }
}
