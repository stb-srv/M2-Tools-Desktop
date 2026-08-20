import { Button } from "@/components/ui/button";
import { Plus, Trash2, X } from "lucide-react";
import type { MobDropGroup, MobDropItem } from "../types";
import { Field, clampPercent } from "./shared";

export function MobDetailPanel({
  selected,
  selectedIndex,
  icons,
  realChance,
  onUpdateGroup,
  onUpdateItem,
  onRemoveItem,
  onDeleteClick,
  onAddItemClick,
}: {
  selected: MobDropGroup;
  selectedIndex: number;
  icons: Record<number, string | null>;
  realChance: (percent: number) => string;
  onUpdateGroup: (index: number, patch: Partial<MobDropGroup>) => void;
  onUpdateItem: (groupIndex: number, itemIndex: number, patch: Partial<MobDropItem>) => void;
  onRemoveItem: (groupIndex: number, itemIndex: number) => void;
  onDeleteClick: () => void;
  onAddItemClick: () => void;
}) {
  return (
    <>
      <div className="flex items-start justify-between rounded-lg border border-border p-3">
        <div className="flex flex-wrap items-end gap-3">
          <Field label="Gruppenname">
            <input
              value={selected.name}
              onChange={(e) => onUpdateGroup(selectedIndex, { name: e.target.value })}
              className="w-48 rounded-md border border-border bg-background px-2 py-1 text-sm"
            />
          </Field>
          <Field label="Mob-VNUM">
            <input
              type="number"
              value={selected.mob_vnum}
              onChange={(e) => onUpdateGroup(selectedIndex, { mob_vnum: Number(e.target.value) || 0 })}
              className="w-24 rounded-md border border-border bg-background px-2 py-1 text-sm"
            />
          </Field>
          <Field label="Type">
            <input
              value={selected.drop_type}
              onChange={(e) => onUpdateGroup(selectedIndex, { drop_type: e.target.value })}
              className="w-24 rounded-md border border-border bg-background px-2 py-1 text-sm"
            />
          </Field>
        </div>
        <Button variant="destructive" size="sm" onClick={onDeleteClick}>
          <Trash2 className="size-4" />
          Mob löschen
        </Button>
      </div>

      <div className="space-y-2 rounded-lg border border-border p-3">
        <div className="flex items-center justify-between">
          <h2 className="text-sm font-medium text-muted-foreground">Drops ({selected.items.length})</h2>
          <Button size="sm" variant="outline" onClick={onAddItemClick}>
            <Plus className="size-3.5" />
            Item hinzufügen
          </Button>
        </div>
        <div className="space-y-1">
          {selected.items.map((item, itemIndex) => (
            <div key={itemIndex} className="flex items-center gap-3 rounded-md border border-border p-2">
              <div className="flex size-9 shrink-0 items-center justify-center rounded-md border border-border bg-muted/40">
                {icons[item.item_vnum] ? (
                  <img
                    src={icons[item.item_vnum]!}
                    alt=""
                    className="max-h-full w-7 object-contain [image-rendering:pixelated]"
                  />
                ) : (
                  <span className="text-[10px] text-muted-foreground">{item.item_vnum}</span>
                )}
              </div>
              <span className="w-20 shrink-0 text-xs text-muted-foreground">#{item.item_vnum}</span>
              <Field label="Anzahl">
                <input
                  type="number"
                  min={1}
                  value={item.count}
                  onChange={(e) =>
                    onUpdateItem(selectedIndex, itemIndex, {
                      count: Math.max(1, Number(e.target.value) || 1),
                    })
                  }
                  className="w-16 rounded-md border border-border bg-background px-2 py-1 text-sm"
                />
              </Field>
              <Field label="Prozent">
                <input
                  type="number"
                  min={0}
                  max={100}
                  step={0.01}
                  value={item.percent}
                  onChange={(e) =>
                    onUpdateItem(selectedIndex, itemIndex, {
                      percent: clampPercent(Number(e.target.value) || 0),
                    })
                  }
                  className="w-20 rounded-md border border-border bg-background px-2 py-1 text-sm"
                />
              </Field>
              <span
                className="text-xs text-muted-foreground"
                title="Reale Drop-Chance auf diesem Server (siehe i-Info oben)"
              >
                ≈{realChance(item.percent)}
              </span>
              <Button
                variant="ghost"
                size="icon-sm"
                className="ml-auto"
                onClick={() => onRemoveItem(selectedIndex, itemIndex)}
              >
                <X className="size-4" />
              </Button>
            </div>
          ))}
          {selected.items.length === 0 && <p className="text-sm text-muted-foreground">Noch keine Drops.</p>}
        </div>
      </div>
    </>
  );
}
