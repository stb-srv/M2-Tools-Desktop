import { useMemo, useState } from "react";
import type { MobDropGroup } from "../types";
import { simulateDrops } from "../dropChance";
import { Modal } from "./shared";

const PRESETS = [100, 1000, 10000];

// Uses the same per-item real-chance formula shown everywhere else in this
// editor (see dropChance.ts) - each kill is an independent trial, so this
// can't drift from what the "≈X% real" column already promises per drop.
export function SimulatorModal({ group, icons, onClose }: {
  group: MobDropGroup;
  icons: Record<number, string | null>;
  onClose: () => void;
}) {
  const [kills, setKills] = useState(1000);

  const results = useMemo(() => simulateDrops(group.items, kills), [group.items, kills]);

  return (
    <Modal onClose={onClose} widthClassName="w-[28rem]">
      <div className="space-y-3">
        <div>
          <p className="text-sm font-medium">Drop-Simulation: {group.name}</p>
          <p className="text-xs text-muted-foreground">
            Erwartete Ausbeute über eine gewählte Anzahl Kills, auf Basis der realen Drop-Chance
            (siehe „i"-Info oben). Jeder Kill wird als unabhängiger Versuch gerechnet.
          </p>
        </div>

        <div className="flex items-center gap-2">
          {PRESETS.map((preset) => (
            <button
              key={preset}
              onClick={() => setKills(preset)}
              className={`rounded-md border border-border px-2 py-1 text-xs ${
                kills === preset ? "bg-primary text-primary-foreground" : "hover:bg-muted"
              }`}
            >
              {preset.toLocaleString("de-DE")}
            </button>
          ))}
          <input
            type="number"
            min={0}
            value={kills}
            onChange={(e) => setKills(Math.max(0, Number(e.target.value) || 0))}
            className="w-28 rounded-md border border-border bg-background px-2 py-1 text-xs"
          />
          <span className="text-xs text-muted-foreground">Kills</span>
        </div>

        <div className="max-h-80 space-y-1 overflow-y-auto">
          {results.map((r) => (
            <div
              key={r.item_vnum}
              className="flex items-center gap-3 rounded-md border border-border p-2 text-sm"
            >
              <div className="flex size-8 shrink-0 items-center justify-center rounded-md border border-border bg-muted/40">
                {icons[r.item_vnum] ? (
                  <img
                    src={icons[r.item_vnum]!}
                    alt=""
                    className="max-h-full w-6 object-contain [image-rendering:pixelated]"
                  />
                ) : (
                  <span className="text-[10px] text-muted-foreground">{r.item_vnum}</span>
                )}
              </div>
              <span className="w-16 shrink-0 text-xs text-muted-foreground">#{r.item_vnum}</span>
              <div className="flex flex-1 flex-col text-xs">
                <span>
                  Mind. 1×: <strong>{r.chanceAtLeastOne.toFixed(r.chanceAtLeastOne < 1 ? 3 : 1)}%</strong>
                </span>
                <span className="text-muted-foreground">
                  Erwartete Anzahl: {r.expectedCount.toFixed(r.expectedCount < 1 ? 3 : 1)}
                </span>
              </div>
            </div>
          ))}
          {results.length === 0 && (
            <p className="p-2 text-sm text-muted-foreground">Dieser Mob hat noch keine Drops.</p>
          )}
        </div>
      </div>
    </Modal>
  );
}
