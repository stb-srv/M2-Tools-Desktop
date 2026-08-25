import { useEffect, useRef, useState } from "react";
import { invoke } from "@tauri-apps/api/core";
import { runAsyncAction } from "@/lib/asyncAction";
import { logActivity } from "@/lib/logActivity";
import { reportSectionDirty } from "@/store/navigation";
import { useSaveShortcut } from "@/lib/useSaveShortcut";
import { Button } from "@/components/ui/button";
import { AlertTriangle, Info, Plus, Trash2, RefreshCw, CheckCircle2 } from "lucide-react";
import { EntityBrowser, type EntityRow } from "@/features/shared/EntityBrowser";
import { formatRealDropChance } from "@/features/mob-drop-editor/dropChance";

interface DropGroupItem {
  item_ref: string;
  percent: number;
  count: number;
}

interface DropItemGroup {
  name: string;
  mob_vnum: number;
  items: DropGroupItem[];
}

interface TableRows {
  columns: string[];
  rows: (string | null)[][];
}

function emptyGroup(): DropItemGroup {
  return { name: "", mob_vnum: 0, items: [] };
}

async function resolveMobLabel(vnum: number): Promise<string> {
  try {
    const result = await invoke<TableRows>("search_table_rows", {
      database: "player",
      table: "mob_proto",
      column: "vnum",
      query: String(vnum),
    });
    const idx = result.columns.indexOf("locale_name");
    const row = result.rows.find((r) => Number(r[result.columns.indexOf("vnum")]) === vnum);
    return row && idx >= 0 && row[idx] ? `${row[idx]} (${vnum})` : `${vnum} (nicht gefunden!)`;
  } catch {
    return `${vnum} (nicht gefunden!)`;
  }
}

export function DropItemGroupEditor() {
  const [groups, setGroups] = useState<DropItemGroup[] | null>(null);
  const [itemLabels, setItemLabels] = useState<Record<string, string>>({});
  const [mobLabels, setMobLabels] = useState<Record<number, string>>({});
  const [loading, setLoading] = useState(false);
  const [loadError, setLoadError] = useState<string | null>(null);
  const [saving, setSaving] = useState(false);
  const [saveError, setSaveError] = useState<string | null>(null);
  const [saveOk, setSaveOk] = useState(false);
  const [confirmOpen, setConfirmOpen] = useState(false);
  const [mobPickerFor, setMobPickerFor] = useState<number | null>(null);
  const [itemPickerFor, setItemPickerFor] = useState<{ group: number; item: number } | null>(null);

  const loadedSnapshot = useRef("");
  const dirty = groups !== null && JSON.stringify(groups) !== loadedSnapshot.current;
  useEffect(() => {
    reportSectionDirty("drop-generator", dirty);
    return () => reportSectionDirty("drop-generator", false);
  }, [dirty]);

  async function resolveAllLabels(list: DropItemGroup[]) {
    const mobEntries = await Promise.all(
      Array.from(new Set(list.map((g) => g.mob_vnum))).map(async (v) => [v, await resolveMobLabel(v)] as const),
    );
    setMobLabels(Object.fromEntries(mobEntries));

    const itemVnums = new Set<string>();
    list.forEach((g) => g.items.forEach((it) => it.item_ref.trim() && itemVnums.add(it.item_ref.trim())));
    const itemEntries = await Promise.all(
      Array.from(itemVnums).map(async (v) => {
        try {
          const proto = await invoke<{ locale_name: string } | null>("get_item_proto", { vnum: Number(v) });
          return [v, proto ? `${proto.locale_name} (${v})` : `${v} (nicht gefunden!)`] as const;
        } catch {
          return [v, `${v} (nicht gefunden!)`] as const;
        }
      }),
    );
    setItemLabels(Object.fromEntries(itemEntries));
  }

  async function load() {
    await runAsyncAction(() => invoke<DropItemGroup[]>("read_drop_item_group_file"), {
      onStart: () => {
        setLoading(true);
        setLoadError(null);
        setSaveOk(false);
      },
      onSuccess: async (list) => {
        setGroups(list);
        loadedSnapshot.current = JSON.stringify(list);
        await resolveAllLabels(list);
      },
      onError: setLoadError,
      onFinally: () => setLoading(false),
    });
  }

  function updateGroup(idx: number, patch: Partial<DropItemGroup>) {
    if (!groups) return;
    setGroups(groups.map((g, i) => (i === idx ? { ...g, ...patch } : g)));
    setSaveOk(false);
  }

  function addGroup() {
    setGroups((prev) => [...(prev ?? []), emptyGroup()]);
  }

  function removeGroup(idx: number) {
    setGroups((prev) => (prev ? prev.filter((_, i) => i !== idx) : prev));
  }

  function addItem(groupIdx: number) {
    if (!groups) return;
    const next = groups.map((g, i) =>
      i === groupIdx ? { ...g, items: [...g.items, { item_ref: "", percent: 1, count: 1 }] } : g,
    );
    setGroups(next);
  }

  function updateItem(groupIdx: number, itemIdx: number, patch: Partial<DropGroupItem>) {
    if (!groups) return;
    const next = groups.map((g, i) =>
      i !== groupIdx ? g : { ...g, items: g.items.map((it, j) => (j === itemIdx ? { ...it, ...patch } : it)) },
    );
    setGroups(next);
    setSaveOk(false);
  }

  function removeItem(groupIdx: number, itemIdx: number) {
    if (!groups) return;
    setGroups(groups.map((g, i) => (i !== groupIdx ? g : { ...g, items: g.items.filter((_, j) => j !== itemIdx) })));
  }

  const duplicateMobVnums = new Set(
    groups
      ? groups
          .map((g) => g.mob_vnum)
          .filter((v, i, arr) => v > 0 && arr.indexOf(v) !== i)
      : [],
  );

  const canSave =
    groups !== null &&
    groups.length > 0 &&
    groups.every((g) => g.name.trim() !== "" && g.mob_vnum > 0 && g.items.length > 0 && g.items.every((it) => it.item_ref.trim() !== "")) &&
    duplicateMobVnums.size === 0;

  async function save() {
    if (!groups) return;
    setConfirmOpen(false);
    await runAsyncAction(() => invoke<string | null>("write_drop_item_group_file", { groups }), {
      onStart: () => {
        setSaving(true);
        setSaveError(null);
      },
      onSuccess: () => {
        setSaveOk(true);
        loadedSnapshot.current = JSON.stringify(groups);
        logActivity("drop-generator", "save", `drop_item_group.txt gespeichert (${groups.length} Gruppe(n))`, "drop_item_group.txt");
      },
      onError: setSaveError,
      onFinally: () => setSaving(false),
    });
  }

  useSaveShortcut("drop-generator", dirty && canSave && !saving, () => setConfirmOpen(true));

  return (
    <div className="max-w-4xl space-y-4 pb-10">
      <div className="rounded-lg border border-border p-3 text-sm text-muted-foreground">
        <p className="mb-1 font-medium text-foreground">drop_item_group.txt — Zufalls-Gruppen pro Mob</p>
        <p>
          Jede Gruppe gehört zu genau einem Mob (über die Mob-VNUM) und enthält beliebig viele Items mit je einer
          eigenen Chance. Beispiel: Gruppe für Mob 101 mit "Item A: 0,5%" und "Item B: 0,3%" - bei einem Kill wird
          <strong> jeder Eintrag einzeln gewürfelt</strong>: es können also Item A und B gleichzeitig droppen, nur
          eines der beiden, oder auch keins.
        </p>
      </div>

      <div className="flex items-start gap-2 rounded-lg border border-border bg-muted/30 p-3 text-sm text-muted-foreground">
        <Info className="mt-0.5 size-4 shrink-0" />
        <span>
          Unterschied zum "Item-Gruppen"-Tab (special_item_group.txt, Kisten-Loot): dort wird{" "}
          <strong>ein</strong> Treffer aus dem Pool gezogen. Hier gibt es keine Pool-Ziehung - jeder Eintrag ist ein
          eigener, unabhängiger Wurf.
        </span>
      </div>

      <div className="flex items-start gap-2 rounded-lg border border-amber-500/40 bg-amber-500/10 p-3 text-sm text-amber-700 dark:text-amber-400">
        <AlertTriangle className="mt-0.5 size-4 shrink-0" />
        <span>
          Wirkt erst nach einem kompletten Server-Neustart (kein <code>/reload</code> dafür). Ein nicht auflösbares
          Item lässt den <strong>kompletten Server beim nächsten Start nicht mehr hochfahren</strong>.
        </span>
      </div>

      <div className="flex gap-2">
        <Button variant="outline" onClick={load} disabled={loading}>
          <RefreshCw className={`size-4 ${loading ? "animate-spin" : ""}`} />
          Laden
        </Button>
        {groups && (
          <Button variant="outline" size="sm" onClick={addGroup}>
            <Plus className="size-3.5" />
            Gruppe hinzufügen
          </Button>
        )}
      </div>

      {loadError && <p className="text-sm text-destructive">{loadError}</p>}

      {groups && (
        <div className="space-y-3">
          {groups.map((group, groupIdx) => {
            const isDuplicateMob = group.mob_vnum > 0 && duplicateMobVnums.has(group.mob_vnum);
            return (
              <div key={groupIdx} className="space-y-2 rounded-lg border border-border p-3">
                <div className="flex flex-wrap items-center gap-2">
                  <input
                    value={group.name}
                    onChange={(e) => updateGroup(groupIdx, { name: e.target.value })}
                    placeholder="Gruppenname"
                    className="w-48 rounded-md border border-border bg-background px-2 py-1 text-sm"
                  />
                  <button
                    type="button"
                    onClick={() => setMobPickerFor(groupIdx)}
                    className="rounded-md border border-border bg-background px-2 py-1 text-left text-xs hover:bg-muted"
                  >
                    {group.mob_vnum > 0 ? (mobLabels[group.mob_vnum] ?? group.mob_vnum) : "Mob wählen…"}
                  </button>
                  <div className="ml-auto">
                    <Button variant="ghost" size="icon-sm" onClick={() => removeGroup(groupIdx)}>
                      <Trash2 className="size-3.5" />
                    </Button>
                  </div>
                </div>
                {isDuplicateMob && (
                  <p className="text-xs text-destructive">
                    Diese Mob-VNUM wird bereits von einer anderen Gruppe benutzt - der Server würde beide beim
                    nächsten Laden automatisch zu einer Gruppe zusammenführen. Bitte auf verschiedene Mobs
                    aufteilen oder die Items in einer Gruppe zusammenfassen.
                  </p>
                )}

                <table className="w-full text-sm">
                  <thead>
                    <tr className="text-xs text-muted-foreground">
                      <th className="px-1 py-1 text-left font-medium">Item</th>
                      <th className="px-1 py-1 text-left font-medium">Prozent (Datei-Wert)</th>
                      <th className="px-1 py-1 text-left font-medium">≈ real</th>
                      <th className="px-1 py-1 text-left font-medium">Anzahl</th>
                      <th />
                    </tr>
                  </thead>
                  <tbody>
                    {group.items.map((item, itemIdx) => (
                      <tr key={itemIdx} className="border-t border-border">
                        <td className="px-1 py-1">
                          <button
                            type="button"
                            onClick={() => setItemPickerFor({ group: groupIdx, item: itemIdx })}
                            className="rounded-md border border-border bg-background px-2 py-1 text-left text-xs hover:bg-muted"
                          >
                            {item.item_ref ? (itemLabels[item.item_ref] ?? item.item_ref) : "Item wählen…"}
                          </button>
                        </td>
                        <td className="px-1 py-1">
                          <input
                            type="number"
                            step="0.000001"
                            value={item.percent}
                            onChange={(e) => updateItem(groupIdx, itemIdx, { percent: Number(e.target.value) })}
                            className="w-24 rounded-md border border-border bg-background px-1.5 py-1 text-xs"
                          />
                        </td>
                        <td className="px-1 py-1 text-xs text-muted-foreground">{formatRealDropChance(item.percent)}</td>
                        <td className="px-1 py-1">
                          <input
                            type="number"
                            value={item.count}
                            onChange={(e) => updateItem(groupIdx, itemIdx, { count: Number(e.target.value) })}
                            className="w-16 rounded-md border border-border bg-background px-1.5 py-1 text-xs"
                          />
                        </td>
                        <td className="px-1 py-1 text-right">
                          <Button variant="ghost" size="icon-sm" onClick={() => removeItem(groupIdx, itemIdx)}>
                            <Trash2 className="size-3.5" />
                          </Button>
                        </td>
                      </tr>
                    ))}
                  </tbody>
                </table>
                <Button variant="outline" size="sm" onClick={() => addItem(groupIdx)}>
                  <Plus className="size-3.5" />
                  Item hinzufügen
                </Button>
              </div>
            );
          })}

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
            <p className="text-xs text-destructive">
              Jede Gruppe braucht Name, Mob, mindestens ein Item und darf keine bereits verwendete Mob-VNUM
              wiederholen.
            </p>
          )}
        </div>
      )}

      {mobPickerFor !== null && (
        <div className="fixed inset-0 z-50 flex items-center justify-center bg-black/50">
          <div className="max-h-[85vh] w-[32rem] space-y-2 overflow-y-auto rounded-lg border border-border bg-card p-4">
            <div className="flex items-center justify-between">
              <p className="text-sm font-medium">Mob wählen</p>
              <Button variant="ghost" size="sm" onClick={() => setMobPickerFor(null)}>
                Schließen
              </Button>
            </div>
            <EntityBrowser
              kind="mob"
              autoFocus
              onPick={(row: EntityRow) => {
                updateGroup(mobPickerFor, { mob_vnum: row.vnum });
                setMobLabels((prev) => ({ ...prev, [row.vnum]: `${row.name} (${row.vnum})` }));
                setMobPickerFor(null);
              }}
            />
          </div>
        </div>
      )}

      {itemPickerFor && (
        <div className="fixed inset-0 z-50 flex items-center justify-center bg-black/50">
          <div className="max-h-[85vh] w-[32rem] space-y-2 overflow-y-auto rounded-lg border border-border bg-card p-4">
            <div className="flex items-center justify-between">
              <p className="text-sm font-medium">Item wählen</p>
              <Button variant="ghost" size="sm" onClick={() => setItemPickerFor(null)}>
                Schließen
              </Button>
            </div>
            <EntityBrowser
              kind="item"
              autoFocus
              onPick={(row: EntityRow) => {
                const vnumStr = String(row.vnum);
                updateItem(itemPickerFor.group, itemPickerFor.item, { item_ref: vnumStr });
                setItemLabels((prev) => ({ ...prev, [vnumStr]: `${row.name} (${vnumStr})` }));
                setItemPickerFor(null);
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
                drop_item_group.txt wird komplett überschrieben (Backup wird vorher angelegt). Wirkt erst nach
                einem Server-Neustart.
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
