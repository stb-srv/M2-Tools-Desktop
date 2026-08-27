import { useEffect, useState } from "react";
import { invoke } from "@tauri-apps/api/core";
import { Coins, RefreshCw, HelpCircle } from "lucide-react";
import { Button } from "@/components/ui/button";
import { Skeleton } from "@/components/ui/skeleton";
import { runAsyncAction } from "@/lib/asyncAction";
import { openManual } from "@/lib/manual";

interface EconomyStats {
  player_count: number;
  total_gold: number;
  shop_item_count: number;
  avg_shop_price: number | null;
  min_shop_price: number | null;
  max_shop_price: number | null;
}

interface TopGoldHolder {
  player_id: number;
  name: string | null;
  gold: number;
}

const TOP_HOLDERS_LIMIT = 20;

function formatGold(value: number | null): string {
  if (value === null) return "–";
  return Math.round(value).toLocaleString("de-DE");
}

export function EconomyDashboard() {
  const [stats, setStats] = useState<EconomyStats | null>(null);
  const [holders, setHolders] = useState<TopGoldHolder[] | null>(null);
  const [loading, setLoading] = useState(false);
  const [error, setError] = useState<string | null>(null);

  async function load() {
    await runAsyncAction(
      async () => {
        const [s, h] = await Promise.all([
          invoke<EconomyStats>("get_economy_stats"),
          invoke<TopGoldHolder[]>("get_top_gold_holders", { limit: TOP_HOLDERS_LIMIT }),
        ]);
        return { s, h };
      },
      {
        onStart: () => {
          setLoading(true);
          setError(null);
        },
        onSuccess: ({ s, h }) => {
          setStats(s);
          setHolders(h);
        },
        onError: setError,
        onFinally: () => setLoading(false),
      },
    );
  }

  useEffect(() => {
    load();
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, []);

  return (
    <div className="max-w-3xl space-y-4">
      <div className="flex items-center gap-2">
        <Coins className="size-6 text-muted-foreground" />
        <h1 className="text-2xl font-semibold">Wirtschafts-Dashboard</h1>
        <Button variant="ghost" size="icon-sm" title="Hilfe zu diesem Modul" onClick={() => openManual("economy-dashboard")}>
          <HelpCircle className="size-4" />
        </Button>
      </div>
      <p className="text-sm text-muted-foreground">
        Momentaufnahme der Server-Wirtschaft: Goldmenge im Umlauf (Summe über{" "}
        <code>player.player.gold</code>), Preisspanne über alle tatsächlich in Shops gelisteten
        Items (<code>item_proto.gold</code>, der NPC-Verkaufspreis), und die größten
        Gold-Bestände. Reine Lesewerte, keine Änderungen.
      </p>

      <div className="flex items-center gap-2">
        <Button variant="outline" onClick={load} disabled={loading}>
          <RefreshCw className={`size-4 ${loading ? "animate-spin" : ""}`} />
          Neu laden
        </Button>
      </div>

      {error && <p className="whitespace-pre-wrap text-sm text-destructive">{error}</p>}

      {!stats && !error && (
        <div className="grid grid-cols-2 gap-3 sm:grid-cols-3">
          {Array.from({ length: 6 }).map((_, i) => (
            <div key={i} className="space-y-1.5 rounded-md border border-border p-3">
              <Skeleton className="h-3 w-20" />
              <Skeleton className="h-5 w-16" />
            </div>
          ))}
        </div>
      )}

      {stats && (
        <div className="grid grid-cols-2 gap-3 sm:grid-cols-3">
          <div className="rounded-md border border-border p-3">
            <p className="text-xs text-muted-foreground">Spieler</p>
            <p className="text-lg font-semibold">{stats.player_count.toLocaleString("de-DE")}</p>
          </div>
          <div className="rounded-md border border-border p-3">
            <p className="text-xs text-muted-foreground">Gold im Umlauf</p>
            <p className="text-lg font-semibold">{formatGold(stats.total_gold)}</p>
          </div>
          <div className="rounded-md border border-border p-3">
            <p className="text-xs text-muted-foreground">Gelistete Shop-Items</p>
            <p className="text-lg font-semibold">{stats.shop_item_count.toLocaleString("de-DE")}</p>
          </div>
          <div className="rounded-md border border-border p-3">
            <p className="text-xs text-muted-foreground">Ø Shop-Preis</p>
            <p className="text-lg font-semibold">{formatGold(stats.avg_shop_price)}</p>
          </div>
          <div className="rounded-md border border-border p-3">
            <p className="text-xs text-muted-foreground">Min. Shop-Preis</p>
            <p className="text-lg font-semibold">{formatGold(stats.min_shop_price)}</p>
          </div>
          <div className="rounded-md border border-border p-3">
            <p className="text-xs text-muted-foreground">Max. Shop-Preis</p>
            <p className="text-lg font-semibold">{formatGold(stats.max_shop_price)}</p>
          </div>
        </div>
      )}

      <div>
        <h2 className="mb-2 text-sm font-semibold text-muted-foreground">
          Top {TOP_HOLDERS_LIMIT} Gold-Bestände
        </h2>
        <div className="space-y-1 rounded-md border border-border p-1">
          {(holders ?? []).map((h, i) => (
            <div
              key={h.player_id}
              className="flex items-center gap-2 rounded-md px-2 py-1.5 text-sm hover:bg-muted"
            >
              <span className="w-6 shrink-0 text-right text-xs text-muted-foreground">{i + 1}.</span>
              <span className="flex-1 truncate">{h.name ?? `Spieler #${h.player_id}`}</span>
              <span className="shrink-0 font-mono text-xs">{formatGold(h.gold)}</span>
            </div>
          ))}
          {holders && holders.length === 0 && (
            <p className="p-2 text-sm text-muted-foreground">Keine Spieler gefunden.</p>
          )}
          {!holders && !error && (
            <div className="space-y-1 p-1">
              <Skeleton className="h-8 w-full" />
              <Skeleton className="h-8 w-full" />
              <Skeleton className="h-8 w-full" />
            </div>
          )}
        </div>
      </div>
    </div>
  );
}
