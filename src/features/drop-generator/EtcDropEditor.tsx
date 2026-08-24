import { useState } from "react";
import { invoke } from "@tauri-apps/api/core";
import { runAsyncAction } from "@/lib/asyncAction";
import { logActivity } from "@/lib/logActivity";
import { Button } from "@/components/ui/button";
import { AlertTriangle, Info, Plus, Trash2, RefreshCw, CheckCircle2 } from "lucide-react";
import { EntityBrowser, type EntityRow } from "@/features/shared/EntityBrowser";
import { formatRealDropChance } from "@/features/mob-drop-editor/dropChance";

interface EtcDropEntry {
  item_name: string;
  percent: number;
}

interface ItemBrief {
  vnum: number;
  locale_name: string;
}

export function EtcDropEditor() {
  const [entries, setEntries] = useState<EtcDropEntry[] | null>(null);
  const [labels, setLabels] = useState<Record<string, string | null>>({});
  const [loading, setLoading] = useState(false);
  const [loadError, setLoadError] = useState<string | null>(null);
  const [saving, setSaving] = useState(false);
  const [saveError, setSaveError] = useState<string | null>(null);
  const [saveOk, setSaveOk] = useState(false);
  const [confirmOpen, setConfirmOpen] = useState(false);
  const [pickerFor, setPickerFor] = useState<number | null>(null);

  async function resolveLabels(list: EtcDropEntry[]) {
    const entries = await Promise.all(
      list.map(async (e) => {
        try {
          const brief = await invoke<ItemBrief | null>("find_item_by_internal_name", { name: e.item_name });
          return [e.item_name, brief ? `${brief.locale_name} (${brief.vnum})` : null] as const;
        } catch {
          return [e.item_name, null] as const;
        }
      }),
    );
    setLabels(Object.fromEntries(entries));
  }

  async function load() {
    await runAsyncAction(() => invoke<EtcDropEntry[]>("read_etc_drop_file"), {
      onStart: () => {
        setLoading(true);
        setLoadError(null);
        setSaveOk(false);
      },
      onSuccess: async (list) => {
        setEntries(list);
        await resolveLabels(list);
      },
      onError: setLoadError,
      onFinally: () => setLoading(false),
    });
  }

  function updateEntry(idx: number, patch: Partial<EtcDropEntry>) {
    if (!entries) return;
    setEntries(entries.map((e, i) => (i === idx ? { ...e, ...patch } : e)));
    setSaveOk(false);
  }

  function removeEntry(idx: number) {
    setEntries((prev) => (prev ? prev.filter((_, i) => i !== idx) : prev));
  }

  const canSave = entries !== null && entries.every((e) => e.item_name.trim() !== "" && e.percent > 0);

  async function save() {
    if (!entries) return;
    setConfirmOpen(false);
    await runAsyncAction(() => invoke<string | null>("write_etc_drop_file", { entries }), {
      onStart: () => {
        setSaving(true);
        setSaveError(null);
      },
      onSuccess: () => {
        setSaveOk(true);
        logActivity("drop-generator", "save", `etc_drop_item.txt gespeichert (${entries.length} Eintrag/Einträge)`, "etc_drop_item.txt");
      },
      onError: setSaveError,
      onFinally: () => setSaving(false),
    });
  }

  return (
    <div className="max-w-3xl space-y-4 pb-10">
      <div className="rounded-lg border border-border p-3 text-sm text-muted-foreground">
        <p className="mb-1 font-medium text-foreground">etc_drop_item.txt — Item→Prozent-Tabelle, mob-unabhängig</p>
        <p>
          Diese Datei verknüpft <strong>keinen</strong> Mob mit einem Item - sie ist nur eine Nachschlagetabelle
          "Item X hat Chance Y". Welcher Mob welchen Eintrag benutzt, legt ein eigenes Feld im jeweiligen
          Mob-Proto fest (außerhalb dieser Datei, hier nicht editierbar). Beispiel: ein Eintrag "Kleiner Ring →
          0,5" bedeutet nur, dass IRGENDEIN Mob, der auf diesen Eintrag verweist, den Ring mit dieser Chance
          droppt.
        </p>
      </div>

      <div className="flex items-start gap-2 rounded-lg border border-amber-500/40 bg-amber-500/10 p-3 text-sm text-amber-700 dark:text-amber-400">
        <AlertTriangle className="mt-0.5 size-4 shrink-0" />
        <span>
          Wirkt erst nach einem kompletten Server-Neustart (kein <code>/reload</code> dafür). Ein nicht auflösbarer
          Eintrag lässt den <strong>kompletten Server beim nächsten Start nicht mehr hochfahren</strong>.
        </span>
      </div>

      <div className="flex items-start gap-2 rounded-lg border border-border bg-muted/30 p-3 text-sm text-muted-foreground">
        <Info className="mt-0.5 size-4 shrink-0" />
        <span>
          Diese Datei speichert Items über ihren internen Namen, nicht über die VNUM - dieser Editor löst das für
          dich automatisch auf, wenn du ein Item über die Suche wählst.
        </span>
      </div>

      <div className="flex gap-2">
        <Button variant="outline" onClick={load} disabled={loading}>
          <RefreshCw className={`size-4 ${loading ? "animate-spin" : ""}`} />
          Laden
        </Button>
        {entries && (
          <Button variant="outline" size="sm" onClick={() => setPickerFor(entries.length)}>
            <Plus className="size-3.5" />
            Eintrag hinzufügen
          </Button>
        )}
      </div>

      {loadError && <p className="text-sm text-destructive">{loadError}</p>}

      {entries && (
        <div className="overflow-x-auto rounded-md border border-border">
          <table className="w-full text-sm">
            <thead className="bg-muted/40">
              <tr>
                <th className="px-2 py-1 text-left font-medium">Item</th>
                <th className="px-2 py-1 text-left font-medium">Prozent (Datei-Wert)</th>
                <th className="px-2 py-1 text-left font-medium">≈ real</th>
                <th />
              </tr>
            </thead>
            <tbody>
              {entries.map((entry, idx) => {
                const label = labels[entry.item_name];
                return (
                  <tr key={idx} className="border-t border-border">
                    <td className="px-2 py-1">
                      <button
                        type="button"
                        onClick={() => setPickerFor(idx)}
                        className="rounded-md border border-border bg-background px-2 py-1 text-left text-xs hover:bg-muted"
                      >
                        {label === undefined
                          ? entry.item_name
                          : label === null
                            ? `${entry.item_name} (nicht in item_proto gefunden!)`
                            : label}
                      </button>
                    </td>
                    <td className="px-2 py-1">
                      <input
                        type="number"
                        step="0.000001"
                        value={entry.percent}
                        onChange={(e) => updateEntry(idx, { percent: Number(e.target.value) })}
                        className="w-24 rounded-md border border-border bg-background px-1.5 py-1 text-xs"
                      />
                    </td>
                    <td className="px-2 py-1 text-xs text-muted-foreground">{formatRealDropChance(entry.percent)}</td>
                    <td className="px-2 py-1 text-right">
                      <Button variant="ghost" size="icon-sm" onClick={() => removeEntry(idx)}>
                        <Trash2 className="size-3.5" />
                      </Button>
                    </td>
                  </tr>
                );
              })}
              {entries.length === 0 && (
                <tr>
                  <td colSpan={4} className="px-2 py-3 text-center text-muted-foreground">
                    Keine Einträge.
                  </td>
                </tr>
              )}
            </tbody>
          </table>
        </div>
      )}

      {entries && (
        <>
          {saveError && <p className="text-sm text-destructive">{saveError}</p>}
          {saveOk && (
            <p className="flex items-center gap-1 text-sm text-green-600">
              <CheckCircle2 className="size-4" /> Gespeichert.
            </p>
          )}
          <Button disabled={!canSave || saving} onClick={() => setConfirmOpen(true)}>
            {saving ? "Speichere…" : "Speichern"}
          </Button>
          {!canSave && (
            <p className="text-xs text-destructive">Jeder Eintrag braucht ein Item und einen Prozentwert &gt; 0.</p>
          )}
        </>
      )}

      {pickerFor !== null && (
        <div className="fixed inset-0 z-50 flex items-center justify-center bg-black/50">
          <div className="max-h-[85vh] w-[32rem] space-y-2 overflow-y-auto rounded-lg border border-border bg-card p-4">
            <div className="flex items-center justify-between">
              <p className="text-sm font-medium">Item wählen</p>
              <Button variant="ghost" size="sm" onClick={() => setPickerFor(null)}>
                Schließen
              </Button>
            </div>
            <EntityBrowser
              kind="item"
              autoFocus
              onPick={async (row: EntityRow) => {
                const idx = pickerFor;
                setPickerFor(null);
                await runAsyncAction(() => invoke<string>("get_item_internal_name", { vnum: row.vnum }), {
                  onSuccess: (internalName) => {
                    setEntries((prev) => {
                      const base = prev ?? [];
                      const next = [...base];
                      if (idx < next.length) {
                        next[idx] = { ...next[idx], item_name: internalName };
                      } else {
                        next.push({ item_name: internalName, percent: 1 });
                      }
                      return next;
                    });
                    setLabels((prev) => ({ ...prev, [internalName]: `${row.name} (${row.vnum})` }));
                  },
                  onError: setLoadError,
                });
              }}
            />
          </div>
        </div>
      )}

      {confirmOpen && (
        <div className="fixed inset-0 z-50 flex items-center justify-center bg-black/50">
          <div className="w-96 space-y-3 rounded-lg border border-border bg-card p-4">
            <p className="flex items-start gap-2 text-sm">
              <AlertTriangle className="mt-0.5 size-4 shrink-0 text-destructive" />
              <span>
                etc_drop_item.txt wird komplett überschrieben (Backup wird vorher angelegt). Wirkt erst nach einem
                Server-Neustart.
              </span>
            </p>
            <div className="flex justify-end gap-2">
              <Button variant="outline" onClick={() => setConfirmOpen(false)}>
                Abbrechen
              </Button>
              <Button onClick={save}>Speichern</Button>
            </div>
          </div>
        </div>
      )}
    </div>
  );
}
