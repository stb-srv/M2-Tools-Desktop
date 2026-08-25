import { useEffect, useState } from "react";
import { invoke } from "@tauri-apps/api/core";
import { Button } from "@/components/ui/button";
import { Save, Trash2 } from "lucide-react";
import type { ItemProtoInput } from "../types";

interface ItemPreset {
  id: number;
  name: string;
  item: ItemProtoInput;
}

interface ItemPresetsSectionProps {
  currentItem: ItemProtoInput;
  onLoad: (item: ItemProtoInput) => void;
}

// Named, reusable field-value templates ("Standard-Ohrring", ...) -
// independent of any real item_proto row (unlike "Referenz-Item
// übernehmen" right below this section), so creating many similar custom
// items doesn't require hunting for a reference item each time. See
// src-tauri/src/item_presets.rs.
export function ItemPresetsSection({ currentItem, onLoad }: ItemPresetsSectionProps) {
  const [presets, setPresets] = useState<ItemPreset[]>([]);
  const [loading, setLoading] = useState(false);
  const [saveName, setSaveName] = useState("");
  const [saving, setSaving] = useState(false);
  const [error, setError] = useState<string | null>(null);

  async function refresh() {
    setLoading(true);
    setError(null);
    try {
      setPresets(await invoke<ItemPreset[]>("list_item_presets"));
    } catch (e) {
      setError(String(e));
    } finally {
      setLoading(false);
    }
  }

  useEffect(() => {
    refresh();
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, []);

  async function save() {
    const name = saveName.trim();
    if (!name) return;
    setSaving(true);
    setError(null);
    try {
      await invoke("save_item_preset", { name, item: currentItem });
      setSaveName("");
      await refresh();
    } catch (e) {
      setError(String(e));
    } finally {
      setSaving(false);
    }
  }

  async function remove(id: number) {
    setError(null);
    try {
      await invoke("delete_item_preset", { id });
      await refresh();
    } catch (e) {
      setError(String(e));
    }
  }

  return (
    <section className="space-y-2 rounded-lg border border-border p-4">
      <h2 className="text-sm font-medium text-muted-foreground">Vorlagen (optional)</h2>
      <p className="text-xs text-muted-foreground">
        Eigene wiederverwendbare Vorlagen (z.B. "Standard-Ohrring") — unabhängig von einem
        bestehenden Item, für schnelles Anlegen mehrerer ähnlicher Custom-Items. Das Icon ist nie
        Teil einer Vorlage und muss immer neu gewählt werden.
      </p>

      <div className="flex items-center gap-2">
        <input
          className="h-8 flex-1 rounded-md border border-border bg-background px-2 text-sm"
          placeholder="Name der Vorlage"
          value={saveName}
          onChange={(e) => setSaveName(e.target.value)}
        />
        <Button size="sm" variant="outline" onClick={save} disabled={saving || !saveName.trim()}>
          <Save className="size-3.5" />
          Als Vorlage speichern
        </Button>
      </div>

      {loading ? (
        <p className="text-xs text-muted-foreground">Lade…</p>
      ) : presets.length === 0 ? (
        <p className="text-xs text-muted-foreground">Noch keine Vorlagen gespeichert.</p>
      ) : (
        <div className="space-y-1">
          {presets.map((p) => (
            <div key={p.id} className="flex items-center gap-2 rounded-md px-2 py-1 text-sm hover:bg-muted">
              <span className="flex-1 truncate">{p.name}</span>
              <Button size="sm" variant="outline" onClick={() => onLoad(p.item)}>
                Übernehmen
              </Button>
              <Button size="sm" variant="ghost" onClick={() => remove(p.id)} title="Vorlage löschen">
                <Trash2 className="size-3.5" />
              </Button>
            </div>
          ))}
        </div>
      )}

      {error && <p className="text-xs text-destructive">{error}</p>}
    </section>
  );
}
