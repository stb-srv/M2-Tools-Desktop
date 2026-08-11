import { useEffect, useState } from "react";
import { invoke } from "@tauri-apps/api/core";
import { Button } from "@/components/ui/button";
import { Search, ChevronLeft, ChevronRight, Loader2 } from "lucide-react";

export interface EntityRow {
  vnum: number;
  name: string;
}

interface EntityBrowsePage {
  rows: EntityRow[];
  total: number;
}

const PAGE_SIZE = 20;

// Vorher hatten Item Editor/Aufwertungs-Editor/Mob-Proto-Editor je ihre
// eigene, reine Such-Anzeige: eine Zahlen-Suche wie "7000" ohne exakten
// Treffer lieferte schlicht nichts, keine Liste zum Durchblättern - selbst
// wenn direkt daneben (z.B. 70001, 70002...) etwas existierte. Diese
// Komponente ersetzt das durch eine immer gefüllte, paginierte Liste (leere
// Suche = kompletter Bestand nach vnum sortiert), die sich per Suchfeld
// eingrenzen lässt statt nur binär "gefunden"/"nichts".
export function EntityBrowser({
  kind,
  onPick,
  pickLabel = "Auswählen",
  autoFocus,
  maxHeightClass = "max-h-64",
  initialQuery,
  onRowsChange,
  renderLeading,
  renderTrailing,
}: {
  kind: "item" | "mob";
  onPick: (row: EntityRow) => void;
  pickLabel?: string;
  autoFocus?: boolean;
  maxHeightClass?: string;
  /** Pre-fills and immediately searches with this query on mount - e.g. the
   * Quest Builder's free-text assistant re-opens this picker with the
   * phrase it couldn't already resolve on its own. Only read once at mount
   * (this component is always freshly mounted when a picker opens, never
   * kept alive with a changing prop), so changing it after mount has no
   * effect. */
  initialQuery?: string;
  /** Fires whenever the currently displayed page of rows changes - lets a
   * caller side-load per-row extras (icons, status) the same way the
   * pre-migration hand-rolled pickers did, without EntityBrowser itself
   * needing to know anything about icons/vnum-specific status. */
  onRowsChange?: (rows: EntityRow[]) => void;
  /** Rendered before a row's name - e.g. an item icon. */
  renderLeading?: (row: EntityRow) => React.ReactNode;
  /** Rendered after a row's name/vnum, before the pick button - e.g. a
   * status badge. */
  renderTrailing?: (row: EntityRow) => React.ReactNode;
}) {
  const [query, setQuery] = useState(initialQuery ?? "");
  const [page, setPage] = useState(0);
  // Bumped by runSearch() so a manual re-search always re-triggers the
  // effect below, even when page is already 0 and wouldn't otherwise change.
  // Real bug this replaced: runSearch() used to fire its own separate
  // invoke() for that "already on page 0" case, without the effect's
  // `cancelled` stale-response guard - a second search fired while an
  // earlier one was still in flight could have its result overwritten by
  // the earlier one resolving later (out-of-order network timing). Routing
  // every search through the same effect means there's only one fetch path
  // to guard, not two to keep in sync.
  const [refreshKey, setRefreshKey] = useState(0);
  const [rows, setRows] = useState<EntityRow[]>([]);
  const [total, setTotal] = useState(0);
  const [loading, setLoading] = useState(false);
  const [error, setError] = useState<string | null>(null);

  const command = kind === "item" ? "browse_items" : "browse_mobs";

  useEffect(() => {
    let cancelled = false;
    setLoading(true);
    setError(null);
    invoke<EntityBrowsePage>(command, {
      query: query.trim() || null,
      offset: page * PAGE_SIZE,
      limit: PAGE_SIZE,
    })
      .then((result) => {
        if (cancelled) return;
        setRows(result.rows);
        setTotal(result.total);
      })
      .catch((e) => {
        if (cancelled) return;
        setError(String(e));
        setRows([]);
        setTotal(0);
      })
      .finally(() => {
        if (!cancelled) setLoading(false);
      });
    return () => {
      cancelled = true;
    };
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, [command, page, refreshKey]);

  useEffect(() => {
    onRowsChange?.(rows);
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, [rows]);

  function runSearch() {
    setPage(0);
    setRefreshKey((k) => k + 1);
  }

  const totalPages = Math.max(1, Math.ceil(total / PAGE_SIZE));

  return (
    <div className="space-y-2">
      <div className="flex gap-2">
        <input
          autoFocus={autoFocus}
          value={query}
          onChange={(e) => setQuery(e.target.value)}
          onKeyDown={(e) => e.key === "Enter" && runSearch()}
          placeholder="Nach Name oder VNUM filtern (leer = alle durchblättern)…"
          className="flex-1 rounded-md border border-border bg-background px-2 py-1 text-sm"
        />
        <Button variant="outline" onClick={runSearch} disabled={loading}>
          <Search className="size-4" />
        </Button>
      </div>

      {error && <p className="text-xs text-destructive">{error}</p>}

      <div className={`space-y-1 overflow-y-auto rounded-md border border-border ${maxHeightClass}`}>
        {loading && rows.length === 0 && (
          <p className="flex items-center gap-1.5 p-2 text-sm text-muted-foreground">
            <Loader2 className="size-3.5 animate-spin" /> Lade…
          </p>
        )}
        {!loading && rows.length === 0 && !error && (
          <p className="p-2 text-sm text-muted-foreground">Keine Treffer.</p>
        )}
        {rows.map((r) => (
          <div key={r.vnum} className="flex items-center justify-between gap-2 px-2 py-1 text-sm hover:bg-muted">
            <span className="flex min-w-0 items-center gap-2">
              {renderLeading?.(r)}
              <span className="truncate">
                {r.name || <span className="text-muted-foreground">(kein Name)</span>}{" "}
                <span className="text-muted-foreground">#{r.vnum}</span>
                {renderTrailing?.(r)}
              </span>
            </span>
            <Button size="sm" variant="outline" onClick={() => onPick(r)}>
              {pickLabel}
            </Button>
          </div>
        ))}
      </div>

      {total > PAGE_SIZE && (
        <div className="flex items-center justify-between text-xs text-muted-foreground">
          <span>
            {total.toLocaleString("de-DE")} Treffer · Seite {page + 1}/{totalPages}
          </span>
          <div className="flex gap-1">
            <Button
              variant="outline"
              size="icon-sm"
              onClick={() => setPage((p) => Math.max(0, p - 1))}
              disabled={page === 0 || loading}
            >
              <ChevronLeft className="size-3.5" />
            </Button>
            <Button
              variant="outline"
              size="icon-sm"
              onClick={() => setPage((p) => Math.min(totalPages - 1, p + 1))}
              disabled={page >= totalPages - 1 || loading}
            >
              <ChevronRight className="size-3.5" />
            </Button>
          </div>
        </div>
      )}
    </div>
  );
}
