//! Item-Importer: fertige Ausrüstungs-Pakete (Icons + 3D-Modelle) oder
//! reine Icon-Ordner automatisch als Items anlegen (Port von
//! `src/features/module-importer`).

mod components;
pub mod shared;

use crate::components::PageHeader;
use crate::store::navigation::Section;
use components::icon_item_importer::IconItemImporter;
use components::import_history::ImportHistory;
use components::package_importer::PackageImporter;
use components::quick_remove_item::QuickRemoveItem;
use leptos::prelude::*;


#[derive(Clone, Copy, PartialEq, Eq)]
enum ImporterMode {
    Package,
    IconOnly,
}

#[component]
pub fn ModuleImporter() -> impl IntoView {
    let history_version = RwSignal::new(0u32);
    let mode = RwSignal::new(ImporterMode::Package);
    let bump = Callback::new(move |_| history_version.update(|v| *v += 1));

    let tab_class = move |m: ImporterMode| {
        if mode.get() == m {
            "px-3 py-1 bg-primary text-primary-foreground"
        } else {
            "px-3 py-1 "
        }
    };

    view! {
        <div class="space-y-6 pb-10">
            <PageHeader module=Section::ModuleImporter title="Item-Importer"/>

            <div class="flex overflow-hidden rounded-md border border-border text-sm w-fit">
                <button on:click=move |_| mode.set(ImporterMode::Package) class=move || tab_class(ImporterMode::Package)>
                    "Waffen / Rüstung (mit 3D-Modell)"
                </button>
                <button on:click=move |_| mode.set(ImporterMode::IconOnly) class=move || tab_class(ImporterMode::IconOnly)>
                    "Zubehör (nur Icon, z. B. Schuhe/Ketten/Schilder)"
                </button>
            </div>

            {move || match mode.get() {
                ImporterMode::Package => view! {
                    <p class="text-sm text-muted-foreground">
                        "Erkennt fertige Ausrüstungs-Pakete (Icons + 3D-Modelle, z. B. von Grafikern gelieferte Waffen-/ Rüstungs-Sets - auch gemischt in einem Ordner) automatisch und legt daraus Items mit Beispielwerten an - inklusive Icon, 3D-Modell und aller nötigen Client-Dateien."
                    </p>
                    <PackageImporter on_imported=bump/>
                }
                .into_any(),
                ImporterMode::IconOnly => view! {
                    <p class="text-sm text-muted-foreground">
                        "Für Items ganz ohne 3D-Modell: legt aus einem Ordner voller Icon-Dateien beliebig viele Items an, Typ/Subtyp/Werte pro Item einzeln einstellbar."
                    </p>
                    <IconItemImporter on_imported=bump/>
                }
                .into_any(),
            }}

            <ImportHistory refresh_key=history_version/>
            <QuickRemoveItem/>
        </div>
    }
}
