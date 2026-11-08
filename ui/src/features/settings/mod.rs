//! Einstellungen: Bereiche links (bzw. oben als Leiste), Inhalt rechts.

pub mod client_tab;
pub mod connection_tab;
pub mod general_tab;
pub mod notifications_tab;
pub mod server_paths_tab;
pub mod shared;
pub mod updates_tab;

use crate::components::{Button, ButtonSize, ButtonVariant, Icon};
use crate::i18n::t;
use crate::util::open_manual;
use crate::store::navigation::Section;
use crate::store::settings_section::{settings_section, SettingsSection};
use crate::store::update::update_store;
use leptos::prelude::*;

struct SectionDef {
    id: SettingsSection,
    label: &'static str,
    description: &'static str,
    icon: &'static str,
}

const SECTIONS: [SectionDef; 6] = [
    SectionDef {
        id: SettingsSection::General,
        label: "Allgemein",
        description: "Design, Sprache, Größe der Oberfläche und Voreinstellungen für die Editoren.",
        icon: "SlidersHorizontal",
    },
    SectionDef {
        id: SettingsSection::Connection,
        label: "Verbindung",
        description: "Server-Profile mit SSH- und MySQL-Zugang. Den aktiven Server wechselst du auch auf der Startseite.",
        icon: "Plug",
    },
    SectionDef {
        id: SettingsSection::ServerPaths,
        label: "Server-Pfade",
        description: "Wo die Dateien auf dem Gameserver liegen. Gilt für das aktive Server-Profil, gespeichert wird beim Verlassen eines Feldes oder mit Enter.",
        icon: "FolderTree",
    },
    SectionDef {
        id: SettingsSection::Client,
        label: "Client & Werkzeuge",
        description: "Lokale Ordner und Programme auf diesem PC.",
        icon: "Monitor",
    },
    SectionDef {
        id: SettingsSection::Notifications,
        label: "Benachrichtigungen",
        description: "Meldungen an Discord o.ä., wenn etwas schiefgeht.",
        icon: "Bell",
    },
    SectionDef {
        id: SettingsSection::Updates,
        label: "Updates",
        description: "M2Manager aktualisiert sich selbst über GitHub-Releases.",
        icon: "Download",
    },
];

fn section_def(id: SettingsSection) -> &'static SectionDef {
    SECTIONS.iter().find(|s| s.id == id).unwrap_or(&SECTIONS[0])
}

fn section_content(id: SettingsSection) -> AnyView {
    match id {
        SettingsSection::General => view! { <general_tab::GeneralTab/> }.into_any(),
        SettingsSection::Connection => view! { <connection_tab::ConnectionTab/> }.into_any(),
        SettingsSection::ServerPaths => view! { <server_paths_tab::ServerPathsTab/> }.into_any(),
        SettingsSection::Client => view! { <client_tab::ClientTab/> }.into_any(),
        SettingsSection::Notifications => view! { <notifications_tab::NotificationsTab/> }.into_any(),
        SettingsSection::Updates => view! { <updates_tab::UpdatesTab/> }.into_any(),
    }
}

#[component]
pub fn Settings() -> impl IntoView {
    let section = settings_section();
    let update_available = move || update_store().available.with(|a| a.is_some());
    let current = move || section_def(section.get());

    let nav_buttons = SECTIONS
        .iter()
        .map(|s| {
            let id = s.id;
            let active = move || section.get() == id;
            view! {
                <button
                    on:click=move |_| section.set(id)
                    aria-current=move || active().then_some("page")
                    class=move || {
                        if active() {
                            "flex shrink-0 items-center gap-2.5 rounded-lg px-3 py-2.5 text-left text-sm transition-colors lg:w-full bg-primary font-semibold text-primary-foreground shadow-sm"
                        } else {
                            "flex shrink-0 items-center gap-2.5 rounded-lg px-3 py-2.5 text-left text-sm transition-colors lg:w-full text-muted-foreground hover:bg-muted hover:text-foreground"
                        }
                    }
                >
                    <Icon name=s.icon class="size-[1.125rem] shrink-0"/>
                    <span class="flex-1 whitespace-nowrap">{s.label}</span>
                    <Show when=move || id == SettingsSection::Updates && update_available()>
                        <span class=move || {
                            if active() { "size-2 shrink-0 rounded-full bg-primary-foreground" } else { "size-2 shrink-0 rounded-full bg-blue-500" }
                        }></span>
                    </Show>
                    <Show when=active>
                        <Icon name="ChevronRight" class="hidden size-4 shrink-0 lg:block"/>
                    </Show>
                </button>
            }
        })
        .collect_view();

    // Alle Bereiche bleiben gemountet (nur per `hidden` ausgeblendet), damit
    // z.B. der Update-Check oder ein Verbindungstest nicht bei jedem Wechsel
    // neu läuft.
    let bodies = SECTIONS
        .iter()
        .map(|s| {
            let id = s.id;
            view! { <div class=move || if section.get() != id { "hidden" } else { "" }>{section_content(id)}</div> }
        })
        .collect_view();

    view! {
        <div class="space-y-6">
            <div class="flex items-center gap-2">
                <h1 class="text-2xl font-semibold">{move || t("settings.title")}</h1>
                <Button
                    variant=ButtonVariant::Ghost
                    size=ButtonSize::IconSm
                    title="Hilfe zu diesem Modul"
                    on_click=move |_| open_manual(Some(Section::Settings.into()))
                >
                    <Icon name="HelpCircle" class="size-4"/>
                </Button>
            </div>

            <div class="grid gap-6 lg:grid-cols-[16rem_minmax(0,1fr)] xl:gap-8">
                // Schmal: Bereiche als waagerechte Leiste; ab lg links als feste Spalte.
                <nav class="-mx-1 flex gap-1 overflow-x-auto px-1 pb-1 lg:sticky lg:top-0 lg:mx-0 lg:flex-col lg:self-start lg:overflow-visible lg:rounded-xl lg:border lg:border-border lg:bg-card lg:p-2 lg:shadow-sm">
                    {nav_buttons}
                </nav>

                <div class="min-w-0 space-y-5">
                    <header class="flex items-start gap-4 rounded-xl border border-border bg-card px-5 py-4 shadow-sm">
                        <div class="flex size-11 shrink-0 items-center justify-center rounded-xl bg-primary text-primary-foreground">
                            <Icon name=Signal::derive(move || current().icon) class="size-5"/>
                        </div>
                        <div class="min-w-0">
                            <div class="text-xs font-medium uppercase tracking-wide text-muted-foreground">"Einstellungen"</div>
                            <h2 class="text-xl font-semibold leading-tight">{move || current().label}</h2>
                            <p class="mt-1 text-sm text-muted-foreground">{move || current().description}</p>
                        </div>
                    </header>

                    {bodies}
                </div>
            </div>
        </div>
    }
}
