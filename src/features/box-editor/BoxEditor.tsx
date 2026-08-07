import { useEffect, useState } from "react";
import { invoke } from "@tauri-apps/api/core";
import { runAsyncAction } from "@/lib/asyncAction";
import { Button } from "@/components/ui/button";
import { Search, Plus, Trash2, RefreshCw, CheckCircle2, AlertTriangle, Info, Package } from "lucide-react";
import { useNavigationStore } from "@/store/navigation";

const GIFTBOX_TYPE = 23;
const FLAG_STACKABLE = 1 << 2;
const ANTIFLAG_NOT_STACKABLE = 1 << 15;

interface ItemProtoFullLite {
  vnum: number;
  locale_name: string;
  name: string;
  type: number;
  flag: number;
  antiflag: number;
}

function boxItemIsReady(proto: ItemProtoFullLite): boolean {
  return proto.type === GIFTBOX_TYPE && (proto.flag & FLAG_STACKABLE) !== 0 && (proto.antiflag & ANTIFLAG_NOT_STACKABLE) === 0;
}

// Sets item_proto.type=GIFTBOX(23), sets the "Stapelbar" flag bit, and
// clears the "Nicht stapelbar" antiflag override (the two conflicting bits
// BoxVnumHint/BoxItemPicker both check) - then repacks the client's own
// item_proto file, mirroring exactly what the Item Editor's "proto" step
// does after any type/flag edit (db/item.rs::update_item_proto followed by
// regenerate_item_proto + deploy_item_proto). No server restart needed -
// item_proto is a normal DB-backed prototype table, unlike the boot-only
// special_item_group.txt/refine_proto text files.
async function setupBoxItem(vnum: number): Promise<void> {
  const full = await invoke<Record<string, unknown> | null>("get_item_proto", { vnum });
  if (!full) throw new Error(`Item #${vnum} wurde nicht gefunden.`);
  const patched = {
    ...full,
    type: GIFTBOX_TYPE,
    flag: ((full.flag as number) || 0) | FLAG_STACKABLE,
    antiflag: ((full.antiflag as number) || 0) & ~ANTIFLAG_NOT_STACKABLE,
  };
  await invoke("update_item_proto", { item: patched });
  const generated = await invoke<string>("regenerate_item_proto");
  await invoke("deploy_item_proto", { generatedProtoPath: generated });
}

// Resolves a "Kisten-Item-VNUM" input against the real item_proto row, so a
// typo'd or mismatched vnum is caught immediately instead of only showing up
// as a silent "du hast nichts erhalten" in-game later - GiveItemFromSpecialItemGroup
// (char_item.cpp) looks the group up purely by the box item's own vnum, with
// no cross-check, so a group whose Vnum doesn't match a real GIFTBOX item's
// vnum fails invisibly (logged server-side only, never shown to the player).
function BoxVnumHint({ vnum, onFixed }: { vnum: number; onFixed?: () => void }) {
  const [proto, setProto] = useState<ItemProtoFullLite | null | undefined>(undefined);
  const [fixing, setFixing] = useState(false);
  const [fixError, setFixError] = useState<string | null>(null);

  function reload() {
    if (!vnum) {
      setProto(undefined);
      return;
    }
    invoke<ItemProtoFullLite | null>("get_item_proto", { vnum })
      .then(setProto)
      .catch(() => setProto(null));
  }

  useEffect(() => {
    setFixError(null);
    reload();
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, [vnum]);

  async function fix() {
    setFixing(true);
    setFixError(null);
    try {
      await setupBoxItem(vnum);
      reload();
      onFixed?.();
    } catch (e) {
      setFixError(String(e));
    } finally {
      setFixing(false);
    }
  }

  if (!vnum || proto === undefined) return null;

  if (proto === null) {
    return (
      <p className="flex items-center gap-1 text-xs text-destructive">
        <AlertTriangle className="size-3.5 shrink-0" /> Kein Item mit dieser VNUM gefunden.
      </p>
    );
  }

  const displayName = proto.locale_name || proto.name;

  if (!boxItemIsReady(proto)) {
    return (
      <div className="space-y-1">
        <p className="flex items-center gap-1 text-xs text-destructive">
          <AlertTriangle className="size-3.5 shrink-0" />
          "{displayName}" ist noch nicht als Kisten-Item eingerichtet (Typ GIFTBOX + Stapelbar).
        </p>
        <Button size="sm" variant="outline" onClick={fix} disabled={fixing}>
          {fixing ? "Richte ein…" : "Jetzt automatisch einrichten"}
        </Button>
        {fixError && <p className="text-xs text-destructive">{fixError}</p>}
      </div>
    );
  }

  return (
    <p className="flex items-center gap-1 text-xs text-muted-foreground">
      <CheckCircle2 className="size-3.5 shrink-0 text-green-600" />
      "{displayName}" (GIFTBOX, stapelbar)
    </p>
  );
}

// Search-and-fix picker for the "Kisten-Item-VNUM" field: lets the user find
// a candidate item by name instead of already knowing its vnum, and applies
// setupBoxItem in one click - a plain (non-GIFTBOX) item needs an explicit
// confirmation first since changing its type/flag is not harmless if the
// wrong result gets picked (e.g. a real weapon reused for something else).
function BoxItemPicker({ onPick }: { onPick: (vnum: number) => void }) {
  const [picking, setPicking] = useState(false);
  const [query, setQuery] = useState("");
  const [results, setResults] = useState<ItemSearchResult[]>([]);
  const [searching, setSearching] = useState(false);
  const [statuses, setStatuses] = useState<Record<number, ItemProtoFullLite | null>>({});
  const [confirmVnum, setConfirmVnum] = useState<number | null>(null);
  const [busyVnum, setBusyVnum] = useState<number | null>(null);
  const [error, setError] = useState<string | null>(null);

  async function runSearch() {
    if (!query.trim()) return;
    setError(null);
    await runAsyncAction(() => invoke<ItemSearchResult[]>("search_items", { query: query.trim() }), {
      onStart: () => setSearching(true),
      onSuccess: (found) => {
        setResults(found);
        setStatuses({});
        found.forEach((r) => {
          invoke<ItemProtoFullLite | null>("get_item_proto", { vnum: r.vnum })
            .then((p) => setStatuses((prev) => ({ ...prev, [r.vnum]: p })))
            .catch(() => setStatuses((prev) => ({ ...prev, [r.vnum]: null })));
        });
      },
      onError: () => setResults([]),
      onFinally: () => setSearching(false),
    });
  }

  async function apply(vnum: number) {
    setError(null);
    setBusyVnum(vnum);
    try {
      await setupBoxItem(vnum);
      onPick(vnum);
      setPicking(false);
      setResults([]);
      setQuery("");
      setConfirmVnum(null);
    } catch (e) {
      setError(String(e));
    } finally {
      setBusyVnum(null);
    }
  }

  function handleUseClick(vnum: number) {
    const status = statuses[vnum];
    if (status && status.type !== GIFTBOX_TYPE) {
      setConfirmVnum(vnum);
    } else {
      apply(vnum);
    }
  }

  const confirmTarget = confirmVnum !== null ? statuses[confirmVnum] : null;

  return (
    <div className="flex flex-col gap-1">
      <Button variant="outline" size="sm" onClick={() => setPicking((p) => !p)}>
        <Search className="size-3.5" /> Item suchen & einrichten…
      </Button>
      {picking && (
        <div className="flex w-96 flex-col gap-1 rounded-md border border-border bg-card p-2 shadow-md">
          <div className="flex gap-1">
            <input
              autoFocus
              value={query}
              onChange={(e) => setQuery(e.target.value)}
              onKeyDown={(e) => e.key === "Enter" && runSearch()}
              placeholder="Item nach Name oder VNUM suchen…"
              className="flex-1 rounded-md border border-border bg-background px-2 py-1 text-xs"
            />
            <Button size="sm" variant="outline" onClick={runSearch} disabled={searching}>
              <Search className="size-3.5" />
            </Button>
          </div>
          {error && <p className="text-xs text-destructive">{error}</p>}
          <div className="max-h-56 space-y-1 overflow-y-auto">
            {results.map((r) => {
              const status = statuses[r.vnum];
              const ready = status ? boxItemIsReady(status) : undefined;
              return (
                <div key={r.vnum} className="flex items-center justify-between gap-2 rounded-md px-2 py-1 text-xs hover:bg-muted">
                  <span className="flex-1 truncate">
                    {r.name} <span className="text-muted-foreground">#{r.vnum}</span>
                    {ready === true && <span className="ml-1 text-green-600">(bereits eingerichtet)</span>}
                    {ready === false && <span className="ml-1 text-amber-600">(wird umgestellt)</span>}
                  </span>
                  <Button
                    size="sm"
                    onClick={() => handleUseClick(r.vnum)}
                    disabled={status === undefined || busyVnum === r.vnum}
                  >
                    {busyVnum === r.vnum ? "…" : "Verwenden"}
                  </Button>
                </div>
              );
            })}
            {results.length === 0 && !searching && (
              <p className="p-1 text-xs text-muted-foreground">Noch keine Suche.</p>
            )}
          </div>
        </div>
      )}
      {confirmVnum !== null && (
        <div className="fixed inset-0 z-10 flex items-center justify-center bg-black/50">
          <div className="w-96 space-y-3 rounded-lg border border-border bg-card p-4">
            <p className="flex items-start gap-2 text-sm">
              <AlertTriangle className="mt-0.5 size-4 shrink-0 text-destructive" />
              <span>
                "{confirmTarget?.locale_name || confirmTarget?.name}" hat aktuell nicht den Typ GIFTBOX (Typ{" "}
                {confirmTarget?.type}). Wird jetzt auf GIFTBOX (23) + Stapelbar umgestellt - das ändert, wie sich
                dieses Item im Spiel verhält. Wirklich fortfahren?
              </span>
            </p>
            <div className="flex justify-end gap-2">
              <Button variant="outline" onClick={() => setConfirmVnum(null)}>
                Abbrechen
              </Button>
              <Button variant="destructive" onClick={() => apply(confirmVnum)}>
                Umstellen
              </Button>
            </div>
          </div>
        </div>
      )}
    </div>
  );
}

interface SpecialItemGroupEntry {
  item_ref: string;
  count: number;
  prob: number;
  rare_pct: number | null;
}

interface SpecialItemGroup {
  name: string;
  vnum: number;
  group_type: string | null;
  entries: SpecialItemGroupEntry[];
}

interface ItemSearchResult {
  vnum: number;
  name: string;
}

// Special keywords the real server accepts instead of an item vnum in a
// drop entry (source/game/src/item_manager_read_tables.cpp -
// ReadSpecialDropItemFile) - "bleeding" only exists on ENABLE_WOLFMAN
// builds, which this server has (see itemFlags.ts's own note on the same
// topic), so it's included unconditionally rather than trying to detect
// the build flag from here.
const SPECIAL_KEYWORDS = ["gold", "exp", "mob", "slow", "drain_hp", "poison", "bleeding", "group"];
const KEYWORD_LABELS: Record<string, string> = {
  gold: "Gold",
  exp: "EXP",
  mob: "Zufälliger Mob (mob)",
  slow: "Verlangsamung (slow)",
  drain_hp: "HP-Entzug (drain_hp)",
  poison: "Gift (poison)",
  bleeding: "Blutung (bleeding)",
  group: "Verschachtelte Gruppe (group)",
};

// The "count" column means something different depending on item_ref - for
// "gold"/"exp" it's the actual amount given (verified against the real
// entries this was built against: "1 gold 10 1" = 10 Yang), not a literal
// item count. Labeling it "Anzahl" regardless was confusing enough that a
// user reported not being able to find where to enter a gold amount at
// all, even though the field was already there.
function countLabelFor(itemRef: string): string {
  if (itemRef === "gold") return "Betrag (Yang)";
  if (itemRef === "exp") return "Betrag (EXP)";
  if (SPECIAL_KEYWORDS.includes(itemRef)) return "Wert";
  return "Anzahl";
}

const GROUP_TYPES: { value: string | null; label: string }[] = [
  { value: null, label: "Normal (Standard-Drop)" },
  { value: "pct", label: "pct (Wahrscheinlichkeit anders gewichtet)" },
  { value: "quest", label: "quest (von einem NPC per Quest-Skript ausgelöst)" },
  { value: "special", label: "special" },
];

function Field({ label, children }: { label: string; children: React.ReactNode }) {
  return (
    <label className="flex flex-col gap-1 text-xs text-muted-foreground">
      {label}
      {children}
    </label>
  );
}

function ItemRefEditor({
  value,
  onChange,
  icon,
  ensureIcon,
}: {
  value: string;
  onChange: (v: string) => void;
  icon: string | null | undefined;
  ensureIcon: (vnum: number) => void;
}) {
  const [picking, setPicking] = useState(false);
  const [query, setQuery] = useState("");
  const [results, setResults] = useState<ItemSearchResult[]>([]);
  const [searching, setSearching] = useState(false);

  const isKeyword = SPECIAL_KEYWORDS.includes(value);
  const numericVnum = !isKeyword ? Number(value) || null : null;

  useEffect(() => {
    if (numericVnum) ensureIcon(numericVnum);
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, [numericVnum]);

  async function runSearch() {
    if (!query.trim()) return;
    await runAsyncAction(() => invoke<ItemSearchResult[]>("search_items", { query: query.trim() }), {
      onStart: () => setSearching(true),
      onSuccess: setResults,
      onError: () => setResults([]),
      onFinally: () => setSearching(false),
    });
  }

  return (
    <div className="flex flex-col gap-1">
      <div className="flex items-center gap-1">
        <select
          value={isKeyword ? value : "__item__"}
          onChange={(e) => {
            if (e.target.value === "__item__") {
              onChange("0");
            } else {
              onChange(e.target.value);
            }
          }}
          className="rounded-md border border-border bg-background px-1 py-1 text-xs"
        >
          <option value="__item__">Item (VNUM)</option>
          {SPECIAL_KEYWORDS.map((kw) => (
            <option key={kw} value={kw}>
              {KEYWORD_LABELS[kw] ?? kw}
            </option>
          ))}
        </select>
        {!isKeyword && (
          <>
            {icon && <img src={icon} alt="" className="size-6 rounded border border-border object-contain" />}
            <input
              type="number"
              value={value}
              onChange={(e) => onChange(e.target.value)}
              className="w-24 rounded-md border border-border bg-background px-2 py-1 text-xs"
            />
            <Button variant="outline" size="sm" onClick={() => setPicking((p) => !p)}>
              <Search className="size-3.5" />
            </Button>
          </>
        )}
      </div>
      {picking && !isKeyword && (
        <div className="flex flex-col gap-1 rounded-md border border-border bg-card p-2">
          <div className="flex gap-1">
            <input
              autoFocus
              value={query}
              onChange={(e) => setQuery(e.target.value)}
              onKeyDown={(e) => e.key === "Enter" && runSearch()}
              placeholder="Item suchen…"
              className="flex-1 rounded-md border border-border bg-background px-2 py-1 text-xs"
            />
            <Button size="sm" variant="outline" onClick={runSearch} disabled={searching}>
              <Search className="size-3.5" />
            </Button>
          </div>
          {results.map((r) => (
            <button
              key={r.vnum}
              className="rounded-md px-2 py-1 text-left text-xs hover:bg-muted"
              onClick={() => {
                onChange(String(r.vnum));
                setPicking(false);
                setResults([]);
                setQuery("");
              }}
            >
              {r.name} <span className="text-muted-foreground">#{r.vnum}</span>
            </button>
          ))}
        </div>
      )}
    </div>
  );
}

export function BoxEditor() {
  const setSection = useNavigationStore((s) => s.setSection);
  const [groups, setGroups] = useState<SpecialItemGroup[] | null>(null);
  const [loading, setLoading] = useState(false);
  const [loadError, setLoadError] = useState<string | null>(null);
  const [selectedIndex, setSelectedIndex] = useState<number | null>(null);
  const [icons, setIcons] = useState<Record<number, string | null>>({});

  const [creating, setCreating] = useState(false);
  const [newName, setNewName] = useState("");
  const [newNamePreview, setNewNamePreview] = useState("");
  const [newVnum, setNewVnum] = useState("");

  const [deleteConfirm, setDeleteConfirm] = useState<number | null>(null);
  const [saving, setSaving] = useState(false);
  const [saveConfirm, setSaveConfirm] = useState(false);
  const [saveError, setSaveError] = useState<string | null>(null);
  const [saveOk, setSaveOk] = useState<string | null>(null);

  useEffect(() => {
    load();
  }, []);

  useEffect(() => {
    if (!creating) return;
    if (!newName.trim()) {
      setNewNamePreview("");
      return;
    }
    invoke<string>("sanitize_special_item_group_name", { name: newName })
      .then(setNewNamePreview)
      .catch(() => setNewNamePreview(""));
  }, [newName, creating]);

  async function load() {
    setSelectedIndex(null);
    setSaveOk(null);
    await runAsyncAction(() => invoke<SpecialItemGroup[]>("read_special_item_group_file"), {
      onStart: () => {
        setLoading(true);
        setLoadError(null);
      },
      onSuccess: setGroups,
      onError: setLoadError,
      onFinally: () => setLoading(false),
    });
  }

  function ensureIcon(vnum: number) {
    if (vnum in icons) return;
    invoke<string | null>("get_item_icon", { vnum })
      .then((url) => setIcons((prev) => ({ ...prev, [vnum]: url })))
      .catch(() => setIcons((prev) => ({ ...prev, [vnum]: null })));
  }

  function updateGroup(index: number, patch: Partial<SpecialItemGroup>) {
    setGroups((prev) => prev!.map((g, i) => (i === index ? { ...g, ...patch } : g)));
  }

  function updateEntry(groupIndex: number, entryIndex: number, patch: Partial<SpecialItemGroupEntry>) {
    setGroups((prev) =>
      prev!.map((g, gi) =>
        gi !== groupIndex ? g : { ...g, entries: g.entries.map((e, ei) => (ei === entryIndex ? { ...e, ...patch } : e)) },
      ),
    );
  }

  function addEntry(groupIndex: number) {
    setGroups((prev) =>
      prev!.map((g, gi) =>
        gi !== groupIndex ? g : { ...g, entries: [...g.entries, { item_ref: "0", count: 1, prob: 1, rare_pct: null }] },
      ),
    );
  }

  function removeEntry(groupIndex: number, entryIndex: number) {
    setGroups((prev) =>
      prev!.map((g, gi) => (gi !== groupIndex ? g : { ...g, entries: g.entries.filter((_, ei) => ei !== entryIndex) })),
    );
  }

  function createGroup() {
    if (!newName.trim() || !newNamePreview) return;
    const vnum = Number(newVnum) || 0;
    setGroups((prev) => [...(prev ?? []), { name: newNamePreview, vnum, group_type: null, entries: [] }]);
    setSelectedIndex((groups?.length ?? 0));
    setCreating(false);
    setNewName("");
    setNewVnum("");
  }

  function deleteGroup(index: number) {
    setGroups((prev) => prev!.filter((_, i) => i !== index));
    setSelectedIndex(null);
    setDeleteConfirm(null);
  }

  async function save() {
    setSaveConfirm(false);
    await runAsyncAction(() => invoke<string | null>("write_special_item_group_file", { groups }), {
      onStart: () => {
        setSaving(true);
        setSaveError(null);
      },
      onSuccess: (backupPath) => {
        setSaveOk(backupPath ? `Gespeichert (Backup: ${backupPath})` : "Gespeichert.");
      },
      onError: setSaveError,
      onFinally: () => setSaving(false),
    });
  }

  const selected = selectedIndex !== null ? groups?.[selectedIndex] : null;

  return (
    <div className="max-w-5xl space-y-6 pb-10">
      <h1 className="text-2xl font-semibold">Kisten-Editor</h1>
      <p className="text-sm text-muted-foreground">
        Bearbeitet <code>special_item_group.txt</code> - die Beute-Tabelle, aus der ein Item vom Typ{" "}
        <strong>GIFTBOX</strong> (23) beim Öffnen zieht. Die „verbleibende Anzahl" bei einer mehrfach
        öffenbaren Kiste ist einfach die Stapelanzahl des Items selbst (keine gesonderte Einstellung nötig) -
        siehe <code>ITEM_GIFTBOX</code> im Server-Quellcode: jedes Öffnen zieht 1 vom Stapel ab.
      </p>
      <p className="flex items-start gap-2 rounded-lg border border-amber-500/40 bg-amber-500/10 p-3 text-sm text-amber-700 dark:text-amber-400">
        <Info className="mt-0.5 size-4 shrink-0" />
        Damit eine Kiste diese Tabelle überhaupt nutzt, muss ihr <code>item_proto.type</code> auf{" "}
        <strong>GIFTBOX (23)</strong> stehen und „Stapelbar" gesetzt sein - über „Item suchen &amp;
        einrichten…" bei der Kisten-Item-VNUM lässt sich das jetzt auch direkt hier erledigen, statt
        umständlich im Item-Editor.
      </p>
      <div className="flex items-start gap-2 rounded-lg border border-amber-500/40 bg-amber-500/10 p-3 text-sm text-amber-700 dark:text-amber-400">
        <Info className="mt-0.5 size-4 shrink-0" />
        <span>
          Änderungen wirken erst nach einem <strong>Server-Neustart</strong> - <code>special_item_group.txt</code>{" "}
          wird nur beim Start des Game-Prozesses eingelesen (kein Client-Repack, kein <code>/reload</code>-Sub-Befehl
          dafür verfügbar - im echten Quellcode geprüft). Eine neu angelegte oder geänderte Kisten-Gruppe zeigt sich
          also erst nach einem Neustart; bis dahin meldet das Öffnen der Kiste „Du hast nichts erhalten." und die
          Kiste wird dabei nicht verbraucht.{" "}
          <button className="underline" onClick={() => setSection("server-control")}>
            Zur Server-Steuerung
          </button>
        </span>
      </div>

      <div className="flex items-center gap-2">
        <Button variant="outline" onClick={load} disabled={loading}>
          <RefreshCw className={`size-4 ${loading ? "animate-spin" : ""}`} /> Neu laden
        </Button>
        <Button variant="outline" onClick={() => setCreating(true)}>
          <Plus className="size-4" /> Neue Kisten-Gruppe
        </Button>
        <Button onClick={() => setSaveConfirm(true)} disabled={saving || !groups}>
          {saving ? "Speichere…" : "Speichern"}
        </Button>
        {saveOk && (
          <span className="flex items-center gap-1 text-sm text-green-600">
            <CheckCircle2 className="size-4" /> {saveOk}
          </span>
        )}
      </div>
      {loadError && <p className="text-sm text-destructive">{loadError}</p>}
      {saveError && <p className="whitespace-pre-wrap text-sm text-destructive">{saveError}</p>}

      {creating && (
        <div className="fixed inset-0 flex items-center justify-center bg-black/50">
          <div className="w-96 space-y-3 rounded-lg border border-border bg-card p-4">
            <p className="text-sm font-medium">Neue Kisten-Gruppe</p>
            <Field label="Name (wird bereinigt: keine Leerzeichen/Umlaute)">
              <input
                autoFocus
                value={newName}
                onChange={(e) => setNewName(e.target.value)}
                className="rounded-md border border-border bg-background px-2 py-1 text-sm"
              />
            </Field>
            {newNamePreview && (
              <p className="text-xs text-muted-foreground">
                Wird zu: <code>{newNamePreview}</code>
              </p>
            )}
            <Field label="Kisten-Item-VNUM">
              <input
                type="number"
                value={newVnum}
                onChange={(e) => setNewVnum(e.target.value)}
                className="rounded-md border border-border bg-background px-2 py-1 text-sm"
              />
            </Field>
            <BoxItemPicker onPick={(v) => setNewVnum(String(v))} />
            <BoxVnumHint vnum={Number(newVnum) || 0} />
            <div className="flex justify-end gap-2">
              <Button variant="outline" onClick={() => setCreating(false)}>
                Abbrechen
              </Button>
              <Button disabled={!newNamePreview} onClick={createGroup}>
                Anlegen
              </Button>
            </div>
          </div>
        </div>
      )}

      {groups && (
        <div className="flex gap-4">
          <div className="w-64 shrink-0 space-y-1 rounded-lg border border-border p-2">
            {groups.length === 0 && <p className="p-2 text-sm text-muted-foreground">Keine Gruppen.</p>}
            {groups.map((g, i) => (
              <button
                key={i}
                onClick={() => setSelectedIndex(i)}
                className={`flex w-full items-center gap-2 rounded-md px-2 py-1.5 text-left text-sm ${
                  selectedIndex === i ? "bg-primary text-primary-foreground" : "hover:bg-muted"
                }`}
              >
                <Package className="size-4 shrink-0" />
                <span className="flex-1 truncate">{g.name}</span>
                <span className="text-xs opacity-70">#{g.vnum}</span>
              </button>
            ))}
          </div>

          <div className="flex-1 space-y-3 rounded-lg border border-border p-4">
            {!selected && <p className="text-sm text-muted-foreground">Gruppe links auswählen oder neu anlegen.</p>}
            {selected && selectedIndex !== null && (
              <>
                <div className="flex flex-wrap items-center gap-3">
                  <Field label="Name">
                    <input
                      value={selected.name}
                      onChange={(e) => updateGroup(selectedIndex, { name: e.target.value })}
                      className="w-48 rounded-md border border-border bg-background px-2 py-1 text-sm"
                    />
                  </Field>
                  <Field label="Kisten-Item-VNUM">
                    <input
                      type="number"
                      value={selected.vnum}
                      onChange={(e) => updateGroup(selectedIndex, { vnum: Number(e.target.value) || 0 })}
                      className="w-28 rounded-md border border-border bg-background px-2 py-1 text-sm"
                    />
                    <BoxItemPicker onPick={(v) => updateGroup(selectedIndex, { vnum: v })} />
                    <BoxVnumHint vnum={selected.vnum} />
                  </Field>
                  <Field label="Typ">
                    <select
                      value={selected.group_type ?? ""}
                      onChange={(e) => updateGroup(selectedIndex, { group_type: e.target.value || null })}
                      className="rounded-md border border-border bg-background px-2 py-1 text-sm"
                    >
                      {GROUP_TYPES.map((t) => (
                        <option key={t.label} value={t.value ?? ""}>
                          {t.label}
                        </option>
                      ))}
                    </select>
                  </Field>
                  <Button variant="destructive" size="sm" onClick={() => setDeleteConfirm(selectedIndex)}>
                    <Trash2 className="size-3.5" /> Gruppe löschen
                  </Button>
                </div>

                <div className="space-y-2">
                  <h2 className="text-sm font-medium text-muted-foreground">
                    Beute-Einträge ({selected.entries.length})
                  </h2>
                  {selected.entries.map((entry, ei) => (
                    <div key={ei} className="flex flex-wrap items-end gap-2 rounded-md border border-border p-2">
                      <ItemRefEditor
                        value={entry.item_ref}
                        onChange={(v) => updateEntry(selectedIndex, ei, { item_ref: v })}
                        icon={icons[Number(entry.item_ref)]}
                        ensureIcon={ensureIcon}
                      />
                      <Field label={countLabelFor(entry.item_ref)}>
                        <input
                          type="number"
                          value={entry.count}
                          onChange={(e) => updateEntry(selectedIndex, ei, { count: Number(e.target.value) || 0 })}
                          className="w-24 rounded-md border border-border bg-background px-2 py-1 text-xs"
                        />
                      </Field>
                      <Field label="Gewicht/Chance">
                        <input
                          type="number"
                          value={entry.prob}
                          onChange={(e) => updateEntry(selectedIndex, ei, { prob: Number(e.target.value) || 0 })}
                          className="w-20 rounded-md border border-border bg-background px-2 py-1 text-xs"
                        />
                      </Field>
                      <Field label="Selten-% (optional)">
                        <input
                          type="number"
                          value={entry.rare_pct ?? ""}
                          onChange={(e) =>
                            updateEntry(selectedIndex, ei, {
                              rare_pct: e.target.value === "" ? null : Number(e.target.value) || 0,
                            })
                          }
                          className="w-24 rounded-md border border-border bg-background px-2 py-1 text-xs"
                        />
                      </Field>
                      <Button variant="outline" size="sm" onClick={() => removeEntry(selectedIndex, ei)}>
                        <Trash2 className="size-3.5" />
                      </Button>
                    </div>
                  ))}
                  <Button variant="outline" size="sm" onClick={() => addEntry(selectedIndex)}>
                    <Plus className="size-3.5" /> Eintrag hinzufügen
                  </Button>
                  <p className="flex items-start gap-2 text-xs text-muted-foreground">
                    <Info className="mt-0.5 size-3.5 shrink-0" />
                    „Gewicht/Chance" ist kein Prozentwert, sondern ein relatives Gewicht gegenüber den anderen
                    Einträgen derselben Gruppe (im echten Server-Code als <code>iProb</code> verwendet) - höher
                    heißt wahrscheinlicher, aber die exakte Chance hängt von allen Einträgen zusammen ab.
                  </p>
                </div>
              </>
            )}
          </div>
        </div>
      )}

      {deleteConfirm !== null && (
        <div className="fixed inset-0 flex items-center justify-center bg-black/50">
          <div className="w-96 space-y-3 rounded-lg border border-border bg-card p-4">
            <p className="flex items-center gap-2 text-sm font-medium">
              <AlertTriangle className="size-4 text-destructive" /> Gruppe „{groups?.[deleteConfirm]?.name}" wirklich
              entfernen?
            </p>
            <p className="text-xs text-muted-foreground">
              Wird erst beim „Speichern" wirklich auf den Server geschrieben - bis dahin rückgängig machbar
              durch nicht speichern/neu laden.
            </p>
            <div className="flex justify-end gap-2">
              <Button variant="outline" onClick={() => setDeleteConfirm(null)}>
                Abbrechen
              </Button>
              <Button variant="destructive" onClick={() => deleteGroup(deleteConfirm)}>
                Entfernen
              </Button>
            </div>
          </div>
        </div>
      )}

      {saveConfirm && (
        <div className="fixed inset-0 flex items-center justify-center bg-black/50">
          <div className="w-96 space-y-3 rounded-lg border border-border bg-card p-4">
            <p className="text-sm font-medium">special_item_group.txt jetzt überschreiben?</p>
            <p className="text-xs text-muted-foreground">
              Ein Backup der aktuellen Datei wird vorher auf dem Server angelegt.
            </p>
            <div className="flex justify-end gap-2">
              <Button variant="outline" onClick={() => setSaveConfirm(false)}>
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
