import { useState } from "react";
import { Button } from "@/components/ui/button";
import { Info } from "lucide-react";
import { SERVER_NOTES } from "../serverNotes";

// Self-contained trigger + popover ("Wie Prozentwerte auf diesem Server
// wirken") - no state is needed outside this component.
export function ServerInfoPopover() {
  const [open, setOpen] = useState(false);

  return (
    <div className="relative">
      <Button
        variant="ghost"
        size="icon-sm"
        onClick={() => setOpen((v) => !v)}
        title="Wie Prozentwerte auf diesem Server wirken"
      >
        <Info className="size-4" />
      </Button>
      {open && (
        <div className="absolute left-0 top-full z-10 mt-1 w-96 space-y-3 rounded-lg border border-border bg-card p-3 shadow-lg">
          {SERVER_NOTES.map((note, i) => (
            <div key={i} className="space-y-1.5">
              <p className="text-sm font-semibold">{note.title}</p>
              {note.intro && <p className="text-xs text-muted-foreground">{note.intro}</p>}
              {note.formula && (
                <p className="rounded-md bg-muted/60 px-2 py-1 text-xs font-medium">{note.formula}</p>
              )}
              {note.bullets && (
                <ul className="list-disc space-y-1 pl-4 text-xs text-muted-foreground">
                  {note.bullets.map((b, bi) => (
                    <li key={bi}>{b}</li>
                  ))}
                </ul>
              )}
            </div>
          ))}
          <Button variant="outline" size="sm" onClick={() => setOpen(false)}>
            Schließen
          </Button>
        </div>
      )}
    </div>
  );
}
