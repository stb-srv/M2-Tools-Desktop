import { useEffect, useState } from "react";
import { invoke } from "@tauri-apps/api/core";
import { runAsyncAction } from "@/lib/asyncAction";
import { Button } from "@/components/ui/button";
import { CheckCircle2 } from "lucide-react";
import type { ColumnInfo, TableRows } from "../shared";

const POSITION_FIELDS = ["map_index", "x", "y"];

// Fokussierter Dialog statt des vollen GenericRowEditor - deckt gezielt zwei
// Support-Aufgaben ab: Yang gutschreiben/abziehen (atomar serverseitig, s.
// db/account.rs::adjust_player_gold) und einen festhängenden Spieler per
// direktem Positions-Update entbuggen (x/y/map_index sind die real beim
// Login geladenen Spalten, siehe db/src/ClientManagerPlayer.cpp - wirkt nur,
// während der Spieler offline ist, siehe Hinweis oben auf der Seite).
export function PlayerToolsDialog({
  playerId,
  playerLabel,
  onClose,
}: {
  playerId: string;
  playerLabel: string;
  onClose: () => void;
}) {
  const [goldDelta, setGoldDelta] = useState("");
  const [goldBusy, setGoldBusy] = useState(false);
  const [goldError, setGoldError] = useState<string | null>(null);
  const [goldNewValue, setGoldNewValue] = useState<number | null>(null);

  const [pkColumn, setPkColumn] = useState<string | null>(null);
  const [position, setPosition] = useState<Record<string, string | null> | null>(null);
  const [positionOriginal, setPositionOriginal] = useState<Record<string, string | null> | null>(null);
  const [posLoadError, setPosLoadError] = useState<string | null>(null);
  const [posSaving, setPosSaving] = useState(false);
  const [posSaveError, setPosSaveError] = useState<string | null>(null);
  const [posSaveOk, setPosSaveOk] = useState(false);

  useEffect(() => {
    runAsyncAction(
      async () => {
        const cols = await invoke<ColumnInfo[]>("get_table_columns", { database: "player", table: "player" });
        const pk = cols.find((c) => c.is_primary_key);
        if (!pk) throw new Error("player.player hat keinen Primärschlüssel.");
        const row = await invoke<TableRows | null>("get_table_row", {
          database: "player",
          table: "player",
          pkColumn: pk.name,
          pkValue: playerId,
        });
        if (!row) throw new Error("Spieler nicht gefunden.");
        const values: Record<string, string | null> = {};
        POSITION_FIELDS.forEach((f) => {
          const idx = row.columns.indexOf(f);
          values[f] = idx >= 0 ? row.rows[0][idx] : null;
        });
        return { pkName: pk.name, values };
      },
      {
        onSuccess: ({ pkName, values }) => {
          setPkColumn(pkName);
          setPosition(values);
          setPositionOriginal(values);
        },
        onError: setPosLoadError,
      },
    );
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, [playerId]);

  async function applyGold() {
    if (!goldDelta.trim()) return;
    await runAsyncAction(
      () => invoke<number>("adjust_player_gold", { playerId: Number(playerId), delta: Number(goldDelta) }),
      {
        onStart: () => {
          setGoldBusy(true);
          setGoldError(null);
        },
        onSuccess: (v) => {
          setGoldNewValue(v);
          setGoldDelta("");
        },
        onError: setGoldError,
        onFinally: () => setGoldBusy(false),
      },
    );
  }

  const positionChanged =
    position && positionOriginal
      ? POSITION_FIELDS.filter((f) => position[f] !== positionOriginal[f])
      : [];

  async function savePosition() {
    if (!pkColumn || positionChanged.length === 0) return;
    await runAsyncAction(
      () =>
        invoke("update_table_row", {
          database: "player",
          table: "player",
          pkColumn,
          pkValue: playerId,
          changes: positionChanged.map((f) => [f, position![f]]),
        }),
      {
        onStart: () => {
          setPosSaving(true);
          setPosSaveError(null);
        },
        onSuccess: () => {
          setPositionOriginal(position);
          setPosSaveOk(true);
        },
        onError: setPosSaveError,
        onFinally: () => setPosSaving(false),
      },
    );
  }

  return (
    <div className="fixed inset-0 z-50 flex items-center justify-center bg-black/50">
      <div className="w-96 space-y-4 rounded-lg border border-border bg-card p-4">
        <div className="flex items-center justify-between">
          <p className="text-sm font-medium">Werkzeuge: {playerLabel || `#${playerId}`}</p>
          <Button variant="ghost" size="sm" onClick={onClose}>
            Schließen
          </Button>
        </div>

        <section className="space-y-2 rounded-md border border-border p-3">
          <p className="text-xs font-medium text-muted-foreground">Yang gutschreiben/abziehen</p>
          <div className="flex gap-2">
            <input
              type="number"
              placeholder="z.B. 100000 oder -50000"
              value={goldDelta}
              onChange={(e) => setGoldDelta(e.target.value)}
              className="flex-1 rounded-md border border-border bg-background px-2 py-1 text-sm"
            />
            <Button size="sm" disabled={!goldDelta.trim() || goldBusy} onClick={applyGold}>
              {goldBusy ? "…" : "Anwenden"}
            </Button>
          </div>
          {goldError && <p className="text-sm text-destructive">{goldError}</p>}
          {goldNewValue !== null && (
            <p className="flex items-center gap-1 text-sm text-green-600">
              <CheckCircle2 className="size-4" /> Neuer Yang-Betrag: {goldNewValue}
            </p>
          )}
        </section>

        <section className="space-y-2 rounded-md border border-border p-3">
          <p className="text-xs font-medium text-muted-foreground">
            Position setzen (entbuggen - wirkt nur, wenn der Spieler offline ist)
          </p>
          {posLoadError && <p className="text-sm text-destructive">{posLoadError}</p>}
          {!position && !posLoadError && <p className="text-sm text-muted-foreground">Lade…</p>}
          {position && (
            <>
              <div className="grid grid-cols-3 gap-2">
                {POSITION_FIELDS.map((f) => (
                  <label key={f} className="flex flex-col gap-0.5 text-xs">
                    <span className="font-mono text-muted-foreground">{f}</span>
                    <input
                      type="number"
                      value={position[f] ?? ""}
                      onChange={(e) => {
                        setPosition({ ...position, [f]: e.target.value });
                        setPosSaveOk(false);
                      }}
                      className="rounded-md border border-border bg-background px-2 py-1 text-sm"
                    />
                  </label>
                ))}
              </div>
              {posSaveError && <p className="text-sm text-destructive">{posSaveError}</p>}
              {posSaveOk && (
                <p className="flex items-center gap-1 text-sm text-green-600">
                  <CheckCircle2 className="size-4" /> Gespeichert.
                </p>
              )}
              <Button size="sm" disabled={positionChanged.length === 0 || posSaving} onClick={savePosition}>
                {posSaving ? "Speichere…" : "Position speichern"}
              </Button>
            </>
          )}
        </section>
      </div>
    </div>
  );
}
