import { useEffect, useState, type ReactNode } from "react";
import { cn } from "@/lib/utils";

export interface SectionTab<T extends string> {
  id: T;
  label: string;
  content: ReactNode;
}

// Tab-Leiste für Bereiche, die mehrere früher eigenständige Module bündeln
// (z.B. Server-Events + Tag/Nacht & Schnee). Jeder Tab bleibt nach dem ersten
// Öffnen gemountet und wird nur versteckt - derselbe Grund wie bei App.tsx'
// SectionSlot: sonst gingen beim Tab-Wechsel ungespeicherte Eingaben verloren
// und jeder Tab würde bei jedem Wechsel neu laden.
export function SectionTabs<T extends string>({ tabs }: { tabs: SectionTab<T>[] }) {
  const [active, setActive] = useState<T>(tabs[0].id);
  const [visited, setVisited] = useState<Set<T>>(() => new Set([tabs[0].id]));
  useEffect(() => {
    setVisited((prev) => (prev.has(active) ? prev : new Set(prev).add(active)));
  }, [active]);

  return (
    <div className="flex h-full flex-col gap-4">
      <div className="flex flex-wrap gap-1 border-b border-border">
        {tabs.map((t) => (
          <button
            key={t.id}
            onClick={() => setActive(t.id)}
            className={`rounded-t-md px-3 py-2 text-sm font-medium transition-colors ${
              active === t.id
                ? "border-b-2 border-primary text-foreground"
                : "text-muted-foreground hover:text-foreground"
            }`}
          >
            {t.label}
          </button>
        ))}
      </div>
      <div className="min-h-0 flex-1">
        {tabs.map(
          (t) =>
            visited.has(t.id) && (
              <div key={t.id} className={cn("h-full", active !== t.id && "hidden")}>
                {t.content}
              </div>
            ),
        )}
      </div>
    </div>
  );
}
