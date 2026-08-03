import { useTranslation } from "react-i18next";
import { ChevronDown, ChevronRight, Star } from "lucide-react";
import {
  useNavigationStore,
  NAV_ITEMS,
  CATEGORY_ORDER,
  CATEGORY_LABEL_KEYS,
  type Section,
} from "@/store/navigation";
import { cn } from "@/lib/utils";

function NavRow({ section }: { section: Section }) {
  const { t } = useTranslation();
  const item = NAV_ITEMS.find((i) => i.section === section);
  const current = useNavigationStore((s) => s.section);
  const setSection = useNavigationStore((s) => s.setSection);
  const favorites = useNavigationStore((s) => s.favorites);
  const toggleFavorite = useNavigationStore((s) => s.toggleFavorite);
  const dirty = useNavigationStore((s) => !!s.dirtySections[section]);
  if (!item) return null;

  const Icon = item.icon;
  const isFavorite = favorites.includes(section);
  const isActive = current === section;

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
        <span className="flex-1 truncate">{t(item.labelKey)}</span>
        {dirty && (
          <span
            title={t("nav.unsavedChanges")}
            className={cn(
              "size-1.5 shrink-0 rounded-full",
              isActive ? "bg-primary-foreground" : "bg-amber-500"
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

  const settingsItem = NAV_ITEMS.find((i) => i.category === "settings")!;
  const recentVisible = recent.filter((s) => !favorites.includes(s)).slice(0, 3);

  return (
    <aside className="flex w-60 flex-col border-r border-border bg-card">
      <div className="px-4 py-4 text-lg font-semibold">M2Manager</div>
      <nav className="flex flex-1 flex-col gap-3 overflow-y-auto px-2 pb-2">
        {favorites.length > 0 && (
          <div>
            <div className="px-3 pb-1 text-xs font-semibold uppercase tracking-wide text-muted-foreground">
              {t("nav.groups.favorites")}
            </div>
            <div className="flex flex-col gap-0.5">
              {favorites.map((section) => (
                <NavRow key={`fav-${section}`} section={section} />
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
                <NavRow key={`recent-${section}`} section={section} />
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
                    <NavRow key={section} section={section} />
                  ))}
                </div>
              )}
            </div>
          );
        })}
      </nav>
      <div className="border-t border-border px-2 py-2">
        <NavRow section={settingsItem.section} />
      </div>
    </aside>
  );
}
