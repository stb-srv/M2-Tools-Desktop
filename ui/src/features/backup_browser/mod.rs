//! Backups: Datei-Backups der Editoren (`m2manager_backups`-Ordner) und
//! komplette Datenbank-Sicherungen unter einem gemeinsamen Sidebar-Punkt.

mod backup_browser;
pub mod diff;

use crate::features::db_backups::DbBackups;
use crate::features::shared::{SectionTab, SectionTabs};
pub use backup_browser::BackupBrowser;
use leptos::prelude::*;

#[component]
pub fn BackupsPage() -> impl IntoView {
    view! {
        <SectionTabs tabs=vec![
            SectionTab::new("files", || "Datei-Backups".to_string(), || view! { <BackupBrowser/> }),
            SectionTab::new("database", || "Datenbank-Backups".to_string(), || view! { <DbBackups/> }),
        ]/>
    }
}
