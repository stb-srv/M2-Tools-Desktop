//! Drop-Generator - bündelt alle 5 Drop-relevanten Server-Dateien unter
//! einem Dach. Mob-Drops (mob_drop_item.txt) und Item-Gruppen
//! (special_item_group.txt) sind die eigenständigen Editoren (Mob Drop
//! Editor/Kisten-Editor) - hier eingebettet, nicht dupliziert.

pub mod common_drop_editor;
pub mod drop_item_group_editor;
pub mod etc_drop_editor;
pub mod join;

use crate::components::PageHeader;
use crate::features::box_editor::BoxEditor;
use crate::features::mob_drop_editor::MobDropEditor;
use crate::features::shared::{SectionTab, SectionTabs};
use crate::store::navigation::Section;
use common_drop_editor::CommonDropEditor;
use drop_item_group_editor::DropItemGroupEditor;
use etc_drop_editor::EtcDropEditor;
use leptos::prelude::*;
use serde::Deserialize;

/// Nur der Anzeigename aus `get_item_proto`.
#[derive(Deserialize)]
pub(crate) struct ItemProtoName {
    #[serde(default)]
    pub locale_name: String,
}

#[component]
pub fn DropGenerator() -> impl IntoView {
    // Jeder Tab lädt beim Mounten selbst per SFTP; SectionTabs hält besuchte
    // Tabs gemountet (nur versteckt), damit ungespeicherte Eingaben bleiben.
    let tabs = vec![
        SectionTab::new("mob", || "Mob-Drops".to_string(), || view! { <MobDropEditor/> }),
        SectionTab::new("common", || "Common-Drops".to_string(), || view! { <CommonDropEditor/> }),
        SectionTab::new("etc", || "Etc-Drops".to_string(), || view! { <EtcDropEditor/> }),
        SectionTab::new("special", || "Item-Gruppen".to_string(), || view! { <BoxEditor/> }),
        SectionTab::new("group", || "Zufalls-Gruppen".to_string(), || view! { <DropItemGroupEditor/> }),
    ];
    view! {
        <div class="space-y-4 pb-10">
            <PageHeader module=Section::DropGenerator title="Drop-Generator".to_string()/>
            <p class="max-w-4xl text-sm text-muted-foreground">
                "Alle 5 Drop-relevanten Server-Dateien an einem Ort. Jeder Tab erklärt oben kurz, wofür seine Datei zuständig ist und wie sie sich von den anderen unterscheidet - ausführlicher im Handbuch (Hilfe-Knopf oben)."
            </p>
            <SectionTabs tabs=tabs/>
        </div>
    }
}
