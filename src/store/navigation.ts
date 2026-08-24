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
  Wrench,
  Sparkles,
  Megaphone,
  Moon,
  Table2,
  Beaker,
  Shield,
  Dices,
  LayoutGrid,
  Activity,
} from "lucide-react";

export type Section =
  | "dashboard"
  | "server-control"
  | "build-deploy"
  | "server-events"
  | "db-explorer"
  | "shop-editor"
  | "item-editor"
  | "item-proto-explorer"
  | "item-viewer"
  | "module-importer"
  | "mob-proto-editor"
  | "mob-drop-editor"
  | "drop-generator"
  | "refine-editor"
  | "box-editor"
  | "cube-editor"
  | "quest-builder"
  | "regen-editor"
  | "locale-editor"
  | "backup-browser"
  | "db-backups"
  | "activity-log"
  | "tga-converter"
  | "icon-browser"
  | "model-viewer"
  | "account-manager"
  | "gm-manager"
  | "system-installer"
  | "broadcast-system"
  | "weather-control"
  | "settings";

// Groups drive the collapsible sections in the Sidebar. "settings" is not a
// real group - it's rendered pinned at the bottom, never collapsed.
//
// Aufgeteilt aus dem früheren einzelnen "editors"-Sammeltopf (12 von 22
// Bereichen darin, alles andere 1-3) in vier thematisch enger gefasste
// Gruppen, auf Nutzerwunsch ("es ist fast alles in Datenbank-Editor").
export type Category =
  | "overview"
  | "items"
  | "shopsMonsters"
  | "questsWorld"
  | "serverAdmin"
  | "backups"
  | "assets"
  | "systems"
  | "settings";

// "systems" (System-Installer) deliberately left out here - the module is
// deactivated (see NAV_ITEMS below), so its category has no items and would
// otherwise render as an empty group heading in the Sidebar.
export const CATEGORY_ORDER: Category[] = [
  "overview",
  "items",
  "shopsMonsters",
  "questsWorld",
  "serverAdmin",
  "backups",
  "assets",
];

export const CATEGORY_LABEL_KEYS: Record<Category, string> = {
  overview: "nav.groups.overview",
  items: "nav.groups.items",
  shopsMonsters: "nav.groups.shopsMonsters",
  questsWorld: "nav.groups.questsWorld",
  serverAdmin: "nav.groups.serverAdmin",
  backups: "nav.groups.backups",
  assets: "nav.groups.assets",
  systems: "nav.groups.systems",
  settings: "nav.groups.settings",
};

// Single source of truth for nav metadata - used by the Sidebar and the
// Command Palette (Ctrl+K), so a new section only has to be added in one
// place instead of two lists silently drifting apart.
export const NAV_ITEMS: { section: Section; icon: typeof LayoutDashboard; labelKey: string; category: Category }[] = [
  { section: "dashboard", icon: LayoutDashboard, labelKey: "nav.dashboard", category: "overview" },
  { section: "server-control", icon: Terminal, labelKey: "nav.serverControl", category: "overview" },
  { section: "build-deploy", icon: Wrench, labelKey: "nav.buildDeploy", category: "overview" },
  { section: "item-editor", icon: PackagePlus, labelKey: "nav.itemEditor", category: "items" },
  { section: "item-proto-explorer", icon: Table2, labelKey: "nav.itemProtoExplorer", category: "items" },
  { section: "item-viewer", icon: LayoutGrid, labelKey: "nav.itemViewer", category: "items" },
  { section: "module-importer", icon: PackageSearch, labelKey: "nav.moduleImporter", category: "items" },
  { section: "refine-editor", icon: Hammer, labelKey: "nav.refineEditor", category: "items" },
  { section: "box-editor", icon: Package, labelKey: "nav.boxEditor", category: "items" },
  { section: "cube-editor", icon: Beaker, labelKey: "nav.cubeEditor", category: "items" },
  { section: "shop-editor", icon: Store, labelKey: "nav.shopEditor", category: "shopsMonsters" },
  { section: "mob-proto-editor", icon: PawPrint, labelKey: "nav.mobProtoEditor", category: "shopsMonsters" },
  { section: "mob-drop-editor", icon: Skull, labelKey: "nav.mobDropEditor", category: "shopsMonsters" },
  { section: "drop-generator", icon: Dices, labelKey: "nav.dropGenerator", category: "shopsMonsters" },
  { section: "quest-builder", icon: ScrollText, labelKey: "nav.questBuilder", category: "questsWorld" },
  { section: "regen-editor", icon: Sprout, labelKey: "nav.regenEditor", category: "questsWorld" },
  { section: "locale-editor", icon: Languages, labelKey: "nav.localeEditor", category: "questsWorld" },
  { section: "server-events", icon: Sparkles, labelKey: "nav.serverEvents", category: "serverAdmin" },
  { section: "account-manager", icon: Users, labelKey: "nav.accountManager", category: "serverAdmin" },
  { section: "gm-manager", icon: Shield, labelKey: "nav.gmManager", category: "serverAdmin" },
  { section: "db-explorer", icon: Database, labelKey: "nav.dbExplorer", category: "serverAdmin" },
  { section: "broadcast-system", icon: Megaphone, labelKey: "nav.broadcastSystem", category: "serverAdmin" },
  { section: "weather-control", icon: Moon, labelKey: "nav.weatherControl", category: "serverAdmin" },
  { section: "backup-browser", icon: History, labelKey: "nav.backupBrowser", category: "backups" },
  { section: "db-backups", icon: DatabaseBackup, labelKey: "nav.dbBackups", category: "backups" },
  { section: "activity-log", icon: Activity, labelKey: "nav.activityLog", category: "backups" },
  { section: "tga-converter", icon: ImagePlus, labelKey: "nav.tgaConverter", category: "assets" },
  { section: "icon-browser", icon: Images, labelKey: "nav.iconBrowser", category: "assets" },
  { section: "model-viewer", icon: Box, labelKey: "nav.modelViewer", category: "assets" },
  // System-Installer deliberately deactivated (2026-08-11, user request) -
  // not reliable enough in practice (see [[m2manager_system_installer]]:
  // several real live-install failures across multiple bug-fix rounds).
  // Removed from here (and from CATEGORY_ORDER above) so it's unreachable
  // via Sidebar/Command Palette - code/backend commands intentionally left
  // in place rather than deleted, in case this gets revisited later.
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

/** A search result waiting to be picked up by the module it navigates to -
 * e.g. an item vnum from the global search modal. `targetRef` is opaque to
 * the store itself (a vnum as a string, a quest relative_path, ...); only
 * the receiving module's own `useEffect` knows how to interpret it. */
export interface PendingSelection {
  section: Section;
  targetRef: string;
}

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

  pendingSelection: PendingSelection | null;
  /** Switches to `section` AND leaves a payload for it to pick up on mount -
   * the only cross-module "navigate with data" mechanism in the app (before
   * this, "Im X öffnen"-buttons only ever switched sections, never carried a
   * vnum/path along - see the Item-Proto-Explorer's pre-existing button for
   * the gap this closes). */
  goToWithSelection: (section: Section, targetRef: string) => void;
  /** Reads and clears `pendingSelection` in one step - "consume once" so
   * navigating away and back doesn't re-trigger the same selection. Returns
   * the payload only if it matches `section` (a module should ignore a
   * pending selection meant for a different section). */
  consumePendingSelection: (section: Section) => string | null;
}

export const useNavigationStore = create<NavigationState>((set, get) => ({
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

  pendingSelection: null,
  goToWithSelection: (section, targetRef) =>
    set((state) => {
      const recent = [state.section, ...state.recent.filter((s) => s !== state.section && s !== section)].slice(
        0,
        RECENT_LIMIT
      );
      saveJson(RECENT_KEY, recent);
      return { section, recent, pendingSelection: { section, targetRef } };
    }),
  consumePendingSelection: (section): string | null => {
    const current = get().pendingSelection;
    if (!current || current.section !== section) return null;
    set({ pendingSelection: null });
    return current.targetRef;
  },
}));

// Convenience hook for feature editors: mirrors a local `dirty` boolean into
// the shared nav store so the Sidebar/Command Palette can show an indicator,
// without editors needing to know anything about navigation state.
export function reportSectionDirty(section: Section, dirty: boolean) {
  useNavigationStore.getState().setSectionDirty(section, dirty);
}
