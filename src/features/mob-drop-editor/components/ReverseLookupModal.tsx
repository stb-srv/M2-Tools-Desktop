import { useMemo, useState } from "react";
import { EntityBrowser } from "@/features/shared/EntityBrowser";
import type { MobDropGroup, MobDropItem, ItemSearchResult } from "../types";
import { Modal } from "./shared";

// "Wer droppt Item X" - rein clientseitig, da `groups` beim Öffnen bereits
// die komplette Datei enthält (ein einzelnes File, kein Mob-für-Mob-
// Nachladen nötig).
export function ReverseLookupModal({
  groups,
  realChance,
  onSelectMob,
  onClose,
}: {
  groups: MobDropGroup[];
  realChance: (percent: number) => string;
  onSelectMob: (index: number) => void;
  onClose: () => void;
}) {
  const [selectedItem, setSelectedItem] = useState<ItemSearchResult | null>(null);

  const matches = useMemo(() => {
    if (!selectedItem) return [];
    return groups
      .map((g, index) => ({ g, index, drop: g.items.find((it) => it.item_vnum === selectedItem.vnum) }))
      .filter((m): m is { g: MobDropGroup; index: number; drop: MobDropItem } => !!m.drop);
  }, [groups, selectedItem]);

  return (
    <Modal onClose={onClose}>
      <div className="space-y-2">
        <p className="text-sm font-medium">Wer droppt dieses Item?</p>

        {!selectedItem && (
          <EntityBrowser kind="item" pickLabel="Auswählen" autoFocus maxHeightClass="max-h-64" onPick={setSelectedItem} />
        )}

        {selectedItem && (
          <div className="space-y-2">
            <div className="flex items-center justify-between">
              <p className="text-sm">
                <strong>{selectedItem.name}</strong>{" "}
                <span className="text-muted-foreground">#{selectedItem.vnum}</span> wird von{" "}
                {matches.length} Mob(s) gedroppt:
              </p>
              <button
                className="text-xs text-muted-foreground underline hover:text-foreground"
                onClick={() => setSelectedItem(null)}
              >
                Anderes Item
              </button>
            </div>
            <div className="max-h-72 space-y-1 overflow-y-auto">
              {matches.map(({ g, index, drop }) => (
                <button
                  key={index}
                  onClick={() => onSelectMob(index)}
                  className="flex w-full items-center justify-between rounded-md border border-border px-2 py-1 text-left text-sm hover:bg-muted"
                >
                  <span>
                    {g.name} <span className="text-muted-foreground">(Mob #{g.mob_vnum})</span>
                  </span>
                  <span className="text-xs text-muted-foreground">
                    Anzahl {drop.count} · {drop.percent}% (≈{realChance(drop.percent)} real)
                  </span>
                </button>
              ))}
              {matches.length === 0 && (
                <p className="p-2 text-sm text-muted-foreground">Kein geladener Mob droppt dieses Item.</p>
              )}
            </div>
          </div>
        )}
      </div>
    </Modal>
  );
}
