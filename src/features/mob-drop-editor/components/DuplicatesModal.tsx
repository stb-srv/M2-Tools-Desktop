import { Button } from "@/components/ui/button";
import { Pencil, X } from "lucide-react";
import type { MobDropGroup } from "../types";
import type { DuplicateItemFinding, DuplicateMobFinding } from "../duplicates";
import { Modal } from "./shared";

// Zeigt beide Duplikat-Prüfungen (siehe duplicates.ts) und bietet je Fund
// zwei Aktionen: "Anpassen" springt zum betroffenen Mob im Haupteditor
// (Item-VNUM ist dort nirgends inline editierbar, daher kein eigener
// Bearbeiten-Dialog hier - Muster wie ReverseLookupModal.onSelectMob),
// "Löschen" entfernt genau diese eine Dopplung.
export function DuplicatesModal({
  duplicateItems,
  duplicateMobs,
  groups,
  icons,
  onNavigateToGroup,
  onDeleteItemOccurrence,
  onDeleteGroup,
  onClose,
}: {
  duplicateItems: DuplicateItemFinding[];
  duplicateMobs: DuplicateMobFinding[];
  groups: MobDropGroup[];
  icons: Record<number, string | null>;
  onNavigateToGroup: (groupIndex: number) => void;
  onDeleteItemOccurrence: (groupIndex: number, itemIndex: number) => void;
  onDeleteGroup: (groupIndex: number) => void;
  onClose: () => void;
}) {
  return (
    <Modal onClose={onClose}>
      <div className="max-h-[70vh] w-[32rem] space-y-4 overflow-y-auto">
        <p className="text-sm font-medium">Duplikate</p>

        <div className="space-y-2">
          <p className="text-xs font-medium text-muted-foreground">
            Doppelte Items pro Mob ({duplicateItems.length})
          </p>
          {duplicateItems.length === 0 && (
            <p className="text-sm text-muted-foreground">Keine doppelten Items in einem Mob gefunden.</p>
          )}
          {duplicateItems.map((finding) => (
            <div
              key={`${finding.groupIndex}-${finding.itemVnum}`}
              className="space-y-1 rounded-md border border-border p-2"
            >
              <div className="flex items-center justify-between">
                <span className="text-sm">
                  {finding.mobName} <span className="text-muted-foreground">(Mob #{finding.mobVnum})</span> - Item #
                  {finding.itemVnum} {finding.itemIndices.length}× eingetragen
                </span>
                <Button
                  size="sm"
                  variant="outline"
                  onClick={() => onNavigateToGroup(finding.groupIndex)}
                >
                  <Pencil className="size-3.5" />
                  Anpassen
                </Button>
              </div>
              <div className="flex flex-wrap gap-1">
                {finding.itemIndices.map((itemIndex, occurrence) => (
                  <button
                    key={itemIndex}
                    onClick={() => onDeleteItemOccurrence(finding.groupIndex, itemIndex)}
                    className="flex items-center gap-1 rounded-md border border-border bg-muted/40 px-2 py-1 text-xs hover:bg-destructive/10 hover:text-destructive"
                  >
                    {icons[finding.itemVnum] ? (
                      <img
                        src={icons[finding.itemVnum]!}
                        alt=""
                        className="size-4 object-contain [image-rendering:pixelated]"
                      />
                    ) : null}
                    Eintrag {occurrence + 1} ({groups[finding.groupIndex]?.items[itemIndex]?.percent}%)
                    <X className="size-3" />
                  </button>
                ))}
              </div>
            </div>
          ))}
        </div>

        <div className="space-y-2">
          <p className="text-xs font-medium text-muted-foreground">
            Doppelte Mob-Einträge ({duplicateMobs.length})
          </p>
          {duplicateMobs.length === 0 && (
            <p className="text-sm text-muted-foreground">Keine mehrfach vorkommenden Mobs gefunden.</p>
          )}
          {duplicateMobs.map((finding) => (
            <div key={finding.mobVnum} className="space-y-1 rounded-md border border-border p-2">
              <p className="text-sm">
                Mob #{finding.mobVnum} kommt {finding.groupIndices.length}× als eigener Eintrag vor
              </p>
              <div className="space-y-1">
                {finding.groupIndices.map((groupIndex) => {
                  const g = groups[groupIndex];
                  return (
                    <div
                      key={groupIndex}
                      className="flex items-center justify-between rounded-md border border-border bg-muted/40 px-2 py-1 text-xs"
                    >
                      <span>
                        {g?.name} · {g?.drop_type} · {g?.items.length ?? 0} Drops
                      </span>
                      <div className="flex items-center gap-1">
                        <Button size="sm" variant="outline" onClick={() => onNavigateToGroup(groupIndex)}>
                          <Pencil className="size-3.5" />
                          Anpassen
                        </Button>
                        <Button
                          size="sm"
                          variant="destructive"
                          onClick={() => onDeleteGroup(groupIndex)}
                        >
                          <X className="size-3.5" />
                          Löschen
                        </Button>
                      </div>
                    </div>
                  );
                })}
              </div>
            </div>
          ))}
        </div>
      </div>
    </Modal>
  );
}
