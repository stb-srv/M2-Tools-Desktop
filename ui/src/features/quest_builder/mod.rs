//! PORT-TODO: aus src/features/quest-builder portieren.

use crate::components::PageHeader;
use crate::i18n::t;
use crate::store::navigation::Section;
use leptos::prelude::*;

#[component]
pub fn QuestBuilder() -> impl IntoView {
    view! {
        <div class="space-y-4">
            <PageHeader module=Section::QuestBuilder title=Signal::derive(|| t("nav.questBuilder"))/>
            <p class="text-sm text-muted-foreground">"Dieses Modul wird gerade auf Rust portiert."</p>
        </div>
    }
}

/// Quest-Wiki-Fenster (`index.html?view=wiki`).
/// PORT-TODO: aus src/features/quest-builder/wiki/QuestWiki.tsx portieren.
#[component]
pub fn QuestWiki() -> impl IntoView {
    view! { <div class="p-6 text-sm">"Quest-Wiki wird portiert."</div> }
}
