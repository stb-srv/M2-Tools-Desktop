//! Account-/Spieler-Verwaltung (Port von `src/features/account-manager`).

pub mod components;
pub mod shared;

use crate::components::PageHeader;
use crate::features::gm_manager::GmManager;
use crate::features::shared::{SectionTab, SectionTabs};
use crate::store::navigation::Section;
use components::{AccountSection, GiveItemSection, TableSearchSection};
use leptos::prelude::*;
use shared::PLAYER_TABLE;

// GM-Rechte (`common.gmlist`) hängen an Account + Charakter und leben deshalb
// als zweiter Tab hier statt als eigener Sidebar-Punkt.
#[component]
pub fn AccountManager() -> impl IntoView {
    view! {
        <SectionTabs tabs=vec![
            SectionTab::new("accounts", || "Accounts & Spieler".to_string(), || view! { <AccountsAndPlayers/> }),
            SectionTab::new("gm", || "GM-Verwaltung".to_string(), || view! { <GmManager/> }),
        ]/>
    }
}

#[component]
fn AccountsAndPlayers() -> impl IntoView {
    view! {
        <div class="space-y-8 pb-10">
            <PageHeader module=Section::AccountManager title="Account-/Spieler-Verwaltung".to_string()/>
            <p class="max-w-4xl text-sm text-muted-foreground">
                "Es gibt keinen Kanal zu einem laufenden Spielprozess - Änderungen an bereits online befindlichen Spielern wirken daher unter Umständen erst nach Neuanmeldung, nicht sofort live."
            </p>

            <AccountSection/>
            <TableSearchSection
                title="Spieler"
                target=PLAYER_TABLE
                default_column="name"
                placeholder="Charaktername suchen…"
                player_tools=true
            />
            <GiveItemSection/>
        </div>
    }
}
