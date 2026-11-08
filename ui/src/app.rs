//! Hauptfenster: Einrichtungs-Check, MySQL-Autoverbindung, leiser
//! Update-Check und die Bereiche. Ein einmal besuchter Bereich bleibt
//! gemountet (nur versteckt), damit ungespeicherte Eingaben beim Wechsel
//! nicht verloren gehen.

use crate::components::{command_palette::CommandPalette, crash_watch::CrashWatch, global_search::GlobalSearch, whats_new::WhatsNewDialog};
use crate::features as f;
use crate::layout::Sidebar;
use crate::util::connection::{auto_connect_mysql, get_setting};
use crate::store::navigation::{nav, Section};
use crate::store::update::update_store;
use leptos::prelude::*;
use leptos::task::spawn_local;

fn section_view(s: Section) -> AnyView {
    match s {
        Section::Dashboard => view! { <f::dashboard::Dashboard/> }.into_any(),
        Section::ServerControl => view! { <f::server_control::ServerControl/> }.into_any(),
        Section::BuildDeploy => view! { <f::build_deploy::BuildDeploy/> }.into_any(),
        Section::ServerEvents => view! { <f::server_events::ServerEventsPage/> }.into_any(),
        Section::DbExplorer => view! { <f::db_explorer::DbExplorer/> }.into_any(),
        Section::ShopEditor => view! { <f::shop_editor::ShopEditor/> }.into_any(),
        Section::ItemEditor => view! { <f::item_editor::ItemEditor/> }.into_any(),
        Section::ItemProtoExplorer => view! { <f::item_proto_explorer::ItemProtoExplorer/> }.into_any(),
        Section::ModuleImporter => view! { <f::module_importer::ModuleImporter/> }.into_any(),
        Section::MobProtoEditor => view! { <f::mob_proto_editor::MobProtoEditor/> }.into_any(),
        Section::DropGenerator => view! { <f::drop_generator::DropGenerator/> }.into_any(),
        Section::RefineEditor => view! { <f::refine_editor::RefineEditor/> }.into_any(),
        Section::CubeEditor => view! { <f::cube_editor::CubeEditor/> }.into_any(),
        Section::QuestBuilder => view! { <f::quest_builder::QuestBuilder/> }.into_any(),
        Section::RegenEditor => view! { <f::regen_editor::RegenEditor/> }.into_any(),
        Section::LocaleEditor => view! { <f::locale_editor::LocaleEditor/> }.into_any(),
        Section::BackupBrowser => view! { <f::backup_browser::BackupsPage/> }.into_any(),
        Section::ActivityLog => view! { <f::activity_log::ActivityLog/> }.into_any(),
        Section::TgaConverter => view! { <f::tga_converter::TgaConverter/> }.into_any(),
        Section::IconBrowser => view! { <f::icon_browser::IconBrowser/> }.into_any(),
        Section::ModelViewer => view! { <f::model_viewer::ModelViewer/> }.into_any(),
        Section::AccountManager => view! { <f::account_manager::AccountManager/> }.into_any(),
        Section::GuildManager => view! { <f::guild_manager::GuildManager/> }.into_any(),
        Section::EconomyDashboard => view! { <f::economy_dashboard::EconomyDashboard/> }.into_any(),
        Section::BroadcastSystem => view! { <f::broadcast::BroadcastSystem/> }.into_any(),
        Section::Settings => view! { <f::settings::Settings/> }.into_any(),
    }
}

#[component]
fn SectionSlot(section: Section, visited: RwSignal<Vec<Section>>) -> impl IntoView {
    let n = nav();
    view! {
        <Show when=move || visited.with(|v| v.contains(&section))>
            <div class=move || if n.section.get() == section { "h-full" } else { "h-full hidden" }>
                {section_view(section)}
            </div>
        </Show>
    }
}

#[component]
pub fn App() -> impl IntoView {
    let n = nav();
    let visited = RwSignal::new(vec![Section::Dashboard]);
    Effect::new(move |_| {
        let s = n.section.get();
        if !visited.with_untracked(|v| v.contains(&s)) {
            visited.update(|v| v.push(s));
        }
    });

    let setup_checked = RwSignal::new(false);
    let setup_completed = RwSignal::new(true);
    let mysql_checked = RwSignal::new(false);

    spawn_local(async move {
        setup_completed.set(get_setting("setup_completed").await.as_deref() == Some("true"));
        setup_checked.set(true);
    });

    Effect::new(move |_| {
        if !setup_checked.get() || !setup_completed.get() {
            return;
        }
        spawn_local(async move {
            auto_connect_mysql().await;
            mysql_checked.set(true);
        });
        // Leiser Hintergrund-Check: zeigt bei Erfolg nur den Punkt am
        // Einstellungen-Eintrag, schlägt lautlos fehl (kein Internet o.ä.).
        spawn_local(async move {
            if let Ok(Some(update)) = crate::tauri::update_check().await {
                update_store().set_available(Some(update));
            }
        });
    });

    move || {
        if !setup_checked.get() {
            return view! { <div class="h-screen bg-background"></div> }.into_any();
        }
        if !setup_completed.get() {
            return view! { <f::setup::SetupWizard on_complete=move |_| setup_completed.set(true)/> }.into_any();
        }
        if !mysql_checked.get() {
            return view! { <div class="h-screen bg-background"></div> }.into_any();
        }
        view! {
            <div class="flex h-screen bg-background text-foreground">
                <CommandPalette/>
                <GlobalSearch/>
                <CrashWatch/>
                <WhatsNewDialog/>
                <Sidebar/>
                <main class="flex-1 overflow-y-auto p-6">
                    {Section::ALL.iter().map(|s| view! { <SectionSlot section=*s visited=visited/> }).collect_view()}
                </main>
            </div>
        }
        .into_any()
    }
}
