import { useState } from "react";
import { Button } from "@/components/ui/button";
import { PawPrint, HelpCircle } from "lucide-react";
import { GenericRowEditor } from "@/features/shared/GenericRowEditor";
import { EntityBrowser } from "@/features/shared/EntityBrowser";
import { openManual } from "@/lib/manual";

export function MobProtoEditor() {
  const [editingVnum, setEditingVnum] = useState<number | null>(null);

  return (
    <div className="max-w-3xl space-y-4">
      <div className="flex items-center gap-2">
        <PawPrint className="size-6 text-muted-foreground" />
        <h1 className="text-2xl font-semibold">Mob-Proto-Editor</h1>
        <Button variant="ghost" size="icon-sm" title="Hilfe zu diesem Modul" onClick={() => openManual("mob-proto-editor")}>
          <HelpCircle className="size-4" />
        </Button>
      </div>
      <p className="text-sm text-muted-foreground">
        Bearbeitet <code>player.mob_proto</code> (Monster-/NPC-Stats) direkt in der Datenbank.
        Anders als der Item Editor zeigt dies jede Spalte generisch (Rohwerte, keine bekannten
        Flag-Bedeutungen wie AI-Verhalten) - Spaltensemantik für diese Tabelle wurde nicht gegen
        einen echten Server verifiziert. Neue Mobs anlegen ist bewusst nicht Teil dieser ersten
        Version. Änderungen wirken in aller Regel erst nach einem Server-Neustart (Server-Steuerung
        → Neustarten), da <code>mob_proto</code> typischerweise beim Serverstart in den Speicher
        geladen wird.
      </p>

      <EntityBrowser
        kind="mob"
        pickLabel="Bearbeiten"
        maxHeightClass="max-h-[28rem]"
        onPick={(r) => setEditingVnum(r.vnum)}
      />

      {editingVnum !== null && (
        <GenericRowEditor
          database="player"
          table="mob_proto"
          pkValue={String(editingVnum)}
          onClose={() => setEditingVnum(null)}
        />
      )}
    </div>
  );
}
