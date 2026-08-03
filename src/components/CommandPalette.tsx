import { useEffect, useMemo, useRef, useState } from "react";
import { useTranslation } from "react-i18next";
import { useNavigationStore, NAV_ITEMS } from "@/store/navigation";
import { Search } from "lucide-react";

// Global Ctrl+K/Cmd+K launcher for jumping straight to a section without
// hunting through the sidebar - reuses NAV_ITEMS (the same list the Sidebar
// renders) so the two can never drift out of sync.
export function CommandPalette() {
  const { t } = useTranslation();
  const setSection = useNavigationStore((s) => s.setSection);
  const [open, setOpen] = useState(false);
  const [query, setQuery] = useState("");
  const [activeIndex, setActiveIndex] = useState(0);
  const inputRef = useRef<HTMLInputElement>(null);

  const results = useMemo(() => {
    const q = query.trim().toLowerCase();
    const items = NAV_ITEMS.map((item) => ({ ...item, label: t(item.labelKey) }));
    if (!q) return items;
    return items.filter((item) => item.label.toLowerCase().includes(q));
  }, [query, t]);

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
    const item = results[index];
    if (!item) return;
    setSection(item.section);
    setOpen(false);
  }

  function handleKeyDown(e: React.KeyboardEvent) {
    if (e.key === "ArrowDown") {
      e.preventDefault();
      setActiveIndex((i) => Math.min(i + 1, results.length - 1));
    } else if (e.key === "ArrowUp") {
      e.preventDefault();
      setActiveIndex((i) => Math.max(i - 1, 0));
    } else if (e.key === "Enter") {
      e.preventDefault();
      choose(activeIndex);
    }
  }

  if (!open) return null;

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
        <div className="max-h-80 overflow-y-auto p-1">
          {results.map((item, index) => {
            const Icon = item.icon;
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
                {item.label}
              </button>
            );
          })}
          {results.length === 0 && (
            <p className="p-3 text-sm text-muted-foreground">Keine Treffer.</p>
          )}
        </div>
      </div>
    </div>
  );
}
