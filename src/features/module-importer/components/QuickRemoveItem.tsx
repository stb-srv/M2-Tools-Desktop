import { useState } from "react";
import { invoke } from "@tauri-apps/api/core";
import { logActivity } from "@/lib/logActivity";
import { Button } from "@/components/ui/button";
import { CheckCircle2, Search, Trash2 } from "lucide-react";
import type { ItemSearchResult } from "../shared";

export function QuickRemoveItem() {
  const [query, setQuery] = useState("");
  const [results, setResults] = useState<ItemSearchResult[]>([]);
  const [searching, setSearching] = useState(false);
  const [removingVnum, setRemovingVnum] = useState<number | null>(null);
  const [confirmVnum, setConfirmVnum] = useState<number | null>(null);
  const [lastRemoved, setLastRemoved] = useState<number | null>(null);
  const [error, setError] = useState<string | null>(null);

  async function run() {
    if (!query.trim()) return;
    setSearching(true);
    try {
      setResults(await invoke<ItemSearchResult[]>("search_items", { query: query.trim() }));
    } finally {
      setSearching(false);
    }
  }

  async function remove(vnum: number) {
    setConfirmVnum(null);
    setRemovingVnum(vnum);
    setError(null);
    try {
      await invoke("remove_single_item", { vnum });
      setResults((prev) => prev.filter((r) => r.vnum !== vnum));
      setLastRemoved(vnum);
      logActivity("module-importer", "remove-item", `Einzelnes importiertes Item entfernt (vnum ${vnum})`, "item", String(vnum));
    } catch (e) {
      setError(String(e));
    } finally {
      setRemovingVnum(null);
    }
  }

  return (
    <section className="space-y-2 rounded-lg border border-border p-4">
      <h2 className="text-sm font-medium text-muted-foreground">Einzelnes Item entfernen</h2>
      <p className="text-xs text-muted-foreground">
        Für Items, die nicht (mehr) im Verlauf oben stehen. Löscht Datenbankeintrag, Icon und{" "}
        <code>item_list.txt</code>-Eintrag - genau wie „Rückgängig machen" oben, nur für ein einzelnes Item.
      </p>
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
              <Button
                size="sm"
                variant="destructive"
                disabled={removingVnum === r.vnum}
                onClick={() => setConfirmVnum(r.vnum)}
              >
                <Trash2 className="size-3.5" />
                {removingVnum === r.vnum ? "Entferne…" : "Entfernen"}
              </Button>
            </div>
          ))}
        </div>
      )}
      {lastRemoved !== null && (
        <p className="flex items-center gap-1 text-sm text-green-600">
          <CheckCircle2 className="size-4" /> vnum {lastRemoved} vollständig entfernt.
        </p>
      )}
      {error && <p className="whitespace-pre-wrap text-sm text-destructive">{error}</p>}

      {confirmVnum !== null && (
        <div className="fixed inset-0 flex items-center justify-center bg-black/50">
          <div className="w-[24rem] space-y-3 rounded-lg border border-border bg-card p-4">
            <p className="text-sm font-medium">Item vnum {confirmVnum} endgültig entfernen?</p>
            <p className="text-xs text-muted-foreground">
              Datenbankeintrag, Icon und <code>item_list.txt</code>-Eintrag werden entfernt, <code>item_proto</code>{" "}
              wird neu erzeugt und im Client ersetzt.
            </p>
            <div className="flex justify-end gap-2">
              <Button variant="outline" onClick={() => setConfirmVnum(null)}>
                Abbrechen
              </Button>
              <Button variant="destructive" onClick={() => remove(confirmVnum)}>
                Endgültig entfernen
              </Button>
            </div>
          </div>
        </div>
      )}
    </section>
  );
}
