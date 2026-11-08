//! PORT-TODO: aus src/features/backup-browser portieren.

use crate::components::PageHeader;
use crate::i18n::t;
use crate::store::navigation::Section;
use leptos::prelude::*;

#[component]
pub fn BackupsPage() -> impl IntoView {
    view! {
        <div class="space-y-4">
            <PageHeader module=Section::BackupBrowser title=Signal::derive(|| t("nav.backupBrowser"))/>
            <p class="text-sm text-muted-foreground">"Dieses Modul wird gerade auf Rust portiert."</p>
        </div>
    }
}
