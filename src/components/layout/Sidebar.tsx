import { useTranslation } from "react-i18next";
import { useNavigationStore, type Section } from "@/store/navigation";
import { cn } from "@/lib/utils";
import {
  LayoutDashboard,
  Plug,
  Terminal,
  Database,
  Store,
  Box,
  Settings as SettingsIcon,
} from "lucide-react";

const NAV_ITEMS: { section: Section; icon: typeof LayoutDashboard }[] = [
  { section: "dashboard", icon: LayoutDashboard },
  { section: "connections", icon: Plug },
  { section: "server-control", icon: Terminal },
  { section: "db-explorer", icon: Database },
  { section: "shop-editor", icon: Store },
  { section: "model-viewer", icon: Box },
  { section: "settings", icon: SettingsIcon },
];

const NAV_LABEL_KEY: Record<Section, string> = {
  dashboard: "nav.dashboard",
  connections: "nav.connections",
  "server-control": "nav.serverControl",
  "db-explorer": "nav.dbExplorer",
  "shop-editor": "nav.shopEditor",
  "model-viewer": "nav.modelViewer",
  settings: "nav.settings",
};

export function Sidebar() {
  const { t } = useTranslation();
  const { section, setSection } = useNavigationStore();

  return (
    <aside className="flex w-56 flex-col border-r border-border bg-card">
      <div className="px-4 py-4 text-lg font-semibold">M2Manager</div>
      <nav className="flex flex-1 flex-col gap-1 px-2">
        {NAV_ITEMS.map(({ section: item, icon: Icon }) => (
          <button
            key={item}
            onClick={() => setSection(item)}
            className={cn(
              "flex items-center gap-2 rounded-md px-3 py-2 text-left text-sm transition-colors",
              section === item
                ? "bg-primary text-primary-foreground"
                : "text-foreground hover:bg-muted"
            )}
          >
            <Icon className="size-4" />
            {t(NAV_LABEL_KEY[item])}
          </button>
        ))}
      </nav>
    </aside>
  );
}
