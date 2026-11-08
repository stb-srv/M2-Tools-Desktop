//! PORT-TODO: aus src/features/locale-editor portieren.

use crate::components::PageHeader;
use crate::i18n::t;
use crate::store::navigation::Section;
use leptos::prelude::*;

#[component]
pub fn LocaleEditor() -> impl IntoView {
    view! {
        <div class="space-y-4">
            <PageHeader module=Section::LocaleEditor title=Signal::derive(|| t("nav.localeEditor"))/>
            <p class="text-sm text-muted-foreground">"Dieses Modul wird gerade auf Rust portiert."</p>
        </div>
    }
}
