//! Navigation: aktiver Bereich, Favoriten, zuletzt besucht, eingeklappte
//! Gruppen, "ungespeichert"-Markierungen und die Übergabe einer Auswahl
//! an einen anderen Bereich (`go_to_with_selection`).

use crate::lib::storage;
use leptos::prelude::*;
use std::collections::{HashMap, HashSet};
use std::sync::OnceLock;

macro_rules! sections {
    ($( $v:ident = $id:literal, $icon:literal, $label:literal, $cat:ident; )*) => {
        #[derive(Clone, Copy, PartialEq, Eq, Hash, Debug)]
        pub enum Section { $($v),* }

        impl Section {
            pub const ALL: &'static [Section] = &[$(Section::$v),*];
            pub fn id(self) -> &'static str { match self { $(Section::$v => $id),* } }
            pub fn icon(self) -> &'static str { match self { $(Section::$v => $icon),* } }
            pub fn label_key(self) -> &'static str { match self { $(Section::$v => $label),* } }
            pub fn category(self) -> Category { match self { $(Section::$v => Category::$cat),* } }
            pub fn from_id(id: &str) -> Option<Section> { match id { $($id => Some(Section::$v),)* _ => None } }
        }
    };
}

// Reihenfolge = Reihenfolge in Sidebar und Befehlspalette (Strg+K).
sections! {
    Dashboard = "dashboard", "LayoutDashboard", "nav.dashboard", Overview;
    ServerControl = "server-control", "Terminal", "nav.serverControl", Overview;
    BuildDeploy = "build-deploy", "Wrench", "nav.buildDeploy", Overview;
    ItemEditor = "item-editor", "PackagePlus", "nav.itemEditor", Items;
    ItemProtoExplorer = "item-proto-explorer", "Table2", "nav.itemProtoExplorer", Items;
    ModuleImporter = "module-importer", "PackageSearch", "nav.moduleImporter", Items;
    RefineEditor = "refine-editor", "Hammer", "nav.refineEditor", Items;
    CubeEditor = "cube-editor", "Beaker", "nav.cubeEditor", Items;
    ShopEditor = "shop-editor", "Store", "nav.shopEditor", ShopsMonsters;
    MobProtoEditor = "mob-proto-editor", "PawPrint", "nav.mobProtoEditor", ShopsMonsters;
    DropGenerator = "drop-generator", "Dices", "nav.dropGenerator", ShopsMonsters;
    QuestBuilder = "quest-builder", "ScrollText", "nav.questBuilder", QuestsWorld;
    RegenEditor = "regen-editor", "Sprout", "nav.regenEditor", QuestsWorld;
    LocaleEditor = "locale-editor", "Languages", "nav.localeEditor", QuestsWorld;
    ServerEvents = "server-events", "Sparkles", "nav.serverEvents", ServerAdmin;
    AccountManager = "account-manager", "Users", "nav.accountManager", ServerAdmin;
    GuildManager = "guild-manager", "Castle", "nav.guildManager", ServerAdmin;
    EconomyDashboard = "economy-dashboard", "Coins", "nav.economyDashboard", ServerAdmin;
    DbExplorer = "db-explorer", "Database", "nav.dbExplorer", ServerAdmin;
    BroadcastSystem = "broadcast-system", "Megaphone", "nav.broadcastSystem", ServerAdmin;
    BackupBrowser = "backup-browser", "History", "nav.backupBrowser", Backups;
    ActivityLog = "activity-log", "Activity", "nav.activityLog", Backups;
    TgaConverter = "tga-converter", "ImagePlus", "nav.tgaConverter", Assets;
    IconBrowser = "icon-browser", "Images", "nav.iconBrowser", Assets;
    ModelViewer = "model-viewer", "Box", "nav.modelViewer", Assets;
    Settings = "settings", "Settings", "nav.settings", Settings;
}

#[derive(Clone, Copy, PartialEq, Eq, Hash, Debug)]
pub enum Category {
    Overview,
    Items,
    ShopsMonsters,
    QuestsWorld,
    ServerAdmin,
    Backups,
    Assets,
    Settings,
}

impl Category {
    pub const ORDER: &'static [Category] = &[
        Category::Overview,
        Category::Items,
        Category::ShopsMonsters,
        Category::QuestsWorld,
        Category::ServerAdmin,
        Category::Backups,
        Category::Assets,
    ];
    pub fn id(self) -> &'static str {
        match self {
            Category::Overview => "overview",
            Category::Items => "items",
            Category::ShopsMonsters => "shopsMonsters",
            Category::QuestsWorld => "questsWorld",
            Category::ServerAdmin => "serverAdmin",
            Category::Backups => "backups",
            Category::Assets => "assets",
            Category::Settings => "settings",
        }
    }
    pub fn label_key(self) -> String {
        format!("nav.groups.{}", self.id())
    }
}

/// Früher eigenständige Module, die heute als Tab in einem anderen Bereich
/// leben. Sie behalten ihre Kennung für Änderungsprotokoll und Handbuch.
#[derive(Clone, Copy, PartialEq, Eq, Hash, Debug)]
pub enum EmbeddedModule {
    MobDropEditor,
    BoxEditor,
    WeatherControl,
    GmManager,
    DbBackups,
}

impl EmbeddedModule {
    pub const ALL: &'static [EmbeddedModule] = &[
        EmbeddedModule::MobDropEditor,
        EmbeddedModule::BoxEditor,
        EmbeddedModule::WeatherControl,
        EmbeddedModule::GmManager,
        EmbeddedModule::DbBackups,
    ];
    pub fn id(self) -> &'static str {
        match self {
            EmbeddedModule::MobDropEditor => "mob-drop-editor",
            EmbeddedModule::BoxEditor => "box-editor",
            EmbeddedModule::WeatherControl => "weather-control",
            EmbeddedModule::GmManager => "gm-manager",
            EmbeddedModule::DbBackups => "db-backups",
        }
    }
    pub fn host(self) -> Section {
        match self {
            EmbeddedModule::MobDropEditor | EmbeddedModule::BoxEditor => Section::DropGenerator,
            EmbeddedModule::WeatherControl => Section::ServerEvents,
            EmbeddedModule::GmManager => Section::AccountManager,
            EmbeddedModule::DbBackups => Section::BackupBrowser,
        }
    }
    pub fn label_key(self) -> &'static str {
        match self {
            EmbeddedModule::MobDropEditor => "nav.mobDropEditor",
            EmbeddedModule::BoxEditor => "nav.boxEditor",
            EmbeddedModule::WeatherControl => "nav.weatherControl",
            EmbeddedModule::GmManager => "nav.gmManager",
            EmbeddedModule::DbBackups => "nav.dbBackups",
        }
    }
}

/// Kennung für Änderungsprotokoll und Handbuch.
#[derive(Clone, Copy, PartialEq, Eq, Hash, Debug)]
pub enum ModuleId {
    Section(Section),
    Embedded(EmbeddedModule),
}

impl ModuleId {
    pub fn id(self) -> &'static str {
        match self {
            ModuleId::Section(s) => s.id(),
            ModuleId::Embedded(e) => e.id(),
        }
    }
    pub fn host(self) -> Section {
        match self {
            ModuleId::Section(s) => s,
            ModuleId::Embedded(e) => e.host(),
        }
    }
    pub fn is_embedded(self) -> bool {
        matches!(self, ModuleId::Embedded(_))
    }
    pub fn from_id(id: &str) -> Option<ModuleId> {
        Section::from_id(id)
            .map(ModuleId::Section)
            .or_else(|| EmbeddedModule::ALL.iter().find(|e| e.id() == id).map(|e| ModuleId::Embedded(*e)))
    }
}

impl From<Section> for ModuleId {
    fn from(s: Section) -> Self {
        ModuleId::Section(s)
    }
}
impl From<EmbeddedModule> for ModuleId {
    fn from(e: EmbeddedModule) -> Self {
        ModuleId::Embedded(e)
    }
}

const COLLAPSED_KEY: &str = "m2manager-nav-collapsed";
const FAVORITES_KEY: &str = "m2manager-nav-favorites";
const RECENT_KEY: &str = "m2manager-nav-recent";
const RAIL_COLLAPSED_KEY: &str = "m2manager-nav-rail-collapsed";
const RECENT_LIMIT: usize = 5;

#[derive(Clone, Debug, PartialEq)]
pub struct PendingSelection {
    pub section: Section,
    pub target_ref: String,
}

#[derive(Clone, Copy)]
pub struct NavStore {
    pub section: RwSignal<Section>,
    pub collapsed: RwSignal<HashSet<Category>>,
    pub rail_collapsed: RwSignal<bool>,
    pub favorites: RwSignal<Vec<Section>>,
    pub recent: RwSignal<Vec<Section>>,
    pub dirty: RwSignal<HashMap<Section, bool>>,
    pub pending_selection: RwSignal<Option<PendingSelection>>,
}

static NAV: OnceLock<NavStore> = OnceLock::new();

fn load_sections(key: &str) -> Vec<Section> {
    storage::get_json::<Vec<String>>(key)
        .unwrap_or_default()
        .iter()
        .filter_map(|s| Section::from_id(s))
        .collect()
}

fn save_sections(key: &str, v: &[Section]) {
    storage::set_json(key, &v.iter().map(|s| s.id()).collect::<Vec<_>>());
}

pub(super) fn init() {
    let collapsed: HashSet<Category> = storage::get_json::<HashMap<String, bool>>(COLLAPSED_KEY)
        .unwrap_or_default()
        .into_iter()
        .filter(|(_, v)| *v)
        .filter_map(|(k, _)| Category::ORDER.iter().find(|c| c.id() == k).copied())
        .collect();
    let _ = NAV.set(NavStore {
        section: RwSignal::new(Section::Dashboard),
        collapsed: RwSignal::new(collapsed),
        rail_collapsed: RwSignal::new(storage::get_json(RAIL_COLLAPSED_KEY).unwrap_or(false)),
        favorites: RwSignal::new(load_sections(FAVORITES_KEY)),
        recent: RwSignal::new(load_sections(RECENT_KEY)),
        dirty: RwSignal::new(HashMap::new()),
        pending_selection: RwSignal::new(None),
    });
}

pub fn nav() -> NavStore {
    *NAV.get().expect("store::init() nicht aufgerufen")
}

impl NavStore {
    fn push_recent(&self, next: Section) {
        let cur = self.section.get_untracked();
        let mut recent: Vec<Section> = std::iter::once(cur)
            .chain(self.recent.get_untracked().into_iter().filter(|s| *s != cur && *s != next))
            .collect();
        recent.truncate(RECENT_LIMIT);
        save_sections(RECENT_KEY, &recent);
        self.recent.set(recent);
    }

    pub fn set_section(&self, section: Section) {
        if section == self.section.get_untracked() {
            return;
        }
        self.push_recent(section);
        self.section.set(section);
    }

    pub fn toggle_category(&self, c: Category) {
        self.collapsed.update(|set| {
            if !set.remove(&c) {
                set.insert(c);
            }
            let map: HashMap<&str, bool> = set.iter().map(|c| (c.id(), true)).collect();
            storage::set_json(COLLAPSED_KEY, &map);
        });
    }

    pub fn toggle_rail(&self) {
        let v = !self.rail_collapsed.get_untracked();
        storage::set_json(RAIL_COLLAPSED_KEY, &v);
        self.rail_collapsed.set(v);
    }

    pub fn toggle_favorite(&self, s: Section) {
        self.favorites.update(|f| {
            if let Some(i) = f.iter().position(|x| *x == s) {
                f.remove(i);
            } else {
                f.push(s);
            }
            save_sections(FAVORITES_KEY, f);
        });
    }

    pub fn set_section_dirty(&self, s: Section, dirty: bool) {
        if self.dirty.with_untracked(|m| m.get(&s).copied().unwrap_or(false)) == dirty {
            return;
        }
        self.dirty.update(|m| {
            m.insert(s, dirty);
        });
    }

    pub fn is_dirty(&self, s: Section) -> bool {
        self.dirty.with(|m| m.get(&s).copied().unwrap_or(false))
    }

    /// Wechselt zu `section` und hinterlegt `target_ref` (z.B. eine vnum),
    /// das der Zielbereich mit [`consume_pending_selection`] abholt.
    pub fn go_to_with_selection(&self, section: Section, target_ref: impl Into<String>) {
        if section != self.section.get_untracked() {
            self.push_recent(section);
        }
        self.pending_selection.set(Some(PendingSelection { section, target_ref: target_ref.into() }));
        self.section.set(section);
    }

    /// Liest und löscht die hinterlegte Auswahl - nur, wenn sie für `section` gedacht ist.
    pub fn consume_pending_selection(&self, section: Section) -> Option<String> {
        let cur = self.pending_selection.get_untracked()?;
        if cur.section != section {
            return None;
        }
        self.pending_selection.set(None);
        Some(cur.target_ref)
    }
}

/// Spiegelt einen lokalen "ungespeichert"-Zustand in die Sidebar.
pub fn report_section_dirty(section: Section, dirty: bool) {
    nav().set_section_dirty(section, dirty);
}
