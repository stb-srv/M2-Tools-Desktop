//! PORT-TODO: aus src/features/guild-manager portieren.

use crate::components::PageHeader;
use crate::i18n::t;
use crate::store::navigation::Section;
use leptos::prelude::*;

#[component]
pub fn GuildManager() -> impl IntoView {
    view! {
        <div class="space-y-4">
            <PageHeader module=Section::GuildManager title=Signal::derive(|| t("nav.guildManager"))/>
            <p class="text-sm text-muted-foreground">"Dieses Modul wird gerade auf Rust portiert."</p>
        </div>
    }
}
