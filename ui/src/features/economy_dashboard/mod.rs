//! PORT-TODO: aus src/features/economy-dashboard portieren.

use crate::components::PageHeader;
use crate::i18n::t;
use crate::store::navigation::Section;
use leptos::prelude::*;

#[component]
pub fn EconomyDashboard() -> impl IntoView {
    view! {
        <div class="space-y-4">
            <PageHeader module=Section::EconomyDashboard title=Signal::derive(|| t("nav.economyDashboard"))/>
            <p class="text-sm text-muted-foreground">"Dieses Modul wird gerade auf Rust portiert."</p>
        </div>
    }
}
