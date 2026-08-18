import { useState } from "react";
import { Button } from "@/components/ui/button";
import { Dices, HelpCircle } from "lucide-react";
import { openManual } from "@/lib/manual";
import { MobDropEditor } from "@/features/mob-drop-editor/MobDropEditor";
import { BoxEditor } from "@/features/box-editor/BoxEditor";
import { CommonDropEditor } from "./CommonDropEditor";
import { EtcDropEditor } from "./EtcDropEditor";
import { DropItemGroupEditor } from "./DropItemGroupEditor";

type Tab = "mob" | "common" | "etc" | "special" | "group";

const TABS: { id: Tab; label: string }[] = [
  { id: "mob", label: "Mob-Drops" },
  { id: "common", label: "Common-Drops" },
  { id: "etc", label: "Etc-Drops" },
  { id: "special", label: "Item-Gruppen" },
  { id: "group", label: "Zufalls-Gruppen" },
];

// Bündelt alle 5 Drop-relevanten Server-Dateien unter einem Dach. Mob-Drops
// (mob_drop_item.txt) und Item-Gruppen (special_item_group.txt) sind die
// bereits bestehenden, eigenständigen Editoren (Mob Drop Editor/Kisten-
// Editor) - hier unverändert eingebettet, nicht dupliziert. Beide bleiben
// zusätzlich als eigene Sidebar-Punkte erreichbar (Nutzerentscheidung).
export function DropGenerator() {
  const [tab, setTab] = useState<Tab>("mob");

  return (
    <div className="max-w-5xl space-y-4 pb-10">
      <div className="flex items-center gap-2">
        <Dices className="size-6 text-muted-foreground" />
        <h1 className="text-2xl font-semibold">Drop-Generator</h1>
        <Button variant="ghost" size="icon-sm" title="Hilfe zu diesem Modul" onClick={() => openManual("drop-generator")}>
          <HelpCircle className="size-4" />
        </Button>
      </div>
      <p className="text-sm text-muted-foreground">
        Alle 5 Drop-relevanten Server-Dateien an einem Ort. Jeder Tab erklärt oben kurz, wofür seine Datei
        zuständig ist und wie sie sich von den anderen unterscheidet - ausführlicher im Handbuch (Hilfe-Knopf
        oben).
      </p>

      <div className="flex flex-wrap gap-1 border-b border-border">
        {TABS.map((t) => (
          <button
            key={t.id}
            onClick={() => setTab(t.id)}
            className={`rounded-t-md px-3 py-2 text-sm font-medium transition-colors ${
              tab === t.id
                ? "border-b-2 border-primary text-foreground"
                : "text-muted-foreground hover:text-foreground"
            }`}
          >
            {t.label}
          </button>
        ))}
      </div>

      <div>
        {tab === "mob" && <MobDropEditor />}
        {tab === "common" && <CommonDropEditor />}
        {tab === "etc" && <EtcDropEditor />}
        {tab === "special" && <BoxEditor />}
        {tab === "group" && <DropItemGroupEditor />}
      </div>
    </div>
  );
}
