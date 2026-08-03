import { create } from "zustand";
import {
  LayoutDashboard,
  Terminal,
  Database,
  Store,
  PackagePlus,
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
} from "lucide-react";

export type Section =
  | "dashboard"
  | "server-control"
  | "db-explorer"
  | "shop-editor"
  | "item-editor"
  | "mob-proto-editor"
  | "mob-drop-editor"
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

// Single source of truth for nav metadata - used by the Sidebar and the
// Command Palette (Ctrl+K), so a new section only has to be added in one
// place instead of two lists silently drifting apart.
export const NAV_ITEMS: { section: Section; icon: typeof LayoutDashboard; labelKey: string }[] = [
  { section: "dashboard", icon: LayoutDashboard, labelKey: "nav.dashboard" },
  { section: "server-control", icon: Terminal, labelKey: "nav.serverControl" },
  { section: "db-explorer", icon: Database, labelKey: "nav.dbExplorer" },
  { section: "shop-editor", icon: Store, labelKey: "nav.shopEditor" },
  { section: "item-editor", icon: PackagePlus, labelKey: "nav.itemEditor" },
  { section: "mob-proto-editor", icon: PawPrint, labelKey: "nav.mobProtoEditor" },
  { section: "mob-drop-editor", icon: Skull, labelKey: "nav.mobDropEditor" },
  { section: "quest-builder", icon: ScrollText, labelKey: "nav.questBuilder" },
  { section: "regen-editor", icon: Sprout, labelKey: "nav.regenEditor" },
  { section: "locale-editor", icon: Languages, labelKey: "nav.localeEditor" },
  { section: "backup-browser", icon: History, labelKey: "nav.backupBrowser" },
  { section: "db-backups", icon: DatabaseBackup, labelKey: "nav.dbBackups" },
  { section: "tga-converter", icon: ImagePlus, labelKey: "nav.tgaConverter" },
  { section: "icon-browser", icon: Images, labelKey: "nav.iconBrowser" },
  { section: "model-viewer", icon: Box, labelKey: "nav.modelViewer" },
  { section: "account-manager", icon: Users, labelKey: "nav.accountManager" },
  { section: "settings", icon: SettingsIcon, labelKey: "nav.settings" },
];

interface NavigationState {
  section: Section;
  setSection: (section: Section) => void;
}

export const useNavigationStore = create<NavigationState>((set) => ({
  section: "dashboard",
  setSection: (section) => set({ section }),
}));
