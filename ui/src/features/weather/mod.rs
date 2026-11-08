//! PORT-TODO: aus src/features/weather portieren.

use crate::components::PageHeader;
use crate::i18n::t;
use crate::store::navigation::EmbeddedModule;
use leptos::prelude::*;

#[component]
pub fn WeatherControl() -> impl IntoView {
    view! {
        <div class="space-y-4">
            <PageHeader module=EmbeddedModule::WeatherControl title=Signal::derive(|| t("nav.weatherControl"))/>
            <p class="text-sm text-muted-foreground">"Dieses Modul wird gerade auf Rust portiert."</p>
        </div>
    }
}
