//! Server-Events: Event-Flags (Drop-/EXP-Raten, Saison-Events, sichtbare
//! Events) plus Tag/Nacht & Schnee als zweiter Tab.

pub mod flags;
mod server_events;

use crate::features::shared::{SectionTab, SectionTabs};
use crate::features::weather::WeatherControl;
use leptos::prelude::*;
pub use server_events::ServerEvents;

/// Tag/Nacht & Schnee setzt ebenfalls nur Event-Flags (`day`, `xmas_snow`)
/// und lebt deshalb als zweiter Tab hier statt als eigener Sidebar-Punkt.
#[component]
pub fn ServerEventsPage() -> impl IntoView {
    view! {
        <SectionTabs tabs=vec![
            SectionTab::new("flags", || "Event-Flags".to_string(), || view! { <ServerEvents/> }),
            SectionTab::new("weather", || "Tag/Nacht & Schnee".to_string(), || view! { <WeatherControl/> }),
        ]/>
    }
}
