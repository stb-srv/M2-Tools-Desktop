import { useTranslation } from "react-i18next";
import { useNavigationStore, NAV_ITEMS } from "@/store/navigation";
import { cn } from "@/lib/utils";

export function Sidebar() {
  const { t } = useTranslation();
  const { section, setSection } = useNavigationStore();

  return (
    <aside className="flex w-56 flex-col border-r border-border bg-card">
      <div className="px-4 py-4 text-lg font-semibold">M2Manager</div>
      <nav className="flex flex-1 flex-col gap-1 overflow-y-auto px-2">
        {NAV_ITEMS.map(({ section: item, icon: Icon, labelKey }) => (
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
            {t(labelKey)}
          </button>
        ))}
      </nav>
    </aside>
  );
}
