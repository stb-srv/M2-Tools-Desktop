import { useEffect, useMemo, useState } from "react";
import { invoke } from "@tauri-apps/api/core";
import { Button } from "@/components/ui/button";
import {
  Search,
  ArrowRight,
  Pencil,
  Save,
  X,
  AlertTriangle,
  Info,
  Coins,
  Percent,
  Plus,
  Trash2,
  MapPin,
  Store,
  Loader2,
  HelpCircle,
} from "lucide-react";
import { useNavigationStore } from "@/store/navigation";
import { openManual } from "@/lib/manual";
import { EntityBrowser } from "@/features/shared/EntityBrowser";

interface RefineMaterial {
  vnum: number;
  count: number;
  locale_name: string | null;
}

interface RefineRecipe {
  id: number;
  cost: number;
  prob: number;
  materials: RefineMaterial[];
  used_by_count: number;
}

interface RefineChainNode {
  vnum: number;
  locale_name: string;
  refine_set: number;
  refined_vnum: number;
  recipe: RefineRecipe | null;
}

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

interface ShopSource {
  shop_vnum: number;
  shop_name: string;
  npc_vnum: number;
  count: number;
}

function ItemIcon({ vnum, cache, setCache }: { vnum: number; cache: Record<number, string | null>; setCache: (vnum: number, url: string | null) => void }) {
  useEffect(() => {
    if (vnum in cache) return;
    invoke<string | null>("get_item_icon", { vnum })
      .then((url) => setCache(vnum, url))
      .catch(() => setCache(vnum, null));
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, [vnum]);

  const url = cache[vnum];
  return (
    <div className="flex size-10 shrink-0 items-center justify-center rounded-md border border-border bg-muted/40">
      {url ? <img src={url} alt="" className="size-full object-contain" /> : <span className="text-[9px] text-muted-foreground">#{vnum}</span>}
    </div>
  );
}

interface MaterialRow {
  vnum: number;
  name: string;
  count: number;
}

// Empty materials always pad up to 5 slots in the edit form (matching
// refine_proto's fixed vnum0-4/count0-4 storage) so removing a row just
// means clearing it, no array-length juggling.
function emptyMaterialRow(): MaterialRow {
  return { vnum: 0, name: "", count: 1 };
}

function MaterialSourceHint({
  material,
  mobDrops,
  loadMobDrops,
  mobDropsLoading,
}: {
  material: RefineMaterial;
  mobDrops: MobDropGroup[] | null;
  loadMobDrops: () => void;
  mobDropsLoading: boolean;
}) {
  const [open, setOpen] = useState(false);
  const [shopSources, setShopSources] = useState<ShopSource[] | null>(null);
  const [shopLoading, setShopLoading] = useState(false);

  async function toggle() {
    const next = !open;
    setOpen(next);
    if (!next) return;
    if (!mobDrops) loadMobDrops();
    if (shopSources === null) {
      setShopLoading(true);
      try {
        setShopSources(await invoke<ShopSource[]>("find_refine_shop_sources", { vnum: material.vnum }));
      } finally {
        setShopLoading(false);
      }
    }
  }

  const drops = useMemo(
    () =>
      (mobDrops ?? [])
        .map((g) => ({ group: g, drop: g.items.find((it) => it.item_vnum === material.vnum) }))
        .filter((x): x is { group: MobDropGroup; drop: MobDropItem } => !!x.drop),
    [mobDrops, material.vnum],
  );

  return (
    <div className="inline-block">
      <button
        onClick={toggle}
        className="flex items-center gap-1 text-xs text-muted-foreground underline decoration-dotted hover:text-foreground"
      >
        <MapPin className="size-3" /> Woher bekomme ich das?
      </button>
      {open && (
        <div className="mt-1 max-w-xs space-y-2 rounded-md border border-border bg-card p-2 text-xs">
          <div>
            <p className="mb-1 flex items-center gap-1 font-medium">
              <Loader2 className={`size-3 ${mobDropsLoading ? "animate-spin" : "hidden"}`} /> Mob-Drops
            </p>
            {mobDropsLoading && <p className="text-muted-foreground">Lade mob_drop_item.txt…</p>}
            {!mobDropsLoading && drops.length === 0 && <p className="text-muted-foreground">Kein Mob gefunden, der das droppt.</p>}
            {drops.map(({ group, drop }) => (
              <p key={group.mob_vnum}>
                {group.name} <span className="text-muted-foreground">#{group.mob_vnum}</span> — {drop.percent}%
              </p>
            ))}
          </div>
          <div>
            <p className="mb-1 flex items-center gap-1 font-medium">
              <Store className="size-3" /> Shops
            </p>
            {shopLoading && <p className="text-muted-foreground">Lade…</p>}
            {!shopLoading && (shopSources?.length ?? 0) === 0 && <p className="text-muted-foreground">Kein Shop gefunden.</p>}
            {shopSources?.map((s) => (
              <p key={s.shop_vnum}>
                {s.shop_name} <span className="text-muted-foreground">(NPC #{s.npc_vnum})</span>
              </p>
            ))}
          </div>
        </div>
      )}
    </div>
  );
}

function RecipeEditor({
  node,
  onCancel,
  onSaved,
}: {
  node: RefineChainNode;
  onCancel: () => void;
  onSaved: () => void;
}) {
  const [targetVnum, setTargetVnum] = useState(node.refined_vnum);
  const [targetName, setTargetName] = useState<string | null>(null);
  const [cost, setCost] = useState(node.recipe?.cost ?? 0);
  const [prob, setProb] = useState(node.recipe?.prob ?? 100);
  const [materials, setMaterials] = useState<MaterialRow[]>(() => {
    const rows = (node.recipe?.materials ?? []).map((m) => ({
      vnum: m.vnum,
      name: m.locale_name ?? `#${m.vnum}`,
      count: m.count,
    }));
    while (rows.length < 5) rows.push(emptyMaterialRow());
    return rows;
  });
  const [reuseId, setReuseId] = useState("");
  const [reusePreview, setReusePreview] = useState<RefineRecipe | null>(null);
  const [reuseError, setReuseError] = useState<string | null>(null);
  const [saving, setSaving] = useState(false);
  const [error, setError] = useState<string | null>(null);
  const [materialPickerIndex, setMaterialPickerIndex] = useState<number | null>(null);

  const usedByOthers = (node.recipe?.used_by_count ?? 0) > 1;

  async function loadReusePreview() {
    setReuseError(null);
    setReusePreview(null);
    const id = Number(reuseId);
    if (!Number.isInteger(id) || id <= 0) {
      setReuseError("Bitte eine gültige Rezept-ID eingeben.");
      return;
    }
    try {
      const recipe = await invoke<RefineRecipe | null>("get_refine_recipe", { id });
      if (!recipe) {
        setReuseError(`Rezept ${id} existiert nicht.`);
        return;
      }
      setReusePreview(recipe);
    } catch (e) {
      setReuseError(String(e));
    }
  }

  async function saveNewOrUpdate(asNew: boolean) {
    setSaving(true);
    setError(null);
    try {
      const activeMaterials = materials.filter((m) => m.vnum > 0 && m.count > 0);
      const id = asNew ? null : (node.recipe?.id ?? null);
      const newId = await invoke<number>("save_refine_recipe", {
        id,
        cost,
        prob,
        materials: activeMaterials.map((m) => ({ vnum: m.vnum, count: m.count, locale_name: null })),
      });
      await invoke("set_item_refine_link", { vnum: node.vnum, refineSet: newId, refinedVnum: targetVnum });
      onSaved();
    } catch (e) {
      setError(String(e));
    } finally {
      setSaving(false);
    }
  }

  async function applyReuse() {
    if (!reusePreview) return;
    setSaving(true);
    setError(null);
    try {
      await invoke("set_item_refine_link", { vnum: node.vnum, refineSet: reusePreview.id, refinedVnum: targetVnum });
      onSaved();
    } catch (e) {
      setError(String(e));
    } finally {
      setSaving(false);
    }
  }

  async function clearRecipe() {
    setSaving(true);
    setError(null);
    try {
      await invoke("set_item_refine_link", { vnum: node.vnum, refineSet: 0, refinedVnum: targetVnum });
      onSaved();
    } catch (e) {
      setError(String(e));
    } finally {
      setSaving(false);
    }
  }

  return (
    <div className="space-y-3 rounded-md border border-primary/40 bg-primary/5 p-3 text-sm">
      <div className="flex items-center justify-between">
        <p className="font-medium">
          {node.locale_name} <span className="text-muted-foreground">#{node.vnum}</span> bearbeiten
        </p>
        <Button variant="ghost" size="sm" onClick={onCancel}>
          <X className="size-4" />
        </Button>
      </div>

      <div className="space-y-1">
        <p className="text-xs text-muted-foreground">Ziel-Item (worauf wird geuppt, 0 = kein weiteres Upgrade)</p>
        <div className="flex items-center gap-2">
          <input
            type="number"
            value={targetVnum}
            onChange={(e) => {
              setTargetVnum(Number(e.target.value) || 0);
              setTargetName(null);
            }}
            className="w-28 rounded-md border border-border bg-background px-2 py-1 text-sm"
          />
          {targetName && <span className="text-xs text-muted-foreground">{targetName}</span>}
        </div>
        <EntityBrowser
          kind="item"
          onPick={(item) => {
            setTargetVnum(item.vnum);
            setTargetName(item.name);
          }}
        />
      </div>

      {usedByOthers && (
        <p className="flex items-start gap-2 rounded-md bg-amber-500/10 p-2 text-xs text-amber-700 dark:text-amber-400">
          <AlertTriangle className="mt-0.5 size-3.5 shrink-0" />
          Dieses Rezept (#{node.recipe!.id}) wird von {node.recipe!.used_by_count} Items verwendet. „Speichern" ändert
          es für alle davon - „Als neues Rezept speichern" trennt nur dieses Item ab.
        </p>
      )}

      <div className="grid grid-cols-2 gap-3">
        <label className="flex flex-col gap-1 text-xs text-muted-foreground">
          <span className="flex items-center gap-1">
            <Coins className="size-3.5" /> Kosten (Gold)
          </span>
          <input
            type="number"
            value={cost}
            onChange={(e) => setCost(Number(e.target.value) || 0)}
            className="rounded-md border border-border bg-background px-2 py-1 text-sm"
          />
        </label>
        <label className="flex flex-col gap-1 text-xs text-muted-foreground">
          <span className="flex items-center gap-1">
            <Percent className="size-3.5" /> Erfolgschance (%)
          </span>
          <input
            type="number"
            min={0}
            max={100}
            value={prob}
            onChange={(e) => setProb(Number(e.target.value) || 0)}
            className="rounded-md border border-border bg-background px-2 py-1 text-sm"
          />
        </label>
      </div>

      <div className="space-y-1">
        <p className="text-xs text-muted-foreground">Materialien (bis zu 5, Anzahl 0 = leerer Slot)</p>
        {materials.map((m, i) => (
          <div key={i} className="flex items-center gap-2">
            <span className="w-40 truncate text-sm">{m.vnum ? `${m.name} (#${m.vnum})` : "— leer —"}</span>
            <input
              type="number"
              min={0}
              value={m.count}
              onChange={(e) =>
                setMaterials((prev) => prev.map((row, idx) => (idx === i ? { ...row, count: Number(e.target.value) || 0 } : row)))
              }
              className="w-16 rounded-md border border-border bg-background px-2 py-1 text-sm"
            />
            <Button variant="outline" size="sm" onClick={() => setMaterialPickerIndex(materialPickerIndex === i ? null : i)}>
              <Search className="size-3.5" />
            </Button>
            {m.vnum > 0 && (
              <Button
                variant="outline"
                size="sm"
                onClick={() => setMaterials((prev) => prev.map((row, idx) => (idx === i ? emptyMaterialRow() : row)))}
              >
                <Trash2 className="size-3.5" />
              </Button>
            )}
          </div>
        ))}
        {materialPickerIndex !== null && (
          <EntityBrowser
            kind="item"
            pickLabel="Übernehmen"
            onPick={(item) => {
              setMaterials((prev) =>
                prev.map((row, idx) => (idx === materialPickerIndex ? { vnum: item.vnum, name: item.name, count: row.count || 1 } : row)),
              );
              setMaterialPickerIndex(null);
            }}
          />
        )}
      </div>

      <div className="flex flex-wrap items-center gap-2 border-t border-border pt-2">
        <Button size="sm" disabled={saving} onClick={() => saveNewOrUpdate(false)}>
          <Save className="size-3.5" /> {node.recipe ? "Speichern (bestehendes Rezept ändern)" : "Neues Rezept anlegen"}
        </Button>
        {node.recipe && (
          <Button size="sm" variant="outline" disabled={saving} onClick={() => saveNewOrUpdate(true)}>
            <Plus className="size-3.5" /> Als neues Rezept speichern
          </Button>
        )}
        {node.recipe && (
          <Button size="sm" variant="destructive" disabled={saving} onClick={clearRecipe}>
            Rezept entfernen
          </Button>
        )}
      </div>

      <details className="border-t border-border pt-2">
        <summary className="cursor-pointer text-xs text-muted-foreground">Stattdessen bestehendes Rezept per ID wiederverwenden…</summary>
        <div className="mt-2 flex items-center gap-2">
          <input
            value={reuseId}
            onChange={(e) => setReuseId(e.target.value)}
            placeholder="Rezept-ID"
            className="w-28 rounded-md border border-border bg-background px-2 py-1 text-sm"
          />
          <Button size="sm" variant="outline" onClick={loadReusePreview}>
            Vorschau
          </Button>
        </div>
        {reuseError && <p className="mt-1 text-xs text-destructive">{reuseError}</p>}
        {reusePreview && (
          <div className="mt-2 space-y-1 rounded-md bg-muted/40 p-2 text-xs">
            <p>
              Kosten: {reusePreview.cost} Gold, Chance: {reusePreview.prob}%, verwendet von {reusePreview.used_by_count} Item(s)
            </p>
            <p>
              Materialien:{" "}
              {reusePreview.materials.length === 0
                ? "keine"
                : reusePreview.materials.map((m) => `${m.locale_name ?? "#" + m.vnum} x${m.count}`).join(", ")}
            </p>
            <Button size="sm" disabled={saving} onClick={applyReuse}>
              Dieses Rezept übernehmen
            </Button>
          </div>
        )}
      </details>

      {error && <p className="text-sm text-destructive">{error}</p>}
    </div>
  );
}

export function RefineEditor() {
  const setSection = useNavigationStore((s) => s.setSection);
  const [chain, setChain] = useState<RefineChainNode[] | null>(null);
  const [loading, setLoading] = useState(false);
  const [error, setError] = useState<string | null>(null);
  const [editingVnum, setEditingVnum] = useState<number | null>(null);
  const [iconCache, setIconCache] = useState<Record<number, string | null>>({});
  const [mobDrops, setMobDrops] = useState<MobDropGroup[] | null>(null);
  const [mobDropsLoading, setMobDropsLoading] = useState(false);

  function setIcon(vnum: number, url: string | null) {
    setIconCache((prev) => ({ ...prev, [vnum]: url }));
  }

  async function loadMobDrops() {
    if (mobDrops || mobDropsLoading) return;
    setMobDropsLoading(true);
    try {
      setMobDrops(await invoke<MobDropGroup[]>("read_mob_drop_file"));
    } catch {
      setMobDrops([]); // best-effort - material hints just show "not found" if this fails
    } finally {
      setMobDropsLoading(false);
    }
  }

  async function loadChain(vnum: number) {
    setLoading(true);
    setError(null);
    setEditingVnum(null);
    try {
      setChain(await invoke<RefineChainNode[]>("get_refine_chain", { vnum }));
    } catch (e) {
      setError(String(e));
      setChain(null);
    } finally {
      setLoading(false);
    }
  }

  return (
    <div className="max-w-4xl space-y-6 pb-10">
      <div className="flex items-center gap-2">
        <h1 className="text-2xl font-semibold">Aufwertungs-Editor (Refine)</h1>
        <Button variant="ghost" size="icon-sm" title="Hilfe zu diesem Modul" onClick={() => openManual("refine-editor")}>
          <HelpCircle className="size-4" />
        </Button>
      </div>
      <p className="text-sm text-muted-foreground">
        Visualisiert die komplette Aufwertungs-Kette eines Items (Materialien, Gold-Kosten, Erfolgschance je Stufe)
        statt roher <code>refine_proto</code>-Zeilen. <code>refine_set</code> ist dabei eine reine Rezept-ID, die mit
        der Item-Vnum nichts zu tun hat - genau das macht diese Ansicht praktisch.
      </p>

      <div className="flex items-start gap-2 rounded-lg border border-amber-500/40 bg-amber-500/10 p-3 text-sm text-amber-700 dark:text-amber-400">
        <Info className="mt-0.5 size-4 shrink-0" />
        <span>
          Änderungen wirken erst nach einem Server-Neustart - der DB-Prozess lädt <code>refine_proto</code> nur beim
          Start aus der Datenbank, anders als bei Item-Icons/-Modellen gibt es hier keinen Client-Repack.{" "}
          <button className="underline" onClick={() => setSection("server-control")}>
            Zur Server-Steuerung
          </button>
        </span>
      </div>

      <p className="text-xs text-muted-foreground">
        Andere Items können die Erfolgschance einer Aufwertung beeinflussen (Aufwertungs-Schriftrollen).{" "}
        <button className="underline" onClick={() => setSection("item-editor")}>
          Neue Aufwertungs-Schriftrolle erstellen
        </button>
      </p>

      <section className="space-y-3 rounded-lg border border-border p-4">
        <h2 className="text-sm font-medium text-muted-foreground">Item wählen</h2>
        <EntityBrowser kind="item" pickLabel="Kette laden" onPick={(item) => loadChain(item.vnum)} />
      </section>

      {loading && <p className="text-sm text-muted-foreground">Lade Kette…</p>}
      {error && <p className="text-sm text-destructive">{error}</p>}

      {chain && (
        <section className="space-y-4 rounded-lg border border-border p-4">
          <h2 className="text-sm font-medium text-muted-foreground">Aufwertungs-Kette ({chain.length} Stufe(n))</h2>
          <div className="flex flex-col gap-3">
            {chain.map((node, i) => (
              <div key={node.vnum}>
                <div className="flex items-center gap-3">
                  <ItemIcon vnum={node.vnum} cache={iconCache} setCache={setIcon} />
                  <div className="flex-1">
                    <p className="text-sm font-medium">
                      {node.locale_name} <span className="text-muted-foreground">#{node.vnum}</span>
                    </p>
                    {node.refine_set > 0 && !node.recipe && (
                      <p className="flex items-center gap-1 text-xs text-destructive">
                        <AlertTriangle className="size-3" /> Rezept #{node.refine_set} nicht gefunden (verwaiste Verknüpfung)
                      </p>
                    )}
                  </div>
                  <Button variant="outline" size="sm" onClick={() => setEditingVnum(editingVnum === node.vnum ? null : node.vnum)}>
                    <Pencil className="size-3.5" /> Bearbeiten
                  </Button>
                </div>

                {editingVnum === node.vnum && (
                  <div className="mt-2 ml-[3.25rem]">
                    <RecipeEditor node={node} onCancel={() => setEditingVnum(null)} onSaved={() => loadChain(chain[0].vnum)} />
                  </div>
                )}

                {i < chain.length - 1 && node.recipe && (
                  <div className="ml-5 flex items-center gap-3 border-l-2 border-dashed border-border py-2 pl-5 text-xs text-muted-foreground">
                    <ArrowRight className="size-4 shrink-0" />
                    <span className="flex items-center gap-1">
                      <Coins className="size-3.5" /> {node.recipe.cost.toLocaleString("de-DE")} Gold
                    </span>
                    <span className="flex items-center gap-1">
                      <Percent className="size-3.5" /> {node.recipe.prob}%
                    </span>
                    {node.recipe.materials.length > 0 ? (
                      <div className="flex flex-wrap items-center gap-2">
                        {node.recipe.materials.map((m) => (
                          <span key={m.vnum} className="flex items-center gap-1 rounded-md border border-border px-1.5 py-0.5">
                            {m.locale_name ?? `#${m.vnum}`} x{m.count}
                            <MaterialSourceHint
                              material={m}
                              mobDrops={mobDrops}
                              loadMobDrops={loadMobDrops}
                              mobDropsLoading={mobDropsLoading}
                            />
                          </span>
                        ))}
                      </div>
                    ) : (
                      <span>keine Materialien nötig</span>
                    )}
                  </div>
                )}
                {i < chain.length - 1 && !node.recipe && (
                  <div className="ml-5 flex items-center gap-2 border-l-2 border-dashed border-destructive/40 py-2 pl-5 text-xs text-destructive">
                    <ArrowRight className="size-4 shrink-0" /> Kein Rezept verknüpft - Upgrade nicht möglich, obwohl ein Ziel-Item gesetzt ist
                  </div>
                )}
              </div>
            ))}
          </div>
        </section>
      )}
    </div>
  );
}
