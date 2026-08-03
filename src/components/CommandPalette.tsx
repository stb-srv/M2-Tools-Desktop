import { useEffect, useMemo, useRef, useState } from "react";
import { useTranslation } from "react-i18next";
import {
  useNavigationStore,
  NAV_ITEMS,
  CATEGORY_ORDER,
  CATEGORY_LABEL_KEYS,
  type Category,
  type Section,
} from "@/store/navigation";
import { Search } from "lucide-react";

type ResultItem = { section: Section; icon: (typeof NAV_ITEMS)[number]["icon"]; label: string };
type Group = { key: string; labelKey: string; items: ResultItem[] };

// Global Ctrl+K/Cmd+K launcher for jumping straight to a section without
// hunting through the sidebar - reuses NAV_ITEMS (the same list the Sidebar
// renders) so the two can never drift out of sync.
export function CommandPalette() {
  const { t } = useTranslation();
  const setSection = useNavigationStore((s) => s.setSection);
  const favorites = useNavigationStore((s) => s.favorites);
  const recent = useNavigationStore((s) => s.recent);
  const dirtySections = useNavigationStore((s) => s.dirtySections);
  const [open, setOpen] = useState(false);
  const [query, setQuery] = useState("");
  const [activeIndex, setActiveIndex] = useState(0);
  const inputRef = useRef<HTMLInputElement>(null);

  const allItems = useMemo(
    (): ResultItem[] => NAV_ITEMS.map((item) => ({ section: item.section, icon: item.icon, label: t(item.labelKey) })),
    [t]
  );
  const bysection = useMemo(() => new Map(allItems.map((i) => [i.section, i])), [allItems]);

  const groups = useMemo((): Group[] => {
    const q = query.trim().toLowerCase();

    if (q) {
      const items = allItems.filter((item) => item.label.toLowerCase().includes(q));
      return items.length ? [{ key: "results", labelKey: "nav.groups.results", items }] : [];
    }

    const groups: Group[] = [];
    const favItems = favorites.map((s) => bysection.get(s)).filter((i): i is ResultItem => !!i);
    if (favItems.length) groups.push({ key: "favorites", labelKey: "nav.groups.favorites", items: favItems });

    const recentItems = recent
      .filter((s) => !favorites.includes(s))
      .map((s) => bysection.get(s))
      .filter((i): i is ResultItem => !!i);
    if (recentItems.length) groups.push({ key: "recent", labelKey: "nav.groups.recent", items: recentItems });

    for (const category of CATEGORY_ORDER) {
      const items = NAV_ITEMS.filter((i) => i.category === category).map((i) => bysection.get(i.section));
      const resolved = items.filter((i): i is ResultItem => !!i);
      if (resolved.length) groups.push({ key: category, labelKey: CATEGORY_LABEL_KEYS[category as Category], items: resolved });
    }
    const settingsItem = bysection.get("settings" as Section);
    if (settingsItem) groups.push({ key: "settings", labelKey: CATEGORY_LABEL_KEYS.settings, items: [settingsItem] });

    return groups;
  }, [query, allItems, bysection, favorites, recent]);

  const flatResults = useMemo(() => groups.flatMap((g) => g.items), [groups]);

  useEffect(() => {
    function handleGlobalKeyDown(e: KeyboardEvent) {
      if ((e.metaKey || e.ctrlKey) && e.key.toLowerCase() === "k") {
        e.preventDefault();
        setOpen((v) => !v);
      } else if (e.key === "Escape" && open) {
        setOpen(false);
      }
    }
    window.addEventListener("keydown", handleGlobalKeyDown);
    return () => window.removeEventListener("keydown", handleGlobalKeyDown);
  }, [open]);

  useEffect(() => {
    if (open) {
      setQuery("");
      setActiveIndex(0);
      requestAnimationFrame(() => inputRef.current?.focus());
    }
  }, [open]);

  useEffect(() => {
    setActiveIndex(0);
  }, [query]);

  function choose(index: number) {
    const item = flatResults[index];
    if (!item) return;
    setSection(item.section);
    setOpen(false);
  }

  function handleKeyDown(e: React.KeyboardEvent) {
    if (e.key === "ArrowDown") {
      e.preventDefault();
      setActiveIndex((i) => Math.min(i + 1, flatResults.length - 1));
    } else if (e.key === "ArrowUp") {
      e.preventDefault();
      setActiveIndex((i) => Math.max(i - 1, 0));
    } else if (e.key === "Enter") {
      e.preventDefault();
      choose(activeIndex);
    }
  }

  if (!open) return null;

  let runningIndex = -1;

  return (
    <div
      className="fixed inset-0 z-50 flex items-start justify-center bg-black/50 pt-[15vh]"
      onClick={() => setOpen(false)}
    >
      <div
        className="w-full max-w-lg overflow-hidden rounded-lg border border-border bg-card shadow-xl"
        onClick={(e) => e.stopPropagation()}
      >
        <div className="flex items-center gap-2 border-b border-border px-3 py-2">
          <Search className="size-4 shrink-0 text-muted-foreground" />
          <input
            ref={inputRef}
            value={query}
            onChange={(e) => setQuery(e.target.value)}
            onKeyDown={handleKeyDown}
            placeholder="Zu einem Bereich springen… (Strg+K)"
            className="w-full bg-transparent py-1 text-sm outline-none"
          />
        </div>
        <div className="max-h-96 overflow-y-auto p-1">
          {groups.map((group) => (
            <div key={group.key}>
              <div className="px-3 pb-1 pt-2 text-xs font-semibold uppercase tracking-wide text-muted-foreground">
                {t(group.labelKey)}
              </div>
              {group.items.map((item) => {
                runningIndex += 1;
                const index = runningIndex;
                const Icon = item.icon;
                const dirty = !!dirtySections[item.section];
                return (
                  <button
                    key={item.section}
                    onClick={() => choose(index)}
                    onMouseEnter={() => setActiveIndex(index)}
                    className={`flex w-full items-center gap-2 rounded-md px-3 py-2 text-left text-sm ${
                      index === activeIndex ? "bg-primary text-primary-foreground" : "hover:bg-muted"
                    }`}
                  >
                    <Icon className="size-4" />
                    <span className="flex-1 truncate">{item.label}</span>
                    {dirty && (
                      <span
                        className={`size-1.5 shrink-0 rounded-full ${
                          index === activeIndex ? "bg-primary-foreground" : "bg-amber-500"
                        }`}
                      />
                    )}
                  </button>
                );
              })}
            </div>
          ))}
          {groups.length === 0 && <p className="p-3 text-sm text-muted-foreground">Keine Treffer.</p>}
        </div>
      </div>
    </div>
  );
}
