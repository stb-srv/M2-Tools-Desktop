import { EntityBrowser, type EntityRow } from "@/features/shared/EntityBrowser";
import { Modal } from "./shared";

export function ItemPickerModal({
  onClose,
  onPick,
  icons,
  onRowsChange,
}: {
  onClose: () => void;
  onPick: (item: EntityRow) => void;
  icons: Record<number, string | null>;
  onRowsChange: (rows: EntityRow[]) => void;
}) {
  return (
    <Modal onClose={onClose}>
      <EntityBrowser
        kind="item"
        pickLabel="Hinzufügen"
        autoFocus
        maxHeightClass="max-h-64"
        onPick={onPick}
        onRowsChange={onRowsChange}
        renderLeading={(r) =>
          icons[r.vnum] ? (
            <img src={icons[r.vnum]!} alt="" className="size-6 shrink-0 object-contain [image-rendering:pixelated]" />
          ) : null
        }
      />
    </Modal>
  );
}
