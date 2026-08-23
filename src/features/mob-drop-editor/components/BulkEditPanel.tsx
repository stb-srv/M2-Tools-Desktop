import { useState } from "react";
import { Button } from "@/components/ui/button";
import type { BulkMode, BulkScope } from "../types";
import { Field, MAX_DROP_PERCENT } from "./shared";

export interface BulkEditParams {
  scope: BulkScope;
  mode: BulkMode;
  delta: number;
  fixed: number;
  randomMin: number;
  randomMax: number;
  specificVnum: number;
  specificPercent: number;
}

export function BulkEditPanel({
  hasGroups,
  hasSelection,
  selectedName,
  realChance,
  onApply,
}: {
  hasGroups: boolean;
  hasSelection: boolean;
  selectedName?: string;
  realChance: (percent: number) => string;
  onApply: (params: BulkEditParams) => void;
}) {
  const [scope, setScope] = useState<BulkScope>("current");
  const [mode, setMode] = useState<BulkMode>("delta");
  const [delta, setDelta] = useState(0);
  const [fixed, setFixed] = useState(0);
  const [randomMin, setRandomMin] = useState(0);
  const [randomMax, setRandomMax] = useState(30);
  const [specificVnum, setSpecificVnum] = useState("");
  const [specificPercent, setSpecificPercent] = useState(0);
  const [confirming, setConfirming] = useState(false);

  function confirmApply() {
    onApply({
      scope,
      mode,
      delta,
      fixed,
      randomMin,
      randomMax,
      specificVnum: Number(specificVnum),
      specificPercent,
    });
    setConfirming(false);
  }

  return (
    <div className="space-y-3 rounded-lg border border-border p-3">
      <h2 className="text-sm font-medium text-muted-foreground">Prozentwerte in einem Rutsch ändern</h2>

      <div className="flex overflow-hidden rounded-md border border-border text-sm w-fit">
        <button
          onClick={() => setScope("global")}
          className={`px-3 py-1 ${scope === "global" ? "bg-primary text-primary-foreground" : ""}`}
        >
          Global (alle Mobs)
        </button>
        <button
          onClick={() => setScope("current")}
          disabled={!hasSelection}
          className={`px-3 py-1 disabled:opacity-40 ${scope === "current" ? "bg-primary text-primary-foreground" : ""}`}
        >
          Nur dieser Mob
        </button>
      </div>

      <div className="flex flex-wrap gap-2 text-sm">
        {(
          [
            ["delta", "Addieren/Subtrahieren"],
            ["fixed", "Fester Wert"],
            ["random", "Zufall in Bereich"],
            ["specific-item", "Bestimmtes Item überall"],
          ] as const
        ).map(([m, label]) => (
          <button
            key={m}
            onClick={() => setMode(m)}
            className={`rounded-md border border-border px-3 py-1 ${
              mode === m ? "bg-primary text-primary-foreground" : ""
            }`}
          >
            {label}
          </button>
        ))}
      </div>

      {mode === "delta" && (
        <Field label="Betrag (z.B. -5 oder 5)">
          <input
            type="number"
            value={delta}
            onChange={(e) => setDelta(Number(e.target.value) || 0)}
            className="w-24 rounded-md border border-border bg-background px-2 py-1 text-sm"
          />
        </Field>
      )}
      {mode === "fixed" && (
        <div className="flex items-end gap-2">
          <Field label="Neuer Wert für alle">
            <input
              type="number"
              min={0}
              max={MAX_DROP_PERCENT}
              value={fixed}
              onChange={(e) => setFixed(Number(e.target.value) || 0)}
              className="w-24 rounded-md border border-border bg-background px-2 py-1 text-sm"
            />
          </Field>
          <span className="pb-1.5 text-xs text-muted-foreground">≈{realChance(fixed)} real</span>
        </div>
      )}
      {mode === "random" && (
        <div className="flex items-end gap-3">
          <Field label="Min">
            <input
              type="number"
              min={0}
              max={MAX_DROP_PERCENT}
              value={randomMin}
              onChange={(e) => setRandomMin(Number(e.target.value) || 0)}
              className="w-20 rounded-md border border-border bg-background px-2 py-1 text-sm"
            />
          </Field>
          <Field label="Max">
            <input
              type="number"
              min={0}
              max={MAX_DROP_PERCENT}
              value={randomMax}
              onChange={(e) => setRandomMax(Number(e.target.value) || 0)}
              className="w-20 rounded-md border border-border bg-background px-2 py-1 text-sm"
            />
          </Field>
          <span className="pb-1.5 text-xs text-muted-foreground">
            ≈{realChance(randomMin)}–{realChance(randomMax)} real
          </span>
        </div>
      )}
      {mode === "specific-item" && (
        <div className="flex items-end gap-2">
          <Field label="Item-VNUM">
            <input
              type="number"
              value={specificVnum}
              onChange={(e) => setSpecificVnum(e.target.value)}
              className="w-24 rounded-md border border-border bg-background px-2 py-1 text-sm"
            />
          </Field>
          <Field label="Prozent">
            <input
              type="number"
              min={0}
              max={MAX_DROP_PERCENT}
              value={specificPercent}
              onChange={(e) => setSpecificPercent(Number(e.target.value) || 0)}
              className="w-20 rounded-md border border-border bg-background px-2 py-1 text-sm"
            />
          </Field>
          <span className="pb-1.5 text-xs text-muted-foreground">≈{realChance(specificPercent)} real</span>
        </div>
      )}

      <Button
        variant="outline"
        onClick={() => setConfirming(true)}
        disabled={!hasGroups || (scope === "current" && !hasSelection)}
      >
        Anwenden
      </Button>
      <p className="text-xs text-muted-foreground">
        Wirkt zunächst nur auf die geladenen Daten - erst "Speichern" oben schreibt sie in die Datei.
      </p>

      {confirming && (
        <div className="fixed inset-0 flex items-center justify-center bg-black/50">
          <div className="w-96 space-y-3 rounded-lg border border-border bg-card p-4">
            <p className="text-sm">
              Prozentwerte {scope === "global" ? "aller Mobs" : `von "${selectedName}"`} jetzt ändern? Das
              betrifft nur die geladenen Daten, bis du speicherst.
            </p>
            <div className="flex justify-end gap-2">
              <Button variant="outline" onClick={() => setConfirming(false)}>
                Abbrechen
              </Button>
              <Button onClick={confirmApply}>Anwenden</Button>
            </div>
          </div>
        </div>
      )}
    </div>
  );
}
