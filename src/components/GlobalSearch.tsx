import { useEffect, useRef, useState } from "react";
import { invoke } from "@tauri-apps/api/core";
import { useNavigationStore } from "@/store/navigation";
import { Search, Package, PawPrint, ScrollText, Loader2 } from "lucide-react";

interface EntityHit {
  vnum: number;
  name: string;
}

interface QuestSearchLine {
  line_number: number;
  text: string;
}

interface QuestSearchMatch {
  relative_path: string;
  category: string;
  name: string;
  lines: QuestSearchLine[];
}

type ResultGroup =
  | { kind: "item"; label: string; icon: typeof Package; items: EntityHit[] }
  | { kind: "mob"; label: string; icon: typeof PawPrint; items: EntityHit[] }
  | { kind: "quest"; label: string; icon: typeof ScrollText; items: QuestSearchMatch[] };

// Zweiter globaler Modal-Launcher neben CommandPalette.tsx (Strg+K bleibt für
// reine Bereichs-Navigation), gleiches Schalen-Muster (fixed backdrop,
// max-w-lg-Karte, kein Dialog-Primitive) - dieser hier durchsucht echte Daten
// (Items/Mobs/Quests) statt nur NAV_ITEMS, und trägt beim Auswählen eines
// Treffers eine vnum/einen Pfad ins Zielmodul (siehe navigation.ts'
// `goToWithSelection`/`pendingSelection` - vorher gab es dafür im ganzen
// Projekt keinen Mechanismus, siehe [[m2manager_activity_log]]-Nachfolgeplan).
export function GlobalSearch() {
  const goToWithSelection = useNavigationStore((s) => s.goToWithSelection);
  const [open, setOpen] = useState(false);
  const [query, setQuery] = useState("");
  const [loading, setLoading] = useState(false);
  const [groups, setGroups] = useState<ResultGroup[]>([]);
  const inputRef = useRef<HTMLInputElement>(null);

  useEffect(() => {
    function handleGlobalKeyDown(e: KeyboardEvent) {
      if ((e.metaKey || e.ctrlKey) && e.shiftKey && e.key.toLowerCase() === "f") {
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
      setGroups([]);
      requestAnimationFrame(() => inputRef.current?.focus());
    }
  }, [open]);

  async function runSearch() {
    const q = query.trim();
    if (!q) return;
    setLoading(true);
    const [items, mobs, quests] = await Promise.allSettled([
      invoke<EntityHit[]>("search_items", { query: q }),
      invoke<EntityHit[]>("search_mobs", { query: q }),
      invoke<QuestSearchMatch[]>("search_quest_files", { query: q }),
    ]);
    const next: ResultGroup[] = [];
    if (items.status === "fulfilled" && items.value.length) {
      next.push({ kind: "item", label: "Items", icon: Package, items: items.value });
    }
    if (mobs.status === "fulfilled" && mobs.value.length) {
      next.push({ kind: "mob", label: "Mobs", icon: PawPrint, items: mobs.value });
    }
    if (quests.status === "fulfilled" && quests.value.length) {
      next.push({ kind: "quest", label: "Quests", icon: ScrollText, items: quests.value });
    }
    setGroups(next);
    setLoading(false);
  }

  function pickItem(vnum: number) {
    goToWithSelection("item-editor", String(vnum));
    setOpen(false);
  }
  function pickMob(vnum: number) {
    goToWithSelection("mob-proto-editor", String(vnum));
    setOpen(false);
  }
  function pickQuest(match: QuestSearchMatch) {
    goToWithSelection("quest-builder", match.relative_path);
    setOpen(false);
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
            onKeyDown={(e) => e.key === "Enter" && runSearch()}
            placeholder="Items, Mobs, Quests durchsuchen… (Strg+Umschalt+F)"
            className="w-full bg-transparent py-1 text-sm outline-none"
          />
          {loading && <Loader2 className="size-4 shrink-0 animate-spin text-muted-foreground" />}
        </div>
        <div className="max-h-96 overflow-y-auto p-1">
          {groups.map((group) => (
            <div key={group.kind}>
              <div className="flex items-center gap-1 px-3 pb-1 pt-2 text-xs font-semibold uppercase tracking-wide text-muted-foreground">
                <group.icon className="size-3.5" />
                {group.label}
              </div>
              {group.kind === "quest"
                ? group.items.map((m) => (
                    <button
                      key={m.relative_path}
                      onClick={() => pickQuest(m)}
                      className="flex w-full flex-col items-start gap-0.5 rounded-md px-3 py-2 text-left text-sm hover:bg-muted"
                    >
                      <span className="truncate font-medium">{m.relative_path}</span>
                      {m.lines[0] && (
                        <span className="truncate text-xs text-muted-foreground">
                          Zeile {m.lines[0].line_number}: {m.lines[0].text.trim()}
                        </span>
                      )}
                    </button>
                  ))
                : group.items.map((r) => (
                    <button
                      key={r.vnum}
                      onClick={() => (group.kind === "item" ? pickItem(r.vnum) : pickMob(r.vnum))}
                      className="flex w-full items-center justify-between gap-2 rounded-md px-3 py-2 text-left text-sm hover:bg-muted"
                    >
                      <span className="truncate">{r.name || <span className="text-muted-foreground">(kein Name)</span>}</span>
                      <span className="shrink-0 text-xs text-muted-foreground">#{r.vnum}</span>
                    </button>
                  ))}
            </div>
          ))}
          {!loading && groups.length === 0 && query.trim() && (
            <p className="p-3 text-sm text-muted-foreground">Keine Treffer.</p>
          )}
          {!query.trim() && (
            <p className="p-3 text-sm text-muted-foreground">
              Suchbegriff eingeben und Enter drücken - durchsucht Items, Mobs und Quest-Dateien
              gleichzeitig.
            </p>
          )}
        </div>
      </div>
    </div>
  );
}
