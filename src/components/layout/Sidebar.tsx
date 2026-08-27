import { useTranslation } from "react-i18next";
import { ChevronDown, ChevronRight, PanelLeftClose, PanelLeft, Star } from "lucide-react";
import {
  useNavigationStore,
  NAV_ITEMS,
  CATEGORY_ORDER,
  CATEGORY_LABEL_KEYS,
  type Section,
} from "@/store/navigation";
import { useUpdateStore } from "@/store/updateStore";
import { cn } from "@/lib/utils";

function NavRow({ section, rail }: { section: Section; rail: boolean }) {
  const { t } = useTranslation();
  const item = NAV_ITEMS.find((i) => i.section === section);
  const current = useNavigationStore((s) => s.section);
  const setSection = useNavigationStore((s) => s.setSection);
  const favorites = useNavigationStore((s) => s.favorites);
  const toggleFavorite = useNavigationStore((s) => s.toggleFavorite);
  const dirty = useNavigationStore((s) => !!s.dirtySections[section]);
  const updateAvailable = useUpdateStore((s) => (section === "settings" ? s.available : null));
  if (!item) return null;

  const Icon = item.icon;
  const isFavorite = favorites.includes(section);
  const isActive = current === section;
  const label = t(item.labelKey);

  // Icon-only rail mode: no label, no pin button (favorites/recent don't
  // exist as separate sections here anyway - see Sidebar below), dirty/
  // update indicators become a small corner dot on the icon itself instead
  // of an inline span next to text that no longer renders.
  if (rail) {
    return (
      <button
        onClick={() => setSection(section)}
        title={label}
        className={cn(
          "relative mx-auto flex size-9 items-center justify-center rounded-md transition-colors",
          isActive ? "bg-primary text-primary-foreground" : "text-foreground hover:bg-muted"
        )}
      >
        <Icon className="size-4 shrink-0" />
        {dirty && (
          <span
            title={t("nav.unsavedChanges")}
            className="absolute right-1 top-1 size-1.5 rounded-full bg-warning"
          />
        )}
        {updateAvailable && (
          <span
            title={`Update verfügbar: v${updateAvailable.version}`}
            className="absolute right-1 top-1 size-1.5 rounded-full bg-primary"
          />
        )}
      </button>
    );
  }

  return (
    <div className="group relative flex items-center">
      <button
        onClick={() => setSection(section)}
        className={cn(
          "flex flex-1 items-center gap-2 rounded-md px-3 py-2 text-left text-sm transition-colors",
          isActive ? "bg-primary text-primary-foreground" : "text-foreground hover:bg-muted"
        )}
      >
        <Icon className="size-4 shrink-0" />
        <span className="flex-1 truncate">{label}</span>
        {dirty && (
          <span
            title={t("nav.unsavedChanges")}
            className={cn(
              "size-1.5 shrink-0 rounded-full",
              isActive ? "bg-primary-foreground" : "bg-warning"
            )}
          />
        )}
        {updateAvailable && (
          <span
            title={`Update verfügbar: v${updateAvailable.version}`}
            className={cn(
              "size-1.5 shrink-0 rounded-full",
              isActive ? "bg-primary-foreground" : "bg-primary"
            )}
          />
        )}
      </button>
      <button
        onClick={(e) => {
          e.stopPropagation();
          toggleFavorite(section);
        }}
        title={t(isFavorite ? "nav.unpin" : "nav.pin")}
        className={cn(
          "absolute right-1 rounded p-1 transition-opacity",
          isFavorite ? "opacity-100" : "opacity-0 group-hover:opacity-100",
          isActive ? "text-primary-foreground hover:bg-primary-foreground/20" : "text-muted-foreground hover:bg-border"
        )}
      >
        <Star className={cn("size-3", isFavorite && "fill-current")} />
      </button>
    </div>
  );
}

export function Sidebar() {
  const { t } = useTranslation();
  const collapsed = useNavigationStore((s) => s.collapsed);
  const toggleCategory = useNavigationStore((s) => s.toggleCategory);
  const favorites = useNavigationStore((s) => s.favorites);
  const recent = useNavigationStore((s) => s.recent);
  const railCollapsed = useNavigationStore((s) => s.railCollapsed);
  const toggleRail = useNavigationStore((s) => s.toggleRail);

  const settingsItem = NAV_ITEMS.find((i) => i.category === "settings")!;
  const recentVisible = recent.filter((s) => !favorites.includes(s)).slice(0, 3);

  // Icon-only rail: with ~30 nav entries now, favorites/recent/category
  // headers don't fit in a narrow column - fall back to one flat icon list
  // in the same overall order instead of trying to shrink the grouped view.
  if (railCollapsed) {
    return (
      <aside className="flex w-14 flex-col border-r border-border bg-card">
        <div className="flex items-center justify-center py-4">
          <button
            onClick={toggleRail}
            title={t("nav.expandSidebar")}
            className="rounded-md p-1.5 text-muted-foreground hover:bg-muted hover:text-foreground"
          >
            <PanelLeft className="size-4" />
          </button>
        </div>
        <nav className="flex flex-1 flex-col gap-1 overflow-y-auto px-1 pb-2">
          {NAV_ITEMS.filter((i) => i.category !== "settings").map(({ section }) => (
            <NavRow key={section} section={section} rail />
          ))}
        </nav>
        <div className="border-t border-border px-1 py-2">
          <NavRow section={settingsItem.section} rail />
        </div>
      </aside>
    );
  }

  return (
    <aside className="flex w-60 flex-col border-r border-border bg-card">
      <div className="flex items-center justify-between px-4 py-4">
        <span className="text-lg font-semibold">M2Manager</span>
        <button
          onClick={toggleRail}
          title={t("nav.collapseSidebar")}
          className="rounded-md p-1 text-muted-foreground hover:bg-muted hover:text-foreground"
        >
          <PanelLeftClose className="size-4" />
        </button>
      </div>
      <nav className="flex flex-1 flex-col gap-3 overflow-y-auto px-2 pb-2">
        {favorites.length > 0 && (
          <div>
            <div className="px-3 pb-1 text-xs font-semibold uppercase tracking-wide text-muted-foreground">
              {t("nav.groups.favorites")}
            </div>
            <div className="flex flex-col gap-0.5">
              {favorites.map((section) => (
                <NavRow key={`fav-${section}`} section={section} rail={false} />
              ))}
            </div>
          </div>
        )}

        {recentVisible.length > 0 && (
          <div>
            <div className="px-3 pb-1 text-xs font-semibold uppercase tracking-wide text-muted-foreground">
              {t("nav.groups.recent")}
            </div>
            <div className="flex flex-col gap-0.5">
              {recentVisible.map((section) => (
                <NavRow key={`recent-${section}`} section={section} rail={false} />
              ))}
            </div>
          </div>
        )}

        {CATEGORY_ORDER.map((category) => {
          const items = NAV_ITEMS.filter((i) => i.category === category);
          const isCollapsed = !!collapsed[category];
          return (
            <div key={category}>
              <button
                onClick={() => toggleCategory(category)}
                className="flex w-full items-center gap-1 px-3 pb-1 text-xs font-semibold uppercase tracking-wide text-muted-foreground hover:text-foreground"
              >
                {isCollapsed ? <ChevronRight className="size-3" /> : <ChevronDown className="size-3" />}
                {t(CATEGORY_LABEL_KEYS[category])}
              </button>
              {!isCollapsed && (
                <div className="flex flex-col gap-0.5">
                  {items.map(({ section }) => (
                    <NavRow key={section} section={section} rail={false} />
                  ))}
                </div>
              )}
            </div>
          );
        })}
      </nav>
      <div className="border-t border-border px-2 py-2">
        <NavRow section={settingsItem.section} rail={false} />
      </div>
    </aside>
  );
}
