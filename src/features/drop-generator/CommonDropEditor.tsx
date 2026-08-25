import { useEffect, useRef, useState } from "react";
import { invoke } from "@tauri-apps/api/core";
import { runAsyncAction } from "@/lib/asyncAction";
import { logActivity } from "@/lib/logActivity";
import { reportSectionDirty } from "@/store/navigation";
import { useSaveShortcut } from "@/lib/useSaveShortcut";
import { Button } from "@/components/ui/button";
import { AlertTriangle, Info, Plus, Trash2, RefreshCw, CheckCircle2, X } from "lucide-react";
import { EntityBrowser, type EntityRow } from "@/features/shared/EntityBrowser";
import { formatRealDropChance } from "@/features/mob-drop-editor/dropChance";
import { MAX_DROP_PERCENT } from "@/features/mob-drop-editor/components/shared";

interface MobDropItem {
  item_vnum: number;
  count: number;
  percent: number;
}

interface MobDropGroup {
  name: string;
  mob_vnum: number;
  drop_type: string;
  items: MobDropItem[];
}

function emptyGroup(): MobDropGroup {
  return { name: "", mob_vnum: 0, drop_type: "drop", items: [] };
}

// Live gegen den echten Dev-Server verifiziert (2026-08-18): common_drop_item.txt
// benutzt auf diesem Server tatsächlich dieselbe Group/Mob/Type-Grammatik wie
// mob_drop_item.txt (nicht das im generischen Server-Quellcode gefundene
// Rang-Level-Bracket-Format) - siehe Handbuch für Details. Dieser Editor ist
// deshalb bewusst als schlankere Variante des Mob-Drop-Editors gebaut
// (gleiche Datenstruktur, weniger Komfortfunktionen wie Bulk-Edit/CSV-Export).
export function CommonDropEditor() {
  const [groups, setGroups] = useState<MobDropGroup[] | null>(null);
  const [labels, setLabels] = useState<Record<number, string>>({});
  const [loading, setLoading] = useState(false);
  const [loadError, setLoadError] = useState<string | null>(null);
  const [selectedIndex, setSelectedIndex] = useState<number | null>(null);
  const [saving, setSaving] = useState(false);
  const [saveError, setSaveError] = useState<string | null>(null);
  const [saveOk, setSaveOk] = useState(false);
  const [confirmOpen, setConfirmOpen] = useState(false);
  const [deleteConfirm, setDeleteConfirm] = useState<number | null>(null);
  const [itemPicker, setItemPicker] = useState(false);
  const [creating, setCreating] = useState(false);
  const [newName, setNewName] = useState("");
  const [newMobVnum, setNewMobVnum] = useState("");

  const loadedSnapshot = useRef("");
  const dirty = groups !== null && JSON.stringify(groups) !== loadedSnapshot.current;
  useEffect(() => {
    reportSectionDirty("drop-generator", dirty);
    return () => reportSectionDirty("drop-generator", false);
  }, [dirty]);

  async function resolveLabels(list: MobDropGroup[]) {
    const vnums = new Set<number>();
    list.forEach((g) => g.items.forEach((it) => vnums.add(it.item_vnum)));
    const entries = await Promise.all(
      Array.from(vnums).map(async (vnum) => {
        try {
          const proto = await invoke<{ locale_name: string } | null>("get_item_proto", { vnum });
          return [vnum, proto ? proto.locale_name : `#${vnum} (nicht gefunden!)`] as const;
        } catch {
          return [vnum, `#${vnum} (nicht gefunden!)`] as const;
        }
      }),
    );
    setLabels(Object.fromEntries(entries));
  }

  async function load() {
    await runAsyncAction(() => invoke<MobDropGroup[]>("read_common_drop_file"), {
      onStart: () => {
        setLoading(true);
        setLoadError(null);
        setSaveOk(false);
        setSelectedIndex(null);
      },
      onSuccess: async (list) => {
        setGroups(list);
        loadedSnapshot.current = JSON.stringify(list);
        await resolveLabels(list);
      },
      onError: setLoadError,
      onFinally: () => setLoading(false),
    });
  }

  function updateGroup(idx: number, patch: Partial<MobDropGroup>) {
    if (!groups) return;
    setGroups(groups.map((g, i) => (i === idx ? { ...g, ...patch } : g)));
    setSaveOk(false);
  }

  function updateItem(groupIdx: number, itemIdx: number, patch: Partial<MobDropItem>) {
    if (!groups) return;
    setGroups(
      groups.map((g, gi) =>
        gi !== groupIdx ? g : { ...g, items: g.items.map((it, ii) => (ii === itemIdx ? { ...it, ...patch } : it)) },
      ),
    );
    setSaveOk(false);
  }

  function removeItem(groupIdx: number, itemIdx: number) {
    if (!groups) return;
    setGroups(groups.map((g, gi) => (gi !== groupIdx ? g : { ...g, items: g.items.filter((_, ii) => ii !== itemIdx) })));
  }

  function addItem(row: EntityRow) {
    if (selectedIndex === null || !groups) return;
    setGroups(
      groups.map((g, gi) =>
        gi !== selectedIndex ? g : { ...g, items: [...g.items, { item_vnum: row.vnum, count: 1, percent: 0 }] },
      ),
    );
    setLabels((prev) => ({ ...prev, [row.vnum]: row.name }));
    setItemPicker(false);
  }

  function submitNewGroup() {
    const mobVnum = Number(newMobVnum);
    if (!newName.trim() || !Number.isFinite(mobVnum) || mobVnum <= 0) return;
    setGroups((prev) => [...(prev ?? []), { ...emptyGroup(), name: newName.trim(), mob_vnum: mobVnum }]);
    setSelectedIndex(groups ? groups.length : 0);
    setCreating(false);
    setNewName("");
    setNewMobVnum("");
  }

  function confirmDeleteGroup() {
    if (deleteConfirm === null || !groups) return;
    setGroups(groups.filter((_, i) => i !== deleteConfirm));
    if (selectedIndex === deleteConfirm) setSelectedIndex(null);
    else if (selectedIndex !== null && selectedIndex > deleteConfirm) setSelectedIndex(selectedIndex - 1);
    setDeleteConfirm(null);
  }

  function clampPercent(value: number) {
    return Math.max(0, Math.min(MAX_DROP_PERCENT, Math.round(value * 10000) / 10000));
  }

  async function save() {
    if (!groups) return;
    setConfirmOpen(false);
    await runAsyncAction(() => invoke<string | null>("write_common_drop_file", { groups }), {
      onStart: () => {
        setSaving(true);
        setSaveError(null);
      },
      onSuccess: () => {
        setSaveOk(true);
        loadedSnapshot.current = JSON.stringify(groups);
        logActivity("drop-generator", "save", `common_drop_item.txt gespeichert (${groups.length} Gruppe(n))`, "common_drop_item.txt");
      },
      onError: setSaveError,
      onFinally: () => setSaving(false),
    });
  }

  useSaveShortcut("drop-generator", dirty && !saving, () => setConfirmOpen(true));

  const selected = selectedIndex !== null ? (groups?.[selectedIndex] ?? null) : null;

  return (
    <div className="max-w-5xl space-y-4 pb-10">
      <div className="rounded-lg border border-border p-3 text-sm text-muted-foreground">
        <p className="mb-1 font-medium text-foreground">common_drop_item.txt — zusätzliche Mob-Drops</p>
        <p>
          Live gegen diesen Server geprüft: die Datei benutzt hier dasselbe Format wie mob_drop_item.txt
          (<code>Group / Mob / Type / Item-Zeilen</code>), pro Mob-VNUM - nicht das generische Rang-Level-Format,
          das eine erste Version dieses Editors angenommen hatte. Beispiel, das bereits auf dem Server liegt:
          Gruppe "MetinStein1" für Mob 8001 mit Item 19, 100% Chance. In der Praxis eine zweite, separat gepflegte
          Drop-Tabelle - z.B. für Mobs, die nicht in mob_drop_item.txt gepflegt werden sollen.
        </p>
      </div>

      <div className="flex items-start gap-2 rounded-lg border border-amber-500/40 bg-amber-500/10 p-3 text-sm text-amber-700 dark:text-amber-400">
        <AlertTriangle className="mt-0.5 size-4 shrink-0" />
        <span>
          Ob diese Datei wie mob_drop_item.txt live per GM-Befehl neu ladbar ist, wurde nicht separat geprüft -
          im Zweifel nach dem Speichern einen Server-Neustart einplanen.
        </span>
      </div>

      <div className="flex items-start gap-2 rounded-lg border border-border bg-muted/30 p-3 text-sm text-muted-foreground">
        <Info className="mt-0.5 size-4 shrink-0" />
        <span>Prozentwerte vermutlich nach derselben Formel wie mob_drop_item.txt (≈ Datei-Wert ÷ 4 reale Chance) - nicht separat verifiziert, da dieselbe Datenstruktur denselben Lade-/Auswerte-Code nahelegt.</span>
      </div>

      <div className="flex gap-2">
        <Button variant="outline" onClick={load} disabled={loading}>
          <RefreshCw className={`size-4 ${loading ? "animate-spin" : ""}`} />
          Laden
        </Button>
        {groups && (
          <Button variant="outline" size="sm" onClick={() => setCreating(true)}>
            <Plus className="size-3.5" />
            Mob hinzufügen
          </Button>
        )}
      </div>

      {loadError && <p className="text-sm text-destructive">{loadError}</p>}

      {groups && (
        <div className="flex gap-4">
          <div className="w-64 shrink-0 space-y-1">
            {groups.map((g, idx) => (
              <div
                key={idx}
                onClick={() => setSelectedIndex(idx)}
                className={`cursor-pointer rounded-md border border-border p-2 text-sm hover:bg-muted ${
                  selectedIndex === idx ? "bg-muted" : ""
                }`}
              >
                <div className="font-medium">{g.name}</div>
                <div className="text-xs text-muted-foreground">
                  Mob: {g.mob_vnum} · {g.items.length} Drops
                </div>
              </div>
            ))}
            {groups.length === 0 && <p className="p-2 text-sm text-muted-foreground">Keine Einträge.</p>}
          </div>

          <div className="min-w-0 flex-1 space-y-3">
            {!selected && <p className="text-sm text-muted-foreground">Wähle links einen Eintrag aus.</p>}
            {selected && selectedIndex !== null && (
              <>
                <div className="flex items-start justify-between rounded-lg border border-border p-3">
                  <div className="flex flex-wrap items-end gap-3">
                    <label className="flex flex-col gap-1 text-xs text-muted-foreground">
                      Gruppenname
                      <input
                        value={selected.name}
                        onChange={(e) => updateGroup(selectedIndex, { name: e.target.value })}
                        className="w-48 rounded-md border border-border bg-background px-2 py-1 text-sm"
                      />
                    </label>
                    <label className="flex flex-col gap-1 text-xs text-muted-foreground">
                      Mob-VNUM
                      <input
                        type="number"
                        value={selected.mob_vnum}
                        onChange={(e) => updateGroup(selectedIndex, { mob_vnum: Number(e.target.value) || 0 })}
                        className="w-24 rounded-md border border-border bg-background px-2 py-1 text-sm"
                      />
                    </label>
                    <label className="flex flex-col gap-1 text-xs text-muted-foreground">
                      Type
                      <input
                        value={selected.drop_type}
                        onChange={(e) => updateGroup(selectedIndex, { drop_type: e.target.value })}
                        className="w-24 rounded-md border border-border bg-background px-2 py-1 text-sm"
                      />
                    </label>
                  </div>
                  <Button variant="destructive" size="sm" onClick={() => setDeleteConfirm(selectedIndex)}>
                    <Trash2 className="size-4" />
                    Löschen
                  </Button>
                </div>

                <div className="space-y-2 rounded-lg border border-border p-3">
                  <div className="flex items-center justify-between">
                    <h2 className="text-sm font-medium text-muted-foreground">Drops ({selected.items.length})</h2>
                    <Button size="sm" variant="outline" onClick={() => setItemPicker(true)}>
                      <Plus className="size-3.5" />
                      Item hinzufügen
                    </Button>
                  </div>
                  {selected.items.map((item, itemIdx) => (
                    <div key={itemIdx} className="flex items-center gap-3 rounded-md border border-border p-2">
                      <span className="w-40 shrink-0 truncate text-xs text-muted-foreground">
                        {labels[item.item_vnum] ?? `#${item.item_vnum}`}
                      </span>
                      <label className="flex flex-col gap-0.5 text-xs text-muted-foreground">
                        Anzahl
                        <input
                          type="number"
                          min={1}
                          value={item.count}
                          onChange={(e) => updateItem(selectedIndex, itemIdx, { count: Math.max(1, Number(e.target.value) || 1) })}
                          className="w-16 rounded-md border border-border bg-background px-2 py-1 text-sm"
                        />
                      </label>
                      <label className="flex flex-col gap-0.5 text-xs text-muted-foreground">
                        Prozent
                        <input
                          type="number"
                          min={0}
                          max={MAX_DROP_PERCENT}
                          step={0.01}
                          value={item.percent}
                          onChange={(e) => updateItem(selectedIndex, itemIdx, { percent: clampPercent(Number(e.target.value) || 0) })}
                          className="w-20 rounded-md border border-border bg-background px-2 py-1 text-sm"
                        />
                      </label>
                      <span className="text-xs text-muted-foreground">≈{formatRealDropChance(item.percent)}</span>
                      <Button variant="ghost" size="icon-sm" className="ml-auto" onClick={() => removeItem(selectedIndex, itemIdx)}>
                        <X className="size-4" />
                      </Button>
                    </div>
                  ))}
                  {selected.items.length === 0 && <p className="text-sm text-muted-foreground">Noch keine Drops.</p>}
                </div>
              </>
            )}
          </div>
        </div>
      )}

      {groups && (
        <>
          {saveError && <p className="text-sm text-destructive">{saveError}</p>}
          {saveOk && (
            <p className="flex items-center gap-1 text-sm text-green-600">
              <CheckCircle2 className="size-4" /> Gespeichert.
            </p>
          )}
          <Button disabled={saving} onClick={() => setConfirmOpen(true)}>
            {saving ? "Speichere…" : "Speichern"}
          </Button>
        </>
      )}

      {itemPicker && (
        <div className="fixed inset-0 z-50 flex items-center justify-center bg-black/50">
          <div className="max-h-[85vh] w-[32rem] space-y-2 overflow-y-auto rounded-lg border border-border bg-card p-4">
            <div className="flex items-center justify-between">
              <p className="text-sm font-medium">Item wählen</p>
              <Button variant="ghost" size="sm" onClick={() => setItemPicker(false)}>
                Schließen
              </Button>
            </div>
            <EntityBrowser kind="item" autoFocus onPick={addItem} />
          </div>
        </div>
      )}

      {creating && (
        <div className="fixed inset-0 z-50 flex items-center justify-center bg-black/50">
          <div className="w-80 space-y-3 rounded-lg border border-border bg-card p-4">
            <p className="text-sm font-medium">Neuen Eintrag anlegen</p>
            <label className="flex flex-col gap-1 text-xs text-muted-foreground">
              Gruppenname
              <input
                autoFocus
                value={newName}
                onChange={(e) => setNewName(e.target.value)}
                className="rounded-md border border-border bg-background px-2 py-1 text-sm"
              />
            </label>
            <label className="flex flex-col gap-1 text-xs text-muted-foreground">
              Mob-VNUM
              <input
                type="number"
                value={newMobVnum}
                onChange={(e) => setNewMobVnum(e.target.value)}
                className="rounded-md border border-border bg-background px-2 py-1 text-sm"
              />
            </label>
            <div className="flex justify-end gap-2">
              <Button variant="outline" onClick={() => setCreating(false)}>
                Abbrechen
              </Button>
              <Button disabled={!newName.trim() || !newMobVnum} onClick={submitNewGroup}>
                Anlegen
              </Button>
            </div>
          </div>
        </div>
      )}

      {deleteConfirm !== null && groups && (
        <div className="fixed inset-0 z-50 flex items-center justify-center bg-black/50">
          <div className="w-80 space-y-3 rounded-lg border border-border bg-card p-4">
            <p className="text-sm">
              Eintrag <strong>{groups[deleteConfirm]?.name}</strong> wirklich entfernen? Wird erst beim Speichern
              in die Datei übernommen.
            </p>
            <div className="flex justify-end gap-2">
              <Button variant="outline" onClick={() => setDeleteConfirm(null)}>
                Abbrechen
              </Button>
              <Button variant="destructive" onClick={confirmDeleteGroup}>
                Entfernen
              </Button>
            </div>
          </div>
        </div>
      )}

      {confirmOpen && (
        <div className="fixed inset-0 z-50 flex items-center justify-center bg-black/50">
          <div className="w-96 space-y-3 rounded-lg border border-border bg-card p-4">
            <p className="flex items-start gap-2 text-sm">
              <AlertTriangle className="mt-0.5 size-4 shrink-0 text-destructive" />
              <span>common_drop_item.txt wird komplett überschrieben (Backup wird vorher angelegt).</span>
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
