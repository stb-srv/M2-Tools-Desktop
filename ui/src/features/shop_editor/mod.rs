//! PORT-TODO: aus src/features/shop-editor portieren.

use crate::components::PageHeader;
use crate::i18n::t;
use crate::store::navigation::Section;
use leptos::prelude::*;

#[component]
pub fn ShopEditor() -> impl IntoView {
    view! {
        <div class="space-y-4">
            <PageHeader module=Section::ShopEditor title=Signal::derive(|| t("nav.shopEditor"))/>
            <p class="text-sm text-muted-foreground">"Dieses Modul wird gerade auf Rust portiert."</p>
        </div>
    }
}
