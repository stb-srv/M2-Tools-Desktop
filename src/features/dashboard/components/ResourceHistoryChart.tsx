import { useEffect, useState } from "react";
import { useTranslation } from "react-i18next";
import { invoke } from "@tauri-apps/api/core";

interface ResourceHistoryPoint {
  id: number;
  created_at: string;
  cpu_percent: number;
  ram_used_bytes: number | null;
  ram_total_bytes: number | null;
  disk_capacity_percent: number | null;
}

const POINTS_TO_FETCH = 200;
const REFRESH_MS = 60_000;
const CHART_WIDTH = 600;
const CHART_HEIGHT = 120;

/** Maps a series of 0-100 values onto an SVG polyline `points` string. */
function toPolyline(values: (number | null)[], width: number, height: number): string {
  const usable = values.filter((v): v is number => v !== null);
  if (usable.length === 0) return "";
  const step = values.length > 1 ? width / (values.length - 1) : 0;
  return values
    .map((v, i) => (v === null ? null : `${i * step},${height - (Math.min(100, Math.max(0, v)) / 100) * height}`))
    .filter((p): p is string => p !== null)
    .join(" ");
}

/**
 * Minimal hand-rolled SVG line chart - no charting library in this project
 * (checked package.json), and the project's convention is to hand-roll
 * simple UI rather than add a dependency for something this small.
 */
export function ResourceHistoryChart() {
  const { t } = useTranslation();
  const [points, setPoints] = useState<ResourceHistoryPoint[] | null>(null);
  const [error, setError] = useState<string | null>(null);

  useEffect(() => {
    let cancelled = false;
    async function load() {
      try {
        const result = await invoke<ResourceHistoryPoint[]>("get_resource_history", { limit: POINTS_TO_FETCH });
        if (!cancelled) {
          setPoints(result);
          setError(null);
        }
      } catch (e) {
        if (!cancelled) setError(String(e));
      }
    }
    load();
    const interval = setInterval(load, REFRESH_MS);
    return () => {
      cancelled = true;
      clearInterval(interval);
    };
  }, []);

  if (error) {
    return <p className="text-sm text-destructive">{error}</p>;
  }
  if (!points || points.length < 2) {
    return (
      <p className="text-sm text-muted-foreground">
        {t("resourceHistory.notEnoughData")}
      </p>
    );
  }

  const cpuValues = points.map((p) => p.cpu_percent);
  const ramValues = points.map((p) =>
    p.ram_used_bytes !== null && p.ram_total_bytes ? (p.ram_used_bytes / p.ram_total_bytes) * 100 : null
  );

  const latest = points[points.length - 1];
  const oldest = points[0];

  return (
    <div className="space-y-2">
      <svg
        viewBox={`0 0 ${CHART_WIDTH} ${CHART_HEIGHT}`}
        preserveAspectRatio="none"
        className="h-28 w-full rounded-lg border border-border bg-card"
      >
        {[25, 50, 75].map((y) => (
          <line
            key={y}
            x1={0}
            x2={CHART_WIDTH}
            y1={CHART_HEIGHT - (y / 100) * CHART_HEIGHT}
            y2={CHART_HEIGHT - (y / 100) * CHART_HEIGHT}
            className="stroke-border"
            strokeWidth={1}
          />
        ))}
        <polyline
          points={toPolyline(cpuValues, CHART_WIDTH, CHART_HEIGHT)}
          fill="none"
          className="stroke-primary"
          strokeWidth={2}
          vectorEffect="non-scaling-stroke"
        />
        <polyline
          points={toPolyline(ramValues, CHART_WIDTH, CHART_HEIGHT)}
          fill="none"
          className="stroke-amber-500"
          strokeWidth={2}
          vectorEffect="non-scaling-stroke"
        />
      </svg>
      <div className="flex flex-wrap items-center justify-between gap-2 text-xs text-muted-foreground">
        <div className="flex items-center gap-3">
          <span className="flex items-center gap-1">
            <span className="size-2 rounded-full bg-primary" />
            {t("resourceHistory.cpu")}: {latest.cpu_percent.toFixed(1)}%
          </span>
          <span className="flex items-center gap-1">
            <span className="size-2 rounded-full bg-amber-500" />
            {t("resourceHistory.ram")}:{" "}
            {latest.ram_used_bytes !== null && latest.ram_total_bytes
              ? `${((latest.ram_used_bytes / latest.ram_total_bytes) * 100).toFixed(1)}%`
              : "-"}
          </span>
        </div>
        <span>
          {t("resourceHistory.range", {
            from: new Date(oldest.created_at).toLocaleTimeString("de-DE"),
            to: new Date(latest.created_at).toLocaleTimeString("de-DE"),
          })}
        </span>
      </div>
    </div>
  );
}
