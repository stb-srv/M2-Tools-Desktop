//! PORT-TODO: aus src/features/icon-browser portieren.

use crate::components::PageHeader;
use crate::i18n::t;
use crate::store::navigation::Section;
use leptos::prelude::*;

#[component]
pub fn IconBrowser() -> impl IntoView {
    view! {
        <div class="space-y-4">
            <PageHeader module=Section::IconBrowser title=Signal::derive(|| t("nav.iconBrowser"))/>
            <p class="text-sm text-muted-foreground">"Dieses Modul wird gerade auf Rust portiert."</p>
        </div>
    }
}
