//! PORT-TODO: aus src/features/activity-log portieren.

use crate::components::PageHeader;
use crate::i18n::t;
use crate::store::navigation::Section;
use leptos::prelude::*;

#[component]
pub fn ActivityLog() -> impl IntoView {
    view! {
        <div class="space-y-4">
            <PageHeader module=Section::ActivityLog title=Signal::derive(|| t("nav.activityLog"))/>
            <p class="text-sm text-muted-foreground">"Dieses Modul wird gerade auf Rust portiert."</p>
        </div>
    }
}
