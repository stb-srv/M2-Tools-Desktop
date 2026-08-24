import { useEffect, useState } from "react";
import { invoke } from "@tauri-apps/api/core";
import { Button } from "@/components/ui/button";
import { Search, ChevronLeft, ChevronRight, Loader2, HelpCircle, Download, Table2, Link2 } from "lucide-react";
import { openManual } from "@/lib/manual";
import { exportRowsAsCsv } from "@/lib/exportCsv";
import { useNavigationStore } from "@/store/navigation";
import { ITEM_TYPES } from "@/features/item-editor/itemFlags";
import { ItemUsageModal } from "@/features/shared/ItemUsageModal";

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

const PAGE_SIZE_OPTIONS = [20, 50, 100, 200] as const;
const TYPE_LABEL: Record<number, string> = Object.fromEntries(ITEM_TYPES.map((t) => [t.value, t.label]));

// Eigener durchblätterbarer Überblick über die komplette `item_proto`-Tabelle
// (anders als der generische DB Explorer: mit Typ-Label statt Rohwert und
// gezieltem Sprung zum Bearbeiten) und anders als der in andere Editoren
// eingebettete EntityBrowser (nur vnum+Name, kein eigener Navigationspunkt).
export function ItemProtoExplorer() {
  const setSection = useNavigationStore((s) => s.setSection);

  const [query, setQuery] = useState("");
  const [typeFilter, setTypeFilter] = useState<string>("");
  const [page, setPage] = useState(0);
  const [pageSize, setPageSize] = useState<number>(50);
  const [refreshKey, setRefreshKey] = useState(0);

  const [data, setData] = useState<ItemProtoPage | null>(null);
  const [loading, setLoading] = useState(false);
  const [error, setError] = useState<string | null>(null);
  const [icons, setIcons] = useState<Record<number, string | null>>({});

  const [exporting, setExporting] = useState(false);
  const [exportError, setExportError] = useState<string | null>(null);
  const [usageVnum, setUsageVnum] = useState<number | null>(null);

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

  // Exportiert nicht nur die aktuell sichtbare Seite, sondern alle Treffer
  // der aktuellen Suche/des Typ-Filters - dieselbe eine Serveranfrage mit
  // `limit = total` deckt das ab, da das Backend dieselbe WHERE-Klausel wie
  // die normale Seitenanzeige nutzt.
  async function exportAllMatches() {
    if (!data) return;
    setExportError(null);
    setExporting(true);
    try {
      const full = await invoke<ItemProtoPage>("browse_item_proto", {
        query: query.trim() || null,
        typeFilter: typeFilter === "" ? null : Number(typeFilter),
        offset: 0,
        limit: data.total,
      });
      const rows = full.rows.map((r) => [
        r.vnum,
        r.name,
        TYPE_LABEL[r.item_type] ?? String(r.item_type),
        r.subtype,
      ]);
      await exportRowsAsCsv("item_proto_export.csv", ["VNUM", "Name", "Typ", "Subtyp"], rows);
    } catch (e) {
      setExportError(String(e));
    } finally {
      setExporting(false);
    }
  }

  const totalPages = data ? Math.max(1, Math.ceil(data.total / pageSize)) : 1;

  return (
    <div className="flex h-full flex-col gap-3">
      <div className="flex items-center gap-2">
        <Table2 className="size-6 text-muted-foreground" />
        <h1 className="text-2xl font-semibold">Item-Proto-Explorer</h1>
        <Button
          variant="ghost"
          size="icon-sm"
          title="Hilfe zu diesem Modul"
          onClick={() => openManual("item-proto-explorer")}
        >
          <HelpCircle className="size-4" />
        </Button>
      </div>
      <p className="text-sm text-muted-foreground">
        Durchblättert die komplette <code>item_proto</code>-Tabelle mit Typ-Label statt Rohwert -
        zum Bearbeiten eines Items geht es weiter in den Item Editor.
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
        <Button
          variant="outline"
          onClick={exportAllMatches}
          disabled={!data || data.total === 0 || exporting}
          title="Exportiert alle Treffer der aktuellen Suche/des Filters als CSV, nicht nur die sichtbare Seite"
        >
          <Download className="size-4" />
          {exporting ? "Exportiere…" : "Alle Treffer exportieren"}
        </Button>
      </div>

      {error && <p className="text-sm text-destructive">{error}</p>}
      {exportError && <p className="text-sm text-destructive">{exportError}</p>}

      <div className="min-h-0 flex-1 overflow-y-auto rounded-md border border-border">
        <table className="w-full text-sm">
          <thead className="sticky top-0 bg-card">
            <tr className="border-b border-border text-left text-xs text-muted-foreground">
              <th className="p-2 font-medium">Icon</th>
              <th className="p-2 font-medium">VNUM</th>
              <th className="p-2 font-medium">Name</th>
              <th className="p-2 font-medium">Typ</th>
              <th className="p-2 font-medium">Subtyp</th>
              <th className="p-2 font-medium"></th>
            </tr>
          </thead>
          <tbody>
            {data?.rows.map((r) => (
              <tr key={r.vnum} className="border-b border-border/50 hover:bg-muted">
                <td className="p-2">
                  {icons[r.vnum] ? (
                    <img src={icons[r.vnum]!} alt="" className="size-6 [image-rendering:pixelated]" />
                  ) : (
                    <div className="size-6 bg-muted/60" />
                  )}
                </td>
                <td className="p-2 text-muted-foreground">{r.vnum}</td>
                <td className="p-2">{r.name || <span className="text-muted-foreground">(kein Name)</span>}</td>
                <td className="p-2">{TYPE_LABEL[r.item_type] ?? r.item_type}</td>
                <td className="p-2 text-muted-foreground">{r.subtype}</td>
                <td className="p-2 text-right">
                  <Button variant="outline" size="sm" onClick={() => setUsageVnum(r.vnum)}>
                    <Link2 className="size-3.5" />
                    Verwendung prüfen
                  </Button>{" "}
                  <Button variant="outline" size="sm" onClick={() => setSection("item-editor")}>
                    Im Item Editor öffnen
                  </Button>
                </td>
              </tr>
            ))}
            {loading && (
              <tr>
                <td colSpan={6} className="p-3 text-center text-sm text-muted-foreground">
                  <Loader2 className="mx-auto size-4 animate-spin" />
                </td>
              </tr>
            )}
            {!loading && data?.rows.length === 0 && (
              <tr>
                <td colSpan={6} className="p-3 text-center text-sm text-muted-foreground">
                  Keine Treffer.
                </td>
              </tr>
            )}
          </tbody>
        </table>
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

      {usageVnum !== null && <ItemUsageModal vnum={usageVnum} onClose={() => setUsageVnum(null)} />}
    </div>
  );
}
