import { useMemo, useRef, useState } from "react";
import { Button } from "@/components/ui/button";
import { RegenLineRow, newSpawn, type RegenLine } from "./RegenEditor";
import { Plus, RefreshCw, FolderCog, X } from "lucide-react";

export interface MapFolderInfo {
  category: string;
  folder_name: string;
}

export interface RegenMapImage {
  image_data_url: string;
  units_per_pixel: number;
  width_px: number;
  height_px: number;
}

interface ViewBox {
  x: number;
  y: number;
  w: number;
  h: number;
}

// Spawn-Typ ist die erste Zeichen-gesteuerte Kennung (siehe regen.rs:
// m/g/r/s, zweites Zeichen 'a' = aggressiv) - fürs Filtern/Einfärben reicht
// das erste Zeichen, statt jede exotische Kombination einzeln zu kennen.
const FILTER_KINDS: { key: string; label: string; color: string }[] = [
  { key: "m", label: "Einzelner Mob", color: "#38bdf8" },
  { key: "g", label: "Gruppe", color: "#a3e635" },
  { key: "r", label: "Gruppe von Gruppen", color: "#c084fc" },
  { key: "s", label: "Überall", color: "#facc15" },
];

function colorForSpawnType(spawnType: string): string {
  const kind = FILTER_KINDS.find((k) => spawnType.startsWith(k.key));
  return kind?.color ?? "#94a3b8";
}

type DragState =
  | { mode: "pan"; inverse: DOMMatrix; start: DOMPoint; startViewBox: ViewBox }
  | {
      mode: "move";
      inverse: DOMMatrix;
      start: DOMPoint;
      index: number;
      originalX: number;
      originalY: number;
    }
  | {
      mode: "resize";
      inverse: DOMMatrix;
      start: DOMPoint;
      index: number;
      originalRadiusX: number;
      originalRadiusY: number;
    };

export function RegenMapView({
  lines,
  mapImage,
  onUpdateLine,
  onAddLine,
  onRemoveLine,
  onReloadMap,
  onChangeMapFolder,
}: {
  lines: RegenLine[];
  mapImage: RegenMapImage;
  onUpdateLine: (index: number, patch: Partial<RegenLine>) => void;
  onAddLine: (line: RegenLine) => void;
  onRemoveLine: (index: number) => void;
  onReloadMap?: () => void;
  onChangeMapFolder?: () => void;
}) {
  const svgRef = useRef<SVGSVGElement | null>(null);
  const dragRef = useRef<DragState | null>(null);

  const [viewBox, setViewBox] = useState<ViewBox>({
    x: 0,
    y: 0,
    w: mapImage.width_px,
    h: mapImage.height_px,
  });
  const [selectedIndex, setSelectedIndex] = useState<number | null>(null);
  const [addMode, setAddMode] = useState(false);
  const [visibleKinds, setVisibleKinds] = useState<Set<string>>(
    new Set(FILTER_KINDS.map((k) => k.key)),
  );
  const [showExceptions, setShowExceptions] = useState(true);

  // Regen-Koordinaten sind bereits deckungsgleich mit dem Minimap-Pixelraum
  // (verifiziert an einer echten regen.txt mit 939 Spawns - eine anfängliche
  // Annahme von 100 Welt-Einheiten/Pixel hätte alle Marker in eine 8x12px
  // Ecke gequetscht). `units_per_pixel` bleibt trotzdem vom Backend gesetzt,
  // falls sich das für einzelne Karten je unterscheidet.
  const upp = mapImage.units_per_pixel;
  const toPx = (worldX: number, worldY: number) => ({
    x: worldX / upp,
    y: worldY / upp,
  });

  const rawCount = useMemo(() => lines.filter((l) => l.kind === "Raw").length, [lines]);

  function localPoint(clientX: number, clientY: number, matrix: DOMMatrix): DOMPoint {
    const svg = svgRef.current!;
    const pt = svg.createSVGPoint();
    pt.x = clientX;
    pt.y = clientY;
    return pt.matrixTransform(matrix);
  }

  function screenInverse(): DOMMatrix | null {
    const svg = svgRef.current;
    const ctm = svg?.getScreenCTM();
    return ctm ? ctm.inverse() : null;
  }

  function handleWheel(e: React.WheelEvent<SVGSVGElement>) {
    e.preventDefault();
    const inverse = screenInverse();
    if (!inverse) return;
    const pt = localPoint(e.clientX, e.clientY, inverse);
    const factor = e.deltaY > 0 ? 1.15 : 1 / 1.15;
    setViewBox((vb) => {
      const minW = mapImage.width_px / 20;
      const maxW = mapImage.width_px * 4;
      const newW = Math.min(maxW, Math.max(minW, vb.w * factor));
      const ratio = newW / vb.w;
      const newH = vb.h * ratio;
      return {
        x: pt.x - (pt.x - vb.x) * ratio,
        y: pt.y - (pt.y - vb.y) * ratio,
        w: newW,
        h: newH,
      };
    });
  }

  function handleBackgroundPointerDown(e: React.PointerEvent<SVGSVGElement>) {
    const inverse = screenInverse();
    if (!inverse) return;
    const pt = localPoint(e.clientX, e.clientY, inverse);

    if (addMode) {
      const worldX = Math.round(pt.x * upp);
      const worldY = Math.round(pt.y * upp);
      const spawn = { ...newSpawn(), center_x: worldX, center_y: worldY };
      onAddLine(spawn);
      setAddMode(false);
      setSelectedIndex(lines.length);
      return;
    }

    dragRef.current = { mode: "pan", inverse, start: pt, startViewBox: viewBox };
    svgRef.current?.setPointerCapture(e.pointerId);
  }

  function handleMarkerPointerDown(e: React.PointerEvent, index: number) {
    e.stopPropagation();
    if (addMode) return;
    setSelectedIndex(index);
    const inverse = screenInverse();
    if (!inverse) return;
    const line = lines[index];
    if (line.kind === "Raw") return;
    dragRef.current = {
      mode: "move",
      inverse,
      start: localPoint(e.clientX, e.clientY, inverse),
      index,
      originalX: line.center_x,
      originalY: line.center_y,
    };
    svgRef.current?.setPointerCapture(e.pointerId);
  }

  function handleResizePointerDown(e: React.PointerEvent, index: number) {
    e.stopPropagation();
    const inverse = screenInverse();
    if (!inverse) return;
    const line = lines[index];
    if (line.kind === "Raw") return;
    dragRef.current = {
      mode: "resize",
      inverse,
      start: localPoint(e.clientX, e.clientY, inverse),
      index,
      originalRadiusX: line.radius_x,
      originalRadiusY: line.radius_y,
    };
    svgRef.current?.setPointerCapture(e.pointerId);
  }

  function handlePointerMove(e: React.PointerEvent<SVGSVGElement>) {
    const drag = dragRef.current;
    if (!drag) return;
    const current = localPoint(e.clientX, e.clientY, drag.inverse);
    const dx = current.x - drag.start.x;
    const dy = current.y - drag.start.y;

    if (drag.mode === "pan") {
      setViewBox({
        x: drag.startViewBox.x - dx,
        y: drag.startViewBox.y - dy,
        w: drag.startViewBox.w,
        h: drag.startViewBox.h,
      });
    } else if (drag.mode === "move") {
      onUpdateLine(drag.index, {
        center_x: Math.round(drag.originalX + dx * upp),
        center_y: Math.round(drag.originalY + dy * upp),
      });
    } else if (drag.mode === "resize") {
      onUpdateLine(drag.index, {
        radius_x: Math.max(1, Math.round(drag.originalRadiusX + dx * upp)),
        radius_y: Math.max(1, Math.round(drag.originalRadiusY + dy * upp)),
      });
    }
  }

  function handlePointerUp() {
    dragRef.current = null;
  }

  function toggleKind(key: string) {
    setVisibleKinds((prev) => {
      const next = new Set(prev);
      if (next.has(key)) next.delete(key);
      else next.add(key);
      return next;
    });
  }

  const selectedLine = selectedIndex !== null ? lines[selectedIndex] : null;

  return (
    <div className="flex h-full min-h-0 gap-3">
      <div className="relative min-w-0 flex-1 overflow-hidden rounded-lg border border-border bg-black">
        <svg
          ref={svgRef}
          viewBox={`${viewBox.x} ${viewBox.y} ${viewBox.w} ${viewBox.h}`}
          className={`h-full w-full touch-none ${addMode ? "cursor-crosshair" : "cursor-grab"}`}
          onWheel={handleWheel}
          onPointerDown={handleBackgroundPointerDown}
          onPointerMove={handlePointerMove}
          onPointerUp={handlePointerUp}
        >
          <image
            href={mapImage.image_data_url}
            x={0}
            y={0}
            width={mapImage.width_px}
            height={mapImage.height_px}
            preserveAspectRatio="none"
          />

          {lines.map((line, index) => {
            if (line.kind === "Raw") return null;
            if (line.kind === "Exception" && !showExceptions) return null;
            if (line.kind === "Spawn" && !visibleKinds.has(line.spawn_type.charAt(0))) return null;

            const center = toPx(line.center_x, line.center_y);
            const rx = line.radius_x / upp;
            const ry = line.radius_y / upp;
            const selected = index === selectedIndex;
            const strokeWidth = Math.max(viewBox.w / 400, 1);

            return (
              <g key={index}>
                {line.kind === "Exception" ? (
                  <rect
                    x={center.x - rx}
                    y={center.y - ry}
                    width={rx * 2}
                    height={ry * 2}
                    fill="rgba(248,113,113,0.15)"
                    stroke="#f87171"
                    strokeWidth={strokeWidth}
                    strokeDasharray={`${strokeWidth * 3} ${strokeWidth * 2}`}
                    onPointerDown={(e) => handleMarkerPointerDown(e, index)}
                  />
                ) : (
                  <ellipse
                    cx={center.x}
                    cy={center.y}
                    rx={rx}
                    ry={ry}
                    fill={colorForSpawnType(line.spawn_type) + "33"}
                    stroke={colorForSpawnType(line.spawn_type)}
                    strokeWidth={selected ? strokeWidth * 2 : strokeWidth}
                    onPointerDown={(e) => handleMarkerPointerDown(e, index)}
                  />
                )}
                <circle
                  cx={center.x}
                  cy={center.y}
                  r={Math.max(viewBox.w / 250, 2)}
                  fill={line.kind === "Exception" ? "#f87171" : colorForSpawnType(line.spawn_type)}
                  onPointerDown={(e) => handleMarkerPointerDown(e, index)}
                />
                {selected && (
                  <circle
                    cx={center.x + rx}
                    cy={center.y + ry}
                    r={Math.max(viewBox.w / 200, 3)}
                    fill="white"
                    stroke="#111"
                    strokeWidth={strokeWidth}
                    className="cursor-nwse-resize"
                    onPointerDown={(e) => handleResizePointerDown(e, index)}
                  />
                )}
              </g>
            );
          })}
        </svg>

        <div className="absolute right-2 top-2 flex gap-1">
          {onChangeMapFolder && (
            <Button variant="secondary" size="icon-sm" onClick={onChangeMapFolder} title="Kartenordner ändern">
              <FolderCog className="size-3.5" />
            </Button>
          )}
          {onReloadMap && (
            <Button variant="secondary" size="icon-sm" onClick={onReloadMap} title="Karte neu laden">
              <RefreshCw className="size-3.5" />
            </Button>
          )}
        </div>

        <div className="absolute bottom-2 left-2 flex flex-wrap items-center gap-2 rounded-md bg-black/60 px-2 py-1 text-xs text-white">
          {FILTER_KINDS.map((k) => (
            <label key={k.key} className="flex items-center gap-1">
              <input
                type="checkbox"
                checked={visibleKinds.has(k.key)}
                onChange={() => toggleKind(k.key)}
              />
              <span style={{ color: k.color }}>{k.label}</span>
            </label>
          ))}
          <label className="flex items-center gap-1">
            <input
              type="checkbox"
              checked={showExceptions}
              onChange={() => setShowExceptions((v) => !v)}
            />
            <span className="text-red-400">Ausnahmezonen</span>
          </label>
          {rawCount > 0 && (
            <span className="text-muted-foreground">
              {rawCount} Kommentarzeile(n) nur in Tabellenansicht
            </span>
          )}
        </div>
      </div>

      <div className="flex w-72 shrink-0 flex-col gap-2">
        <Button
          variant={addMode ? "default" : "outline"}
          size="sm"
          onClick={() => setAddMode((v) => !v)}
        >
          <Plus className="size-3.5" />
          {addMode ? "Klicke auf die Karte…" : "Spawn hinzufügen"}
        </Button>

        {selectedLine ? (
          <div className="space-y-2">
            <div className="flex items-center justify-between">
              <span className="text-xs font-medium text-muted-foreground">Auswahl</span>
              <Button variant="ghost" size="icon-sm" onClick={() => setSelectedIndex(null)}>
                <X className="size-3.5" />
              </Button>
            </div>
            <RegenLineRow
              line={selectedLine}
              onChange={(patch) => onUpdateLine(selectedIndex!, patch)}
              onRemove={() => {
                onRemoveLine(selectedIndex!);
                setSelectedIndex(null);
              }}
            />
          </div>
        ) : (
          <p className="text-sm text-muted-foreground">
            Klicke auf einen Marker, um ihn zu bearbeiten. Ziehen verschiebt, der Griff unten
            rechts am Marker skaliert den Radius. Mausrad zoomt, Ziehen auf leerer Fläche
            verschiebt den Kartenausschnitt.
          </p>
        )}
      </div>
    </div>
  );
}
