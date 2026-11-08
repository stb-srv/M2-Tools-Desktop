//! PORT-TODO: aus src/features/db-explorer portieren.

use crate::components::PageHeader;
use crate::i18n::t;
use crate::store::navigation::Section;
use leptos::prelude::*;

#[component]
pub fn DbExplorer() -> impl IntoView {
    view! {
        <div class="space-y-4">
            <PageHeader module=Section::DbExplorer title=Signal::derive(|| t("nav.dbExplorer"))/>
            <p class="text-sm text-muted-foreground">"Dieses Modul wird gerade auf Rust portiert."</p>
        </div>
    }
}
