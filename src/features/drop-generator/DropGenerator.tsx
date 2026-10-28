import { useEffect, useState } from "react";
import { cn } from "@/lib/utils";
import { MobDropEditor } from "@/features/mob-drop-editor/MobDropEditor";
import { BoxEditor } from "@/features/box-editor/BoxEditor";
import { CommonDropEditor } from "./CommonDropEditor";
import { EtcDropEditor } from "./EtcDropEditor";
import { DropItemGroupEditor } from "./DropItemGroupEditor";
import { PageHeader } from "@/components/layout/PageHeader";

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
// Editor) - hier eingebettet, nicht dupliziert. Seit 2026-10-25 nur noch
// hier erreichbar, nicht mehr zusätzlich als eigene Sidebar-Punkte.
export function DropGenerator() {
  const [tab, setTab] = useState<Tab>("mob");
  // Same "stays mounted once visited, just hidden" fix as App.tsx's
  // SectionSlot (see there for why) - these 5 tabs each do their own SFTP
  // load on mount, so switching tabs used to silently discard any unsaved
  // edit in the tab you left, exactly the bug this whole pass fixes.
  const [visited, setVisited] = useState<Set<Tab>>(() => new Set(["mob"]));
  useEffect(() => {
    setVisited((prev) => (prev.has(tab) ? prev : new Set(prev).add(tab)));
  }, [tab]);

  return (
    <div className="space-y-4 pb-10">
      <PageHeader module="drop-generator" title="Drop-Generator" />
      <p className="max-w-4xl text-sm text-muted-foreground">
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
        {visited.has("mob") && (
          <div className={cn(tab !== "mob" && "hidden")}>
            <MobDropEditor />
          </div>
        )}
        {visited.has("common") && (
          <div className={cn(tab !== "common" && "hidden")}>
            <CommonDropEditor />
          </div>
        )}
        {visited.has("etc") && (
          <div className={cn(tab !== "etc" && "hidden")}>
            <EtcDropEditor />
          </div>
        )}
        {visited.has("special") && (
          <div className={cn(tab !== "special" && "hidden")}>
            <BoxEditor />
          </div>
        )}
        {visited.has("group") && (
          <div className={cn(tab !== "group" && "hidden")}>
            <DropItemGroupEditor />
          </div>
        )}
      </div>
    </div>
  );
}
