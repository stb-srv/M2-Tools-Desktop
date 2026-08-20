import { Button } from "@/components/ui/button";
import { Search, Plus } from "lucide-react";
import type { MobDropGroup } from "../types";

export function GroupSidebar({
  filteredGroups,
  groupSearch,
  onSearchChange,
  selectedIndex,
  onSelect,
  onCreateClick,
}: {
  filteredGroups: { g: MobDropGroup; index: number }[];
  groupSearch: string;
  onSearchChange: (value: string) => void;
  selectedIndex: number | null;
  onSelect: (index: number) => void;
  onCreateClick: () => void;
}) {
  return (
    <div className="flex w-72 shrink-0 flex-col gap-2">
      <div className="flex gap-2">
        <div className="relative flex-1">
          <Search className="absolute left-2 top-1/2 size-3.5 -translate-y-1/2 text-muted-foreground" />
          <input
            value={groupSearch}
            onChange={(e) => onSearchChange(e.target.value)}
            placeholder="Mob suchen (Name/VNUM)…"
            className="w-full rounded-md border border-border bg-background py-1 pl-7 pr-2 text-sm"
          />
        </div>
        <Button variant="outline" size="icon" onClick={onCreateClick}>
          <Plus className="size-4" />
        </Button>
      </div>
      <div className="flex-1 space-y-1 overflow-y-auto">
        {filteredGroups.map(({ g, index }) => (
          <div
            key={index}
            onClick={() => onSelect(index)}
            className={`cursor-pointer rounded-md border border-border p-2 text-sm hover:bg-muted ${
              selectedIndex === index ? "bg-muted" : ""
            }`}
          >
            <div className="font-medium">{g.name}</div>
            <div className="text-xs text-muted-foreground">
              Mob: {g.mob_vnum} · {g.items.length} Drops
            </div>
          </div>
        ))}
        {filteredGroups.length === 0 && (
          <p className="p-2 text-sm text-muted-foreground">Keine Mobs in der Datei.</p>
        )}
      </div>
    </div>
  );
}
