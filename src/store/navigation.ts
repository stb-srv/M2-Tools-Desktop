import { create } from "zustand";
import {
  LayoutDashboard,
  Terminal,
  Database,
  Store,
  PackagePlus,
  PackageSearch,
  Skull,
  ScrollText,
  Sprout,
  Languages,
  History,
  ImagePlus,
  Box,
  PawPrint,
  Users,
  DatabaseBackup,
  Images,
  Settings as SettingsIcon,
  Hammer,
  Package,
} from "lucide-react";

export type Section =
  | "dashboard"
  | "server-control"
  | "db-explorer"
  | "shop-editor"
  | "item-editor"
  | "module-importer"
  | "mob-proto-editor"
  | "mob-drop-editor"
  | "refine-editor"
  | "box-editor"
  | "quest-builder"
  | "regen-editor"
  | "locale-editor"
  | "backup-browser"
  | "db-backups"
  | "tga-converter"
  | "icon-browser"
  | "model-viewer"
  | "account-manager"
  | "settings";

// Groups drive the collapsible sections in the Sidebar. "settings" is not a
// real group - it's rendered pinned at the bottom, never collapsed.
export type Category = "overview" | "editors" | "backups" | "assets" | "settings";

export const CATEGORY_ORDER: Category[] = ["overview", "editors", "backups", "assets"];

export const CATEGORY_LABEL_KEYS: Record<Category, string> = {
  overview: "nav.groups.overview",
  editors: "nav.groups.editors",
  backups: "nav.groups.backups",
  assets: "nav.groups.assets",
  settings: "nav.groups.settings",
};

// Single source of truth for nav metadata - used by the Sidebar and the
// Command Palette (Ctrl+K), so a new section only has to be added in one
// place instead of two lists silently drifting apart.
export const NAV_ITEMS: { section: Section; icon: typeof LayoutDashboard; labelKey: string; category: Category }[] = [
  { section: "dashboard", icon: LayoutDashboard, labelKey: "nav.dashboard", category: "overview" },
  { section: "server-control", icon: Terminal, labelKey: "nav.serverControl", category: "overview" },
  { section: "db-explorer", icon: Database, labelKey: "nav.dbExplorer", category: "editors" },
  { section: "shop-editor", icon: Store, labelKey: "nav.shopEditor", category: "editors" },
  { section: "item-editor", icon: PackagePlus, labelKey: "nav.itemEditor", category: "editors" },
  { section: "module-importer", icon: PackageSearch, labelKey: "nav.moduleImporter", category: "editors" },
  { section: "mob-proto-editor", icon: PawPrint, labelKey: "nav.mobProtoEditor", category: "editors" },
  { section: "mob-drop-editor", icon: Skull, labelKey: "nav.mobDropEditor", category: "editors" },
  { section: "refine-editor", icon: Hammer, labelKey: "nav.refineEditor", category: "editors" },
  { section: "box-editor", icon: Package, labelKey: "nav.boxEditor", category: "editors" },
  { section: "quest-builder", icon: ScrollText, labelKey: "nav.questBuilder", category: "editors" },
  { section: "regen-editor", icon: Sprout, labelKey: "nav.regenEditor", category: "editors" },
  { section: "locale-editor", icon: Languages, labelKey: "nav.localeEditor", category: "editors" },
  { section: "account-manager", icon: Users, labelKey: "nav.accountManager", category: "editors" },
  { section: "backup-browser", icon: History, labelKey: "nav.backupBrowser", category: "backups" },
  { section: "db-backups", icon: DatabaseBackup, labelKey: "nav.dbBackups", category: "backups" },
  { section: "tga-converter", icon: ImagePlus, labelKey: "nav.tgaConverter", category: "assets" },
  { section: "icon-browser", icon: Images, labelKey: "nav.iconBrowser", category: "assets" },
  { section: "model-viewer", icon: Box, labelKey: "nav.modelViewer", category: "assets" },
  { section: "settings", icon: SettingsIcon, labelKey: "nav.settings", category: "settings" },
];

const COLLAPSED_KEY = "m2manager-nav-collapsed";
const FAVORITES_KEY = "m2manager-nav-favorites";
const RECENT_KEY = "m2manager-nav-recent";
const RECENT_LIMIT = 5;

function loadJson<T>(key: string, fallback: T): T {
  try {
    const raw = localStorage.getItem(key);
    return raw ? (JSON.parse(raw) as T) : fallback;
  } catch {
    return fallback;
  }
}

function saveJson(key: string, value: unknown) {
  localStorage.setItem(key, JSON.stringify(value));
}

const VALID_SECTIONS = new Set(NAV_ITEMS.map((item) => item.section));

interface NavigationState {
  section: Section;
  setSection: (section: Section) => void;

  collapsed: Partial<Record<Category, boolean>>;
  toggleCategory: (category: Category) => void;

  favorites: Section[];
  toggleFavorite: (section: Section) => void;

  recent: Section[];

  dirtySections: Partial<Record<Section, boolean>>;
  setSectionDirty: (section: Section, dirty: boolean) => void;
}

export const useNavigationStore = create<NavigationState>((set) => ({
  section: "dashboard",
  setSection: (section) =>
    set((state) => {
      if (section === state.section) return state;
      const recent = [state.section, ...state.recent.filter((s) => s !== state.section && s !== section)].slice(
        0,
        RECENT_LIMIT
      );
      saveJson(RECENT_KEY, recent);
      return { section, recent };
    }),

  collapsed: loadJson(COLLAPSED_KEY, {}),
  toggleCategory: (category) =>
    set((state) => {
      const collapsed = { ...state.collapsed, [category]: !state.collapsed[category] };
      saveJson(COLLAPSED_KEY, collapsed);
      return { collapsed };
    }),

  favorites: loadJson(FAVORITES_KEY, [] as Section[]).filter((s) => VALID_SECTIONS.has(s)),
  toggleFavorite: (section) =>
    set((state) => {
      const has = state.favorites.includes(section);
      const favorites = has ? state.favorites.filter((s) => s !== section) : [...state.favorites, section];
      saveJson(FAVORITES_KEY, favorites);
      return { favorites };
    }),

  recent: loadJson(RECENT_KEY, [] as Section[]).filter((s) => VALID_SECTIONS.has(s)),

  dirtySections: {},
  setSectionDirty: (section, dirty) =>
    set((state) => {
      if (!!state.dirtySections[section] === dirty) return state;
      return { dirtySections: { ...state.dirtySections, [section]: dirty } };
    }),
}));

// Convenience hook for feature editors: mirrors a local `dirty` boolean into
// the shared nav store so the Sidebar/Command Palette can show an indicator,
// without editors needing to know anything about navigation state.
export function reportSectionDirty(section: Section, dirty: boolean) {
  useNavigationStore.getState().setSectionDirty(section, dirty);
}
