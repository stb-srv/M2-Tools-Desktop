import { useEffect, useState } from "react";
import { invoke } from "@tauri-apps/api/core";
import { Button } from "@/components/ui/button";
import { Trash2 } from "lucide-react";

interface ImportBatch {
  id: number;
  module_name: string;
  item_type: number;
  created_at: string;
  vnums: number[];
  had_effects: boolean;
}

export function ImportHistory({ refreshKey }: { refreshKey: number }) {
  const [batches, setBatches] = useState<ImportBatch[]>([]);
  const [undoingId, setUndoingId] = useState<number | null>(null);
  const [confirmId, setConfirmId] = useState<number | null>(null);
  const [error, setError] = useState<string | null>(null);

  async function refresh() {
    try {
      setBatches(await invoke<ImportBatch[]>("list_import_batches"));
    } catch {
      // No history yet / not reachable - just show the empty state.
    }
  }

  useEffect(() => {
    refresh();
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, [refreshKey]);

  async function undo(id: number) {
    setConfirmId(null);
    setUndoingId(id);
    setError(null);
    try {
      await invoke("undo_import_batch", { id });
      await refresh();
    } catch (e) {
      setError(String(e));
    } finally {
      setUndoingId(null);
    }
  }

  const confirmBatch = batches.find((b) => b.id === confirmId);

  return (
    <section className="space-y-2 rounded-lg border border-border p-4">
      <h2 className="text-sm font-medium text-muted-foreground">Importierte Pakete (Verlauf)</h2>
      {batches.length === 0 ? (
        <p className="text-sm text-muted-foreground">Noch keine Importe in diesem Verlauf.</p>
      ) : (
        <div className="space-y-2">
          {batches.map((b) => (
            <div
              key={b.id}
              className="flex flex-wrap items-center justify-between gap-3 rounded-md border border-border p-2 text-sm"
            >
              <div>
                <p className="font-medium">
                  {b.module_name}{" "}
                  <span className="text-xs font-normal text-muted-foreground">
                    ({b.item_type === 1 ? "Waffen" : "Rüstung/gemischt"})
                  </span>
                </p>
                <p className="text-xs text-muted-foreground">
                  {new Date(b.created_at).toLocaleString()} · {b.vnums.length} Item(s): {b.vnums.join(", ")}
                </p>
              </div>
              <Button
                variant="destructive"
                size="sm"
                disabled={undoingId === b.id}
                onClick={() => setConfirmId(b.id)}
              >
                <Trash2 className="size-3.5" />
                {undoingId === b.id ? "Entferne…" : "Rückgängig machen"}
              </Button>
            </div>
          ))}
        </div>
      )}
      {error && <p className="whitespace-pre-wrap text-sm text-destructive">{error}</p>}

      {confirmBatch && (
        <div className="fixed inset-0 flex items-center justify-center bg-black/50">
          <div className="w-[26rem] space-y-3 rounded-lg border border-border bg-card p-4">
            <p className="text-sm font-medium">
              {confirmBatch.vnums.length} Item(s) aus „{confirmBatch.module_name}" endgültig entfernen?
            </p>
            <ul className="list-disc space-y-1 pl-5 text-sm text-muted-foreground">
              <li>Datenbankeinträge werden gelöscht</li>
              <li>Icons und <code>item_list.txt</code>-Einträge werden entfernt, <code>icon.epk</code> neu gepackt</li>
              <li><code>item_proto</code> wird neu erzeugt und im Client ersetzt</li>
              <li>Kopierte 3D-Modell-/Effekt-/msm-Dateien bleiben liegen (harmlos ohne DB-Eintrag)</li>
            </ul>
            <div className="flex justify-end gap-2">
              <Button variant="outline" onClick={() => setConfirmId(null)}>
                Abbrechen
              </Button>
              <Button variant="destructive" onClick={() => undo(confirmBatch.id)}>
                Endgültig entfernen
              </Button>
            </div>
          </div>
        </div>
      )}
    </section>
  );
}
