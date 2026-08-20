import { useState } from "react";
import { invoke } from "@tauri-apps/api/core";
import { Button } from "@/components/ui/button";
import { Search } from "lucide-react";
import type { ItemProtoInput, ItemSearchResult } from "../shared";

export function ReferenceItemPicker({ onPick }: { onPick: (item: ItemProtoInput) => void }) {
  const [query, setQuery] = useState("");
  const [results, setResults] = useState<ItemSearchResult[]>([]);
  const [searching, setSearching] = useState(false);
  const [pickError, setPickError] = useState<string | null>(null);

  async function run() {
    if (!query.trim()) return;
    setSearching(true);
    try {
      setResults(await invoke<ItemSearchResult[]>("search_items", { query: query.trim() }));
    } finally {
      setSearching(false);
    }
  }

  // "Übernehmen" must never fail silently: the user relies on these values
  // ending up in the import, so a null result or invoke error has to be
  // visible immediately - live-tested 2026-08-05, an import ran with all
  // values 0 because the picked reference was never actually applied.
  async function pick(vnum: number) {
    setPickError(null);
    try {
      const full = await invoke<ItemProtoInput | null>("get_item_proto", { vnum });
      if (!full) {
        setPickError(`Item ${vnum} konnte nicht geladen werden (nicht gefunden).`);
        return;
      }
      onPick(full);
      setResults([]);
      setQuery("");
    } catch (e) {
      setPickError(String(e));
    }
  }

  return (
    <div className="space-y-2">
      <div className="flex gap-2">
        <input
          value={query}
          onChange={(e) => setQuery(e.target.value)}
          onKeyDown={(e) => e.key === "Enter" && run()}
          placeholder="Item nach Name oder VNUM suchen…"
          className="flex-1 rounded-md border border-border bg-background px-2 py-1 text-sm"
        />
        <Button variant="outline" onClick={run} disabled={searching}>
          <Search className="size-4" />
        </Button>
      </div>
      {results.length > 0 && (
        <div className="max-h-40 space-y-1 overflow-y-auto">
          {results.map((r) => (
            <div key={r.vnum} className="flex items-center justify-between rounded-md px-2 py-1 text-sm hover:bg-muted">
              <span>
                {r.name} <span className="text-muted-foreground">#{r.vnum}</span>
              </span>
              <Button size="sm" variant="outline" onClick={() => pick(r.vnum)}>
                Übernehmen
              </Button>
            </div>
          ))}
        </div>
      )}
      {pickError && <p className="text-sm text-destructive">{pickError}</p>}
    </div>
  );
}
