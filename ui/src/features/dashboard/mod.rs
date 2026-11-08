//! PORT-TODO: aus src/features/dashboard portieren.

use crate::components::PageHeader;
use crate::i18n::t;
use crate::store::navigation::Section;
use leptos::prelude::*;

#[component]
pub fn Dashboard() -> impl IntoView {
    view! {
        <div class="space-y-4">
            <PageHeader module=Section::Dashboard title=Signal::derive(|| t("nav.dashboard"))/>
            <p class="text-sm text-muted-foreground">"Dieses Modul wird gerade auf Rust portiert."</p>
        </div>
    }
}
