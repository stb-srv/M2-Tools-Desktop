//! PORT-TODO: aus src/features/settings portieren.

use crate::components::PageHeader;
use crate::i18n::t;
use crate::store::navigation::Section;
use leptos::prelude::*;

#[component]
pub fn Settings() -> impl IntoView {
    view! {
        <div class="space-y-4">
            <PageHeader module=Section::Settings title=Signal::derive(|| t("nav.settings"))/>
            <p class="text-sm text-muted-foreground">"Dieses Modul wird gerade auf Rust portiert."</p>
        </div>
    }
}
