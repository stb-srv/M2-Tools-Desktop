import { useEffect, useState } from "react";
import { invoke } from "@tauri-apps/api/core";
import { Button } from "@/components/ui/button";
import { Search, ChevronLeft, ChevronRight, Loader2, HelpCircle, LayoutGrid } from "lucide-react";
import { openManual } from "@/lib/manual";
import { useNavigationStore } from "@/store/navigation";
import { ITEM_TYPES } from "@/features/item-editor/itemFlags";

interface ItemProtoSummary {
  vnum: number;
  name: string;
  item_type: number;
  subtype: number;
}

interface ItemProtoPage {
  rows: ItemProtoSummary[];
  total: number;
}

const PAGE_SIZE_OPTIONS = [24, 48, 96, 192] as const;
const TYPE_LABEL: Record<number, string> = Object.fromEntries(ITEM_TYPES.map((t) => [t.value, t.label]));

// Eigenständige Karten-/Grid-Ansicht aller Items mit großen Icons - anders
// als der Item-Proto-Explorer (Tabelle mit kleinem Icon pro Zeile, für
// schnelles Durchblättern vieler Datenfelder) geht es hier nur ums visuelle
// Erkennen: großes Icon, VNUM und Name auf einen Blick, zum Suchen "wie sieht
// das Item eigentlich aus".
export function ItemViewer() {
  const setSection = useNavigationStore((s) => s.setSection);

  const [query, setQuery] = useState("");
  const [typeFilter, setTypeFilter] = useState<string>("");
  const [page, setPage] = useState(0);
  const [pageSize, setPageSize] = useState<number>(48);
  const [refreshKey, setRefreshKey] = useState(0);

  const [data, setData] = useState<ItemProtoPage | null>(null);
  const [loading, setLoading] = useState(false);
  const [error, setError] = useState<string | null>(null);
  const [icons, setIcons] = useState<Record<number, string | null>>({});

  useEffect(() => {
    let cancelled = false;
    setLoading(true);
    setError(null);
    invoke<ItemProtoPage>("browse_item_proto", {
      query: query.trim() || null,
      typeFilter: typeFilter === "" ? null : Number(typeFilter),
      offset: page * pageSize,
      limit: pageSize,
    })
      .then((result) => {
        if (cancelled) return;
        setData(result);
      })
      .catch((e) => {
        if (cancelled) return;
        setError(String(e));
        setData(null);
      })
      .finally(() => {
        if (!cancelled) setLoading(false);
      });
    return () => {
      cancelled = true;
    };
  }, [query, typeFilter, page, pageSize, refreshKey]);

  useEffect(() => {
    if (!data) return;
    const missing = [...new Set(data.rows.map((r) => r.vnum))].filter((v) => !(v in icons));
    missing.forEach((vnum) => {
      invoke<string | null>("get_item_icon", { vnum })
        .then((dataUrl) => setIcons((prev) => ({ ...prev, [vnum]: dataUrl })))
        .catch(() => setIcons((prev) => ({ ...prev, [vnum]: null })));
    });
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, [data]);

  function runSearch() {
    setPage(0);
    setRefreshKey((k) => k + 1);
  }

  function changePageSize(next: number) {
    setPageSize(next);
    setPage(0);
  }

  const totalPages = data ? Math.max(1, Math.ceil(data.total / pageSize)) : 1;

  return (
    <div className="flex h-full flex-col gap-3">
      <div className="flex items-center gap-2">
        <LayoutGrid className="size-6 text-muted-foreground" />
        <h1 className="text-2xl font-semibold">Item-Viewer</h1>
        <Button variant="ghost" size="icon-sm" title="Hilfe zu diesem Modul" onClick={() => openManual("item-viewer")}>
          <HelpCircle className="size-4" />
        </Button>
      </div>
      <p className="text-sm text-muted-foreground">
        Alle Items als Karten mit großem Icon, VNUM und Name - zum Durchsuchen und Wiedererkennen. Für die
        tabellarische Ansicht mit allen Feldern siehe Item-Proto-Explorer.
      </p>

      <div className="flex flex-wrap items-center gap-2">
        <div className="relative flex-1 min-w-48">
          <Search className="absolute left-2 top-1/2 size-3.5 -translate-y-1/2 text-muted-foreground" />
          <input
            value={query}
            onChange={(e) => setQuery(e.target.value)}
            onKeyDown={(e) => e.key === "Enter" && runSearch()}
            placeholder="Nach Name oder VNUM filtern…"
            className="w-full rounded-md border border-border bg-background py-1.5 pl-7 pr-2 text-sm"
          />
        </div>
        <select
          value={typeFilter}
          onChange={(e) => {
            setTypeFilter(e.target.value);
            setPage(0);
          }}
          className="rounded-md border border-border bg-background px-2 py-1.5 text-sm"
        >
          <option value="">Alle Typen</option>
          {ITEM_TYPES.map((t) => (
            <option key={t.value} value={t.value}>
              {t.label}
            </option>
          ))}
        </select>
        <Button variant="outline" onClick={runSearch} disabled={loading}>
          <Search className="size-4" />
        </Button>
      </div>

      {error && <p className="text-sm text-destructive">{error}</p>}

      <div className="min-h-0 flex-1 overflow-y-auto rounded-md border border-border p-2">
        {loading && !data && (
          <div className="flex h-full items-center justify-center">
            <Loader2 className="size-5 animate-spin text-muted-foreground" />
          </div>
        )}
        {data && data.rows.length === 0 && !loading && (
          <p className="p-4 text-center text-sm text-muted-foreground">Keine Treffer.</p>
        )}
        {data && data.rows.length > 0 && (
          <div className="grid grid-cols-3 gap-2 sm:grid-cols-4 md:grid-cols-6 lg:grid-cols-8">
            {data.rows.map((r) => (
              <div
                key={r.vnum}
                className="flex flex-col items-center gap-1 rounded-md border border-border p-2 hover:bg-muted"
              >
                <div className="flex size-16 shrink-0 items-center justify-center rounded-md bg-muted/40">
                  {icons[r.vnum] ? (
                    <img
                      src={icons[r.vnum]!}
                      alt=""
                      className="max-h-full max-w-full object-contain [image-rendering:pixelated]"
                    />
                  ) : (
                    <span className="text-[10px] text-muted-foreground">#{r.vnum}</span>
                  )}
                </div>
                <span className="w-full truncate text-center text-xs font-medium" title={r.name}>
                  {r.name || <span className="text-muted-foreground">(kein Name)</span>}
                </span>
                <span className="text-[10px] text-muted-foreground">
                  #{r.vnum} · {TYPE_LABEL[r.item_type] ?? r.item_type}
                </span>
                <Button variant="outline" size="sm" className="mt-1 w-full" onClick={() => setSection("item-editor")}>
                  Bearbeiten
                </Button>
              </div>
            ))}
          </div>
        )}
      </div>

      {data && data.total > 0 && (
        <div className="flex items-center justify-between text-xs text-muted-foreground">
          <span>
            {data.total.toLocaleString("de-DE")} Treffer
            {totalPages > 1 && ` · Seite ${page + 1}/${totalPages}`}
          </span>
          <div className="flex items-center gap-2">
            <label className="flex items-center gap-1">
              pro Seite
              <select
                value={pageSize}
                onChange={(e) => changePageSize(Number(e.target.value))}
                className="rounded-md border border-border bg-background px-1 py-0.5 text-xs"
              >
                {PAGE_SIZE_OPTIONS.map((size) => (
                  <option key={size} value={size}>
                    {size}
                  </option>
                ))}
              </select>
            </label>
            {totalPages > 1 && (
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
            )}
          </div>
        </div>
      )}
    </div>
  );
}
