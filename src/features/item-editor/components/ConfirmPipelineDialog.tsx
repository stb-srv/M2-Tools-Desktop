import { Button } from "@/components/ui/button";
import type { Mode } from "../types";

export function ConfirmPipelineDialog({
  mode,
  itemVnum,
  hasNewIcon,
  hasModelCopy,
  refModelVnum,
  onCancel,
  onConfirm,
}: {
  mode: Mode;
  itemVnum: number;
  hasNewIcon: boolean;
  hasModelCopy: boolean;
  refModelVnum: number | null;
  onCancel: () => void;
  onConfirm: () => void;
}) {
  return (
    <div className="fixed inset-0 flex items-center justify-center bg-black/50">
      <div className="w-[28rem] space-y-3 rounded-lg border border-border bg-card p-4">
        <p className="text-sm font-medium">
          Item {itemVnum} jetzt {mode === "create" ? "anlegen" : "speichern"}?
        </p>
        <ul className="list-disc space-y-1 pl-5 text-sm text-muted-foreground">
          <li>
            Datenbankeintrag wird {mode === "create" ? "erstellt" : "aktualisiert"}
          </li>
          {hasNewIcon && (
            <>
              <li>Icon wird nach pack/icon/icon/item geschrieben</li>
              <li>
                <code>icon.epk</code> wird neu gepackt (Backup wird vorher angelegt)
              </li>
            </>
          )}
          {hasModelCopy && (
            <>
              <li>3D-Modell von vnum {refModelVnum} wird für vnum {itemVnum} kopiert</li>
              <li>
                <code>item.epk</code> wird neu gepackt (Backup wird vorher angelegt)
              </li>
            </>
          )}
          {hasNewIcon && (
            <li>
              <code>item_list.txt</code> wird um vnum {itemVnum} ergänzt/aktualisiert
              (Backup wird vorher angelegt)
            </li>
          )}
          <li>
            <code>item_proto</code> wird aus der DB neu erzeugt und im Client ersetzt (Backup
            wird vorher angelegt)
          </li>
        </ul>
        <div className="flex justify-end gap-2">
          <Button variant="outline" onClick={onCancel}>
            Abbrechen
          </Button>
          <Button onClick={onConfirm}>Ausführen</Button>
        </div>
      </div>
    </div>
  );
}
