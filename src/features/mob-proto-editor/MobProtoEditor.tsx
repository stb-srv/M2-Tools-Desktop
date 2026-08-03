import { useState } from "react";
import { invoke } from "@tauri-apps/api/core";
import { runAsyncAction } from "@/lib/asyncAction";
import { Button } from "@/components/ui/button";
import { Search, AlertTriangle, PawPrint } from "lucide-react";
import { GenericRowEditor } from "@/features/shared/GenericRowEditor";

interface MobSearchResult {
  vnum: number;
  name: string;
}

export function MobProtoEditor() {
  const [query, setQuery] = useState("");
  const [results, setResults] = useState<MobSearchResult[]>([]);
  const [searching, setSearching] = useState(false);
  const [searchError, setSearchError] = useState<string | null>(null);
  const [editingVnum, setEditingVnum] = useState<number | null>(null);

  async function runSearch() {
    if (!query.trim()) return;
    await runAsyncAction(
      () => invoke<MobSearchResult[]>("search_mobs", { query: query.trim() }),
      {
        onStart: () => {
          setSearching(true);
          setSearchError(null);
        },
        onSuccess: setResults,
        onError: setSearchError,
        onFinally: () => setSearching(false),
      },
    );
  }

  return (
    <div className="max-w-3xl space-y-4">
      <div className="flex items-center gap-2">
        <PawPrint className="size-6 text-muted-foreground" />
        <h1 className="text-2xl font-semibold">Mob-Proto-Editor</h1>
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

      <div className="flex gap-2">
        <input
          value={query}
          onChange={(e) => setQuery(e.target.value)}
          onKeyDown={(e) => e.key === "Enter" && runSearch()}
          placeholder="Mob/NPC nach Name oder VNUM suchen…"
          className="flex-1 rounded-md border border-border bg-background px-2 py-1 text-sm"
        />
        <Button variant="outline" onClick={runSearch} disabled={searching}>
          <Search className="size-4" />
        </Button>
      </div>

      {searchError && (
        <p className="flex items-start gap-2 text-sm text-destructive">
          <AlertTriangle className="mt-0.5 size-4 shrink-0" />
          <span>{searchError}</span>
        </p>
      )}

      <div className="space-y-1">
        {results.map((r) => (
          <div
            key={r.vnum}
            className="flex items-center justify-between rounded-md border border-border px-3 py-2 text-sm hover:bg-muted"
          >
            <span>
              {r.name} <span className="text-muted-foreground">#{r.vnum}</span>
            </span>
            <Button size="sm" variant="outline" onClick={() => setEditingVnum(r.vnum)}>
              Bearbeiten
            </Button>
          </div>
        ))}
        {results.length === 0 && !searching && (
          <p className="p-2 text-sm text-muted-foreground">Noch keine Suche.</p>
        )}
      </div>

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
