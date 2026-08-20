import { useState } from "react";
import { invoke } from "@tauri-apps/api/core";
import { runAsyncAction } from "@/lib/asyncAction";
import { Button } from "@/components/ui/button";
import { Search } from "lucide-react";
import { PLAYER_TABLE, type TableRows } from "../shared";

// Kleine inline Spieler-Suche (player.player.name -> id), damit man beim
// Item-Geben nicht erst zur Spieler-Suche weiter oben auf der Seite scrollen
// und sich die ID merken muss.
export function MiniPlayerSearch({ onPick }: { onPick: (id: string, name: string) => void }) {
  const [open, setOpen] = useState(false);
  const [query, setQuery] = useState("");
  const [results, setResults] = useState<{ id: string; name: string }[]>([]);
  const [searching, setSearching] = useState(false);

  async function runSearch() {
    if (!query.trim()) return;
    await runAsyncAction(
      () =>
        invoke<TableRows>("search_table_rows", {
          database: PLAYER_TABLE.database,
          table: PLAYER_TABLE.table,
          column: "name",
          query: query.trim(),
        }),
      {
        onStart: () => setSearching(true),
        onSuccess: (r) => {
          const idIdx = r.columns.indexOf("id");
          const nameIdx = r.columns.indexOf("name");
          setResults(
            r.rows
              .filter((row) => row[idIdx] !== null)
              .map((row) => ({ id: row[idIdx] as string, name: nameIdx >= 0 ? (row[nameIdx] ?? "") : "" })),
          );
        },
        onError: () => setResults([]),
        onFinally: () => setSearching(false),
      },
    );
  }

  return (
    <div className="flex flex-col gap-1">
      <Button type="button" variant="outline" size="sm" onClick={() => setOpen((o) => !o)}>
        <Search className="size-3.5" /> Spieler suchen…
      </Button>
      {open && (
        <div className="w-64 space-y-1 rounded-md border border-border bg-card p-2 shadow-md">
          <div className="flex gap-1">
            <input
              autoFocus
              value={query}
              onChange={(e) => setQuery(e.target.value)}
              onKeyDown={(e) => e.key === "Enter" && runSearch()}
              placeholder="Charaktername…"
              className="flex-1 rounded-md border border-border bg-background px-2 py-1 text-sm"
            />
            <Button size="sm" variant="outline" onClick={runSearch} disabled={searching}>
              <Search className="size-3.5" />
            </Button>
          </div>
          {results.map((r) => (
            <button
              key={r.id}
              className="block w-full rounded px-2 py-1 text-left text-sm hover:bg-muted"
              onClick={() => {
                onPick(r.id, r.name);
                setOpen(false);
                setResults([]);
                setQuery("");
              }}
            >
              {r.name} <span className="text-muted-foreground">#{r.id}</span>
            </button>
          ))}
        </div>
      )}
    </div>
  );
}
