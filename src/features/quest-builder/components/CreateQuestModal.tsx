import { useEffect, useMemo, useState } from "react";
import { invoke } from "@tauri-apps/api/core";
import { Button } from "@/components/ui/button";
import { Search, Plus, X, Wand2, Sparkles } from "lucide-react";
import { EntityBrowser } from "@/features/shared/EntityBrowser";
import {
  DEFAULT_FORM,
  DEFAULT_DUNGEON_FORM,
  DEFAULT_FLOOR,
  DEFAULT_MULTI_STEP_FORM,
  DEFAULT_STEP,
  generateQuestLua,
  generateDungeonQuest,
  generateMultiStepQuest,
  TEMPLATE_HINTS,
  TEMPLATE_LABELS,
  STEP_LABELS,
  type QuestFormState,
  type DungeonFormState,
  type MultiStepFormState,
  type QuestStep,
  type StepKind,
  type TemplateType,
} from "../questTemplates";
import {
  resolveDescription,
  pickBestMatch,
  type LookupKind,
  type NameCandidate,
  type UnresolvedLookup,
} from "../questDescriptionParser";
import { APPLY_TYPES } from "@/features/item-editor/itemFlags";
import { Field, VnumInput, Modal } from "./shared";

interface PickResult {
  vnum: number;
  name: string;
}

// NPCs and monsters are the same `mob_proto` table on this server (see
// db/shop.rs::list_shops, which joins it for shop NPC names) - "npc"/"mob"
// both search it, only "item" searches item_proto.
type PickerKind = "npc" | "mob" | "item";

export function CreateQuestModal({
  onClose,
  onCreated,
}: {
  onClose: () => void;
  onCreated: (path: string) => void | Promise<void>;
}) {
  const [newCategory, setNewCategory] = useState("");
  const [newCategoryPreview, setNewCategoryPreview] = useState("");
  const [newName, setNewName] = useState("");
  const [newNamePreview, setNewNamePreview] = useState("");
  const [templateType, setTemplateType] = useState<TemplateType>("dialog");
  const [form, setForm] = useState<QuestFormState>(DEFAULT_FORM);
  const [dungeonForm, setDungeonForm] = useState<DungeonFormState>(DEFAULT_DUNGEON_FORM);
  const [multiStepForm, setMultiStepForm] = useState<MultiStepFormState>(DEFAULT_MULTI_STEP_FORM);
  const [descriptionText, setDescriptionText] = useState("");
  const [descriptionBusy, setDescriptionBusy] = useState(false);
  const [descriptionNotes, setDescriptionNotes] = useState<string[] | null>(null);
  const [descriptionUnresolved, setDescriptionUnresolved] = useState<UnresolvedLookup[]>([]);
  const [createError, setCreateError] = useState<string | null>(null);
  const [creatingBusy, setCreatingBusy] = useState(false);

  const [picker, setPicker] = useState<{
    kind: PickerKind;
    onPick: (vnum: number) => void;
    initialQuery?: string;
  } | null>(null);

  useEffect(() => {
    if (!newCategory.trim()) {
      setNewCategoryPreview("");
    } else {
      invoke<string>("sanitize_quest_identifier", { name: newCategory })
        .then(setNewCategoryPreview)
        .catch(() => setNewCategoryPreview(""));
    }
    if (!newName.trim()) {
      setNewNamePreview("");
    } else {
      invoke<string>("sanitize_quest_identifier", { name: newName })
        .then(setNewNamePreview)
        .catch(() => setNewNamePreview(""));
    }
  }, [newCategory, newName]);

  function updateForm(patch: Partial<QuestFormState>) {
    setForm((prev) => ({ ...prev, ...patch }));
  }

  function updateDungeonForm(patch: Partial<DungeonFormState>) {
    setDungeonForm((prev) => ({ ...prev, ...patch }));
  }

  function updateFloor(index: number, patch: Partial<(typeof dungeonForm.floors)[number]>) {
    setDungeonForm((prev) => ({
      ...prev,
      floors: prev.floors.map((f, i) => (i === index ? { ...f, ...patch } : f)),
    }));
  }

  function addFloor() {
    setDungeonForm((prev) => ({ ...prev, floors: [...prev.floors, { ...DEFAULT_FLOOR }] }));
  }

  function removeFloor(index: number) {
    setDungeonForm((prev) => ({
      ...prev,
      floors: prev.floors.length > 1 ? prev.floors.filter((_, i) => i !== index) : prev.floors,
    }));
  }

  function updateStep(index: number, patch: Partial<QuestStep>) {
    setMultiStepForm((prev) => ({
      ...prev,
      steps: prev.steps.map((s, i) => (i === index ? { ...s, ...patch } : s)),
    }));
  }

  function addStep() {
    setMultiStepForm((prev) => ({ ...prev, steps: [...prev.steps, { ...DEFAULT_STEP }] }));
  }

  function removeStep(index: number) {
    setMultiStepForm((prev) => ({
      ...prev,
      steps: prev.steps.length > 1 ? prev.steps.filter((_, i) => i !== index) : prev.steps,
    }));
  }

  function updateMultiStepForm(patch: Partial<Omit<MultiStepFormState, "steps">>) {
    setMultiStepForm((prev) => ({ ...prev, ...patch }));
  }

  // "Semi-KI"-Freitext-Assistent: reine Mustererkennung
  // (questDescriptionParser.ts), keine echte KI-Anbindung. Löst erkannte
  // NPC-/Item-/Mob-Namen über dieselben search_items/search_mobs-Befehle
  // wie der VNUM-Picker auf und übernimmt das Ergebnis in den bereits
  // vorhandenen Mehrschritt-Baukasten zur Prüfung - erzeugt selbst nichts,
  // "Anlegen" bleibt der einzige Weg, wirklich eine Quest zu erstellen.
  async function analyzeDescription() {
    if (!descriptionText.trim()) return;
    setDescriptionBusy(true);
    setDescriptionNotes(null);
    setDescriptionUnresolved([]);
    try {
      const lookup = async (phrase: string, kind: LookupKind): Promise<NameCandidate | null> => {
        const command = kind === "item" ? "search_items" : "search_mobs";
        const results = await invoke<PickResult[]>(command, { query: phrase });
        return pickBestMatch(phrase, results);
      };
      const { steps, notes, unresolved, repeatable, cooldownDays } = await resolveDescription(
        descriptionText,
        lookup,
      );
      setTemplateType("multi_step");
      setMultiStepForm({
        steps: steps.length > 0 ? steps : [{ ...DEFAULT_STEP }],
        repeatable,
        cooldownDays: cooldownDays !== null ? String(cooldownDays) : DEFAULT_MULTI_STEP_FORM.cooldownDays,
      });
      setDescriptionNotes(notes);
      setDescriptionUnresolved(unresolved);
    } finally {
      setDescriptionBusy(false);
    }
  }

  // Erlaubt der Hinweis-Liste, direkt einen bestimmten Schritt-Fehlschlag
  // nachzubessern - öffnet denselben Picker wie das VNUM-Feld selbst, nur
  // mit der bereits erkannten Namensphrase vorausgefüllt.
  function resolveUnresolved(entry: UnresolvedLookup) {
    openPicker(
      entry.kind === "npc" ? "npc" : entry.kind === "mob" ? "mob" : "item",
      (v) => updateStep(entry.stepIndex, { [entry.field]: String(v) }),
      entry.query,
    );
    setDescriptionUnresolved((prev) => prev.filter((u) => u !== entry));
  }

  const preview = useMemo(() => {
    const name = newNamePreview || "neue_quest";
    try {
      if (templateType === "dungeon") return generateDungeonQuest(name, dungeonForm);
      if (templateType === "multi_step") return generateMultiStepQuest(name, multiStepForm);
      return generateQuestLua(name, templateType, form);
    } catch {
      return "";
    }
  }, [newNamePreview, templateType, form, dungeonForm, multiStepForm]);

  async function submitCreate() {
    if (!newCategoryPreview || !newNamePreview) return;
    setCreatingBusy(true);
    setCreateError(null);
    try {
      const path = await invoke<string>("create_quest_file", {
        category: newCategoryPreview,
        name: newNamePreview,
        content: preview,
        extension: templateType === "dungeon" ? "quest" : "lua",
      });
      onClose();
      await onCreated(path);
    } catch (e) {
      setCreateError(String(e));
    } finally {
      setCreatingBusy(false);
    }
  }

  function openPicker(kind: PickerKind, onPick: (vnum: number) => void, initialQuery?: string) {
    setPicker({ kind, onPick, initialQuery });
  }

  function pickValue(vnum: number) {
    if (!picker) return;
    picker.onPick(vnum);
    setPicker(null);
  }

  return (
    <>
      <Modal onClose={onClose} wide>
        <div className="flex max-h-[80vh] gap-4">
          <div className="w-80 shrink-0 space-y-3 overflow-y-auto pr-2">
            <p className="text-sm font-medium">Neue Quest anlegen</p>
            <div className="flex gap-2">
              <Field label="Kategorie (Ordner)">
                <input
                  value={newCategory}
                  onChange={(e) => setNewCategory(e.target.value)}
                  placeholder="z.B. Biologie"
                  className="w-36 rounded-md border border-border bg-background px-2 py-1 text-sm"
                />
              </Field>
              <Field label="Name">
                <input
                  value={newName}
                  onChange={(e) => setNewName(e.target.value)}
                  placeholder="z.B. Orkzahn"
                  className="w-36 rounded-md border border-border bg-background px-2 py-1 text-sm"
                />
              </Field>
            </div>
            {(newCategory.trim() || newName.trim()) && (
              <p className="text-xs text-muted-foreground">
                Wird gespeichert als:{" "}
                <code>
                  {newCategoryPreview || "…"}/{newNamePreview || "…"}.
                  {templateType === "dungeon" ? "quest" : "lua"}
                </code>
              </p>
            )}

            <div className="space-y-2 rounded-md border border-border p-2">
              <div className="flex items-center gap-1.5 text-xs font-medium text-muted-foreground">
                <Sparkles className="size-3.5" />
                Freitext-Assistent (optional)
              </div>
              <p className="text-xs text-muted-foreground">
                Mustererkennung, keine echte KI - einfache, klare Sätze funktionieren am besten
                (z.B. "Rede mit Hans, sammle 10 Wolfsfelle, dann bekommt man 100 Yang."). NPC und
                Item am besten mit "bei" trennen ("gib 20 Orkzähne bei Gemi ab") statt in einer
                Aufzählung ("bringe der Gemi, 20 Orkzähne") - sonst kann die Zuordnung danebengehen.
                Füllt den Mehrschritt-Baukasten unten vor - Ergebnis vor dem Anlegen prüfen!
              </p>
              <textarea
                value={descriptionText}
                onChange={(e) => setDescriptionText(e.target.value)}
                placeholder="Rede mit Hans, sammle 10 Wolfsfelle, dann bekommt man 100 Yang."
                className="h-16 w-full rounded-md border border-border bg-background px-2 py-1 text-sm"
              />
              <Button
                variant="outline"
                size="sm"
                onClick={analyzeDescription}
                disabled={!descriptionText.trim() || descriptionBusy}
              >
                <Sparkles className="size-3.5" />
                {descriptionBusy ? "Analysiere…" : "Analysieren"}
              </Button>
              {descriptionNotes && descriptionNotes.length > 0 && (
                <div className="space-y-1.5 rounded-md border border-amber-500/40 bg-amber-500/10 p-2 text-xs text-amber-700 dark:text-amber-400">
                  {descriptionNotes.map((note, i) => (
                    <p key={i}>{note}</p>
                  ))}
                  {descriptionUnresolved.map((entry, i) => (
                    <div key={i} className="flex items-center justify-between gap-2">
                      <span>
                        "{entry.query}" nicht gefunden (Schritt {entry.stepIndex + 1})
                      </span>
                      <Button
                        variant="outline"
                        size="sm"
                        onClick={() => resolveUnresolved(entry)}
                        className="shrink-0"
                      >
                        <Search className="size-3.5" />
                        Suchen
                      </Button>
                    </div>
                  ))}
                </div>
              )}
              {descriptionNotes && descriptionNotes.length === 0 && (
                <p className="text-xs text-muted-foreground">
                  Alles erkannt - Ergebnis im Mehrschritt-Baukasten unten geprüft übernehmen.
                </p>
              )}
            </div>

            <Field label="Vorlage">
              <select
                value={templateType}
                onChange={(e) => setTemplateType(e.target.value as TemplateType)}
                className="w-full rounded-md border border-border bg-background px-2 py-1 text-sm"
              >
                {Object.entries(TEMPLATE_LABELS).map(([value, label]) => (
                  <option key={value} value={value}>
                    {label}
                  </option>
                ))}
              </select>
            </Field>
            <p className="text-xs text-muted-foreground">{TEMPLATE_HINTS[templateType]}</p>

            {templateType !== "dungeon" && templateType !== "multi_step" && (
              <>
                <Field label="NPC-VNUM">
                  <VnumInput
                    value={form.npcVnum}
                    onChange={(v) => updateForm({ npcVnum: v })}
                    onPick={() => openPicker("npc", (v) => updateForm({ npcVnum: String(v) }))}
                  />
                </Field>
                <Field label="Dialog-Auslöser (chat-Label)">
                  <input
                    value={form.chatLabel}
                    onChange={(e) => updateForm({ chatLabel: e.target.value })}
                    className="w-full rounded-md border border-border bg-background px-2 py-1 text-sm"
                  />
                </Field>
                <Field label="NPC-Titel (say_title)">
                  <input
                    value={form.npcTitle}
                    onChange={(e) => updateForm({ npcTitle: e.target.value })}
                    className="w-full rounded-md border border-border bg-background px-2 py-1 text-sm"
                  />
                </Field>
              </>
            )}

            {(templateType === "dialog" || templateType === "buffed_item") && (
              <Field label="Dialogtext">
                <textarea
                  value={form.dialogText}
                  onChange={(e) => updateForm({ dialogText: e.target.value })}
                  className="h-20 w-full rounded-md border border-border bg-background px-2 py-1 text-sm"
                />
              </Field>
            )}

            {templateType === "collect" && (
              <>
                <div className="flex gap-2">
                  <Field label="Item-VNUM (abzugeben)">
                    <VnumInput
                      value={form.requiredItemVnum}
                      onChange={(v) => updateForm({ requiredItemVnum: v })}
                      onPick={() =>
                        openPicker("item", (v) => updateForm({ requiredItemVnum: String(v) }))
                      }
                    />
                  </Field>
                  <Field label="Anzahl">
                    <input
                      type="number"
                      min={1}
                      value={form.requiredItemCount}
                      onChange={(e) => updateForm({ requiredItemCount: e.target.value })}
                      className="w-20 rounded-md border border-border bg-background px-2 py-1 text-sm"
                    />
                  </Field>
                </div>
                <Field label="Text bei Erfolg">
                  <textarea
                    value={form.successText}
                    onChange={(e) => updateForm({ successText: e.target.value })}
                    className="h-16 w-full rounded-md border border-border bg-background px-2 py-1 text-sm"
                  />
                </Field>
                <Field label="Text wenn noch nicht genug Items">
                  <textarea
                    value={form.failText}
                    onChange={(e) => updateForm({ failText: e.target.value })}
                    className="h-16 w-full rounded-md border border-border bg-background px-2 py-1 text-sm"
                  />
                </Field>
              </>
            )}

            {templateType === "chance_collect" && (
              <>
                <div className="flex gap-2">
                  <Field label="Item-VNUM (abzugeben)">
                    <VnumInput
                      value={form.requiredItemVnum}
                      onChange={(v) => updateForm({ requiredItemVnum: v })}
                      onPick={() =>
                        openPicker("item", (v) => updateForm({ requiredItemVnum: String(v) }))
                      }
                    />
                  </Field>
                  <Field label="Anzahl pro Versuch">
                    <input
                      type="number"
                      min={1}
                      value={form.itemsPerAttempt}
                      onChange={(e) => updateForm({ itemsPerAttempt: e.target.value })}
                      className="w-20 rounded-md border border-border bg-background px-2 py-1 text-sm"
                    />
                  </Field>
                </div>
                <div className="flex gap-2">
                  <Field label="Chance der Annahme (%)">
                    <input
                      type="number"
                      min={0}
                      max={100}
                      value={form.acceptChancePercent}
                      onChange={(e) => updateForm({ acceptChancePercent: e.target.value })}
                      className="w-24 rounded-md border border-border bg-background px-2 py-1 text-sm"
                    />
                  </Field>
                  <Field label="Nötige Erfolge bis Belohnung">
                    <input
                      type="number"
                      min={1}
                      value={form.requiredSuccesses}
                      onChange={(e) => updateForm({ requiredSuccesses: e.target.value })}
                      className="w-24 rounded-md border border-border bg-background px-2 py-1 text-sm"
                    />
                  </Field>
                </div>
                <p className="text-xs text-muted-foreground">
                  Das Item wird bei jedem Versuch verbraucht - unabhängig davon, ob der Wurf
                  erfolgreich war (genau wie in Biologie/Orkzahn.lua auf dem Server).
                </p>
                <Field label="Text bei Erfolg (%d = aktueller Fortschritt, %d = Ziel)">
                  <input
                    value={form.progressText}
                    onChange={(e) => updateForm({ progressText: e.target.value })}
                    className="w-full rounded-md border border-border bg-background px-2 py-1 text-sm"
                  />
                </Field>
                <Field label="Text bei Fehlschlag">
                  <textarea
                    value={form.failText}
                    onChange={(e) => updateForm({ failText: e.target.value })}
                    className="h-16 w-full rounded-md border border-border bg-background px-2 py-1 text-sm"
                  />
                </Field>
                <Field label="Text wenn Item nicht vorhanden">
                  <textarea
                    value={form.noItemText}
                    onChange={(e) => updateForm({ noItemText: e.target.value })}
                    className="h-16 w-full rounded-md border border-border bg-background px-2 py-1 text-sm"
                  />
                </Field>
                <Field label="Text bei Abschluss (genug Erfolge erreicht)">
                  <textarea
                    value={form.completeText}
                    onChange={(e) => updateForm({ completeText: e.target.value })}
                    className="h-16 w-full rounded-md border border-border bg-background px-2 py-1 text-sm"
                  />
                </Field>
              </>
            )}

            {templateType === "kill" && (
              <>
                <div className="flex gap-2">
                  <Field label="Monster-VNUM">
                    <VnumInput
                      value={form.mobVnum}
                      onChange={(v) => updateForm({ mobVnum: v })}
                      onPick={() => openPicker("mob", (v) => updateForm({ mobVnum: String(v) }))}
                    />
                  </Field>
                  <Field label="Anzahl zu töten">
                    <input
                      type="number"
                      min={1}
                      value={form.requiredKills}
                      onChange={(e) => updateForm({ requiredKills: e.target.value })}
                      className="w-24 rounded-md border border-border bg-background px-2 py-1 text-sm"
                    />
                  </Field>
                </div>
                <Field label="Fortschrittstext (%d = aktuell, %d = Ziel)">
                  <input
                    value={form.progressText}
                    onChange={(e) => updateForm({ progressText: e.target.value })}
                    className="w-full rounded-md border border-border bg-background px-2 py-1 text-sm"
                  />
                </Field>
                <Field label="Text bei Erfolg">
                  <textarea
                    value={form.successText}
                    onChange={(e) => updateForm({ successText: e.target.value })}
                    className="h-16 w-full rounded-md border border-border bg-background px-2 py-1 text-sm"
                  />
                </Field>
              </>
            )}

            {templateType === "use" && (
              <>
                <Field label="Item-VNUM (zu benutzen)">
                  <VnumInput
                    value={form.useItemVnum}
                    onChange={(v) => updateForm({ useItemVnum: v })}
                    onPick={() =>
                      openPicker("item", (v) => updateForm({ useItemVnum: String(v) }))
                    }
                  />
                </Field>
                <Field label="Text beim Benutzen">
                  <textarea
                    value={form.useText}
                    onChange={(e) => updateForm({ useText: e.target.value })}
                    className="h-16 w-full rounded-md border border-border bg-background px-2 py-1 text-sm"
                  />
                </Field>
              </>
            )}

            {templateType === "buffed_item" && (
              <>
                <div className="flex gap-2">
                  <Field label="Zu verschenkendes Item-VNUM">
                    <VnumInput
                      value={form.buffedItemVnum}
                      onChange={(v) => updateForm({ buffedItemVnum: v })}
                      onPick={() =>
                        openPicker("item", (v) => updateForm({ buffedItemVnum: String(v) }))
                      }
                    />
                  </Field>
                  <Field label="Anzahl">
                    <input
                      type="number"
                      min={1}
                      value={form.buffedItemCount}
                      onChange={(e) => updateForm({ buffedItemCount: e.target.value })}
                      className="w-20 rounded-md border border-border bg-background px-2 py-1 text-sm"
                    />
                  </Field>
                </div>
                <p className="text-xs text-muted-foreground">
                  Für eine fertige "+9"-Waffe/Rüstung hier die VNUM der +9-Stufe eintragen (z.B. aus
                  dem Aufwertungs-Editor), nicht die +0-Basis - die Kette selbst wird hierdurch nicht
                  ausgelöst, nur ein bereits existierendes Item vergeben.
                </p>
                <div className="space-y-1 rounded-md border border-border p-2">
                  <span className="text-xs font-medium text-muted-foreground">
                    Feste Bonus-Attribute (bis zu 4, leer = Slot ungenutzt)
                  </span>
                  {([0, 1, 2, 3] as const).map((slot) => {
                    const typeKey = `buffedAttrType${slot}` as const;
                    const valueKey = `buffedAttrValue${slot}` as const;
                    return (
                      <div key={slot} className="flex gap-2">
                        <Field label={`Attribut ${slot + 1}`}>
                          <select
                            value={form[typeKey]}
                            onChange={(e) => updateForm({ [typeKey]: e.target.value })}
                            className="w-56 rounded-md border border-border bg-background px-2 py-1 text-sm"
                          >
                            {APPLY_TYPES.map((t) => (
                              <option key={t.value} value={t.value}>
                                {t.label}
                              </option>
                            ))}
                          </select>
                        </Field>
                        <Field label="Wert">
                          <input
                            type="number"
                            value={form[valueKey]}
                            onChange={(e) => updateForm({ [valueKey]: e.target.value })}
                            className="w-24 rounded-md border border-border bg-background px-2 py-1 text-sm"
                          />
                        </Field>
                      </div>
                    );
                  })}
                </div>
                <Field label="Belohnung Yang (optional)">
                  <input
                    type="number"
                    value={form.rewardMoney}
                    onChange={(e) => updateForm({ rewardMoney: e.target.value })}
                    className="w-32 rounded-md border border-border bg-background px-2 py-1 text-sm"
                  />
                </Field>
              </>
            )}

            {templateType !== "dungeon" &&
              templateType !== "buffed_item" &&
              templateType !== "multi_step" && (
              <>
                <div className="flex gap-2">
                  <Field label="Belohnung Item-VNUM (optional)">
                    <VnumInput
                      value={form.rewardItemVnum}
                      onChange={(v) => updateForm({ rewardItemVnum: v })}
                      onPick={() =>
                        openPicker("item", (v) => updateForm({ rewardItemVnum: String(v) }))
                      }
                    />
                  </Field>
                  <Field label="Anzahl">
                    <input
                      type="number"
                      min={1}
                      value={form.rewardItemCount}
                      onChange={(e) => updateForm({ rewardItemCount: e.target.value })}
                      className="w-20 rounded-md border border-border bg-background px-2 py-1 text-sm"
                    />
                  </Field>
                </div>
                <Field label="Belohnung Yang (optional)">
                  <input
                    type="number"
                    value={form.rewardMoney}
                    onChange={(e) => updateForm({ rewardMoney: e.target.value })}
                    className="w-32 rounded-md border border-border bg-background px-2 py-1 text-sm"
                  />
                </Field>
              </>
            )}

            {templateType === "dungeon" && (
              <>
                <Field label="Einstiegs-NPC-VNUM">
                  <VnumInput
                    value={dungeonForm.entryNpcVnum}
                    onChange={(v) => updateDungeonForm({ entryNpcVnum: v })}
                    onPick={() =>
                      openPicker("npc", (v) => updateDungeonForm({ entryNpcVnum: String(v) }))
                    }
                  />
                </Field>
                <Field label="Dialog-Auslöser (chat-Label)">
                  <input
                    value={dungeonForm.entryChatLabel}
                    onChange={(e) => updateDungeonForm({ entryChatLabel: e.target.value })}
                    className="w-full rounded-md border border-border bg-background px-2 py-1 text-sm"
                  />
                </Field>
                <Field label="NPC-Titel (say_title)">
                  <input
                    value={dungeonForm.entryTitle}
                    onChange={(e) => updateDungeonForm({ entryTitle: e.target.value })}
                    className="w-full rounded-md border border-border bg-background px-2 py-1 text-sm"
                  />
                </Field>
                <Field label="Einstiegstext">
                  <textarea
                    value={dungeonForm.entryText}
                    onChange={(e) => updateDungeonForm({ entryText: e.target.value })}
                    className="h-16 w-full rounded-md border border-border bg-background px-2 py-1 text-sm"
                  />
                </Field>
                <div className="flex gap-2">
                  <Field label="Mindest-Level (0 = keins)">
                    <input
                      type="number"
                      min={0}
                      value={dungeonForm.requiredLevel}
                      onChange={(e) => updateDungeonForm({ requiredLevel: e.target.value })}
                      className="w-24 rounded-md border border-border bg-background px-2 py-1 text-sm"
                    />
                  </Field>
                  <Field label="Dungeon-Map-Index">
                    <input
                      type="number"
                      value={dungeonForm.dungeonMapIndex}
                      onChange={(e) => updateDungeonForm({ dungeonMapIndex: e.target.value })}
                      className="w-28 rounded-md border border-border bg-background px-2 py-1 text-sm"
                    />
                  </Field>
                </div>
                <div className="flex gap-2">
                  <Field label="Map-Index-Bereich Start">
                    <input
                      type="number"
                      value={dungeonForm.mapIndexRangeStart}
                      onChange={(e) =>
                        updateDungeonForm({ mapIndexRangeStart: e.target.value })
                      }
                      className="w-28 rounded-md border border-border bg-background px-2 py-1 text-sm"
                    />
                  </Field>
                  <Field label="…bis (exklusiv)">
                    <input
                      type="number"
                      value={dungeonForm.mapIndexRangeEnd}
                      onChange={(e) => updateDungeonForm({ mapIndexRangeEnd: e.target.value })}
                      className="w-28 rounded-md border border-border bg-background px-2 py-1 text-sm"
                    />
                  </Field>
                </div>
                <p className="text-xs text-muted-foreground">
                  Der Map-Index-Bereich ist die vom Server für diesen Dungeon-Typ vergebene
                  Instanz-Map-Range (z.B. 660000–670000 beim Daemonenturm) - kommt aus der
                  Server-Dungeon-Konfiguration, nicht aus diesem Tool. Ohne den richtigen Bereich
                  lösen die Boss-Kill-Trigger unten nicht korrekt aus.
                </p>

                <div className="space-y-2 rounded-md border border-border p-2">
                  <div className="flex items-center justify-between">
                    <span className="text-xs font-medium text-muted-foreground">
                      Etagen ({dungeonForm.floors.length})
                    </span>
                    <Button variant="outline" size="sm" onClick={addFloor}>
                      <Plus className="size-3.5" />
                      Etage
                    </Button>
                  </div>
                  {dungeonForm.floors.map((floor, index) => (
                    <div key={index} className="space-y-1 rounded-md border border-border p-2">
                      <div className="flex items-center justify-between">
                        <span className="text-xs font-medium">Etage {index + 1}</span>
                        {dungeonForm.floors.length > 1 && (
                          <Button
                            variant="ghost"
                            size="icon-sm"
                            onClick={() => removeFloor(index)}
                          >
                            <X className="size-3.5" />
                          </Button>
                        )}
                      </div>
                      <Field label="Boss-/Trigger-Monster-VNUM (löst nächste Etage aus)">
                        <VnumInput
                          value={floor.bossVnum}
                          onChange={(v) => updateFloor(index, { bossVnum: v })}
                          onPick={() =>
                            openPicker("mob", (v) => updateFloor(index, { bossVnum: String(v) }))
                          }
                        />
                      </Field>
                      <div className="flex gap-2">
                        <Field label="Einstiegs-X">
                          <input
                            type="number"
                            value={floor.entryX}
                            onChange={(e) => updateFloor(index, { entryX: e.target.value })}
                            className="w-20 rounded-md border border-border bg-background px-2 py-1 text-sm"
                          />
                        </Field>
                        <Field label="Einstiegs-Y">
                          <input
                            type="number"
                            value={floor.entryY}
                            onChange={(e) => updateFloor(index, { entryY: e.target.value })}
                            className="w-20 rounded-md border border-border bg-background px-2 py-1 text-sm"
                          />
                        </Field>
                      </div>
                      <Field label="Regen-Datei (Monster-Spawns dieser Etage)">
                        <input
                          value={floor.regenFile}
                          onChange={(e) => updateFloor(index, { regenFile: e.target.value })}
                          placeholder="data/dungeon/meindungeon/etage1_regen.txt"
                          className="w-full rounded-md border border-border bg-background px-2 py-1 text-sm"
                        />
                      </Field>
                    </div>
                  ))}
                  <p className="text-xs text-muted-foreground">
                    Die Regen-Datei muss auf dem Server bereits existieren (eigenes Dateiformat,
                    hier nicht erzeugt) - für „Etage 1" wird sie beim Betreten geladen, für jede
                    weitere Etage beim Töten des Bosses der vorherigen Etage.
                  </p>
                </div>

                <div className="flex gap-2">
                  <Field label="Belohnung Item-VNUM (optional, letzte Etage)">
                    <VnumInput
                      value={dungeonForm.rewardItemVnum}
                      onChange={(v) => updateDungeonForm({ rewardItemVnum: v })}
                      onPick={() =>
                        openPicker("item", (v) =>
                          updateDungeonForm({ rewardItemVnum: String(v) }),
                        )
                      }
                    />
                  </Field>
                  <Field label="Anzahl">
                    <input
                      type="number"
                      min={1}
                      value={dungeonForm.rewardItemCount}
                      onChange={(e) =>
                        updateDungeonForm({ rewardItemCount: e.target.value })
                      }
                      className="w-20 rounded-md border border-border bg-background px-2 py-1 text-sm"
                    />
                  </Field>
                </div>
                <div className="flex gap-2">
                  <Field label="Belohnung Yang (optional)">
                    <input
                      type="number"
                      value={dungeonForm.rewardMoney}
                      onChange={(e) => updateDungeonForm({ rewardMoney: e.target.value })}
                      className="w-32 rounded-md border border-border bg-background px-2 py-1 text-sm"
                    />
                  </Field>
                  <Field label="Sekunden bis Rauswurf nach Abschluss">
                    <input
                      type="number"
                      min={0}
                      value={dungeonForm.exitDelaySeconds}
                      onChange={(e) =>
                        updateDungeonForm({ exitDelaySeconds: e.target.value })
                      }
                      className="w-24 rounded-md border border-border bg-background px-2 py-1 text-sm"
                    />
                  </Field>
                </div>
              </>
            )}

            {templateType === "multi_step" && (
              <div className="space-y-2 rounded-md border border-border p-2">
                <label className="flex items-center gap-2 text-sm">
                  <input
                    type="checkbox"
                    checked={multiStepForm.repeatable}
                    onChange={(e) => updateMultiStepForm({ repeatable: e.target.checked })}
                  />
                  Wiederholbar (mit Cooldown)
                </label>
                {multiStepForm.repeatable && (
                  <>
                    <Field label="Cooldown (Tage)">
                      <input
                        type="number"
                        min={0}
                        value={multiStepForm.cooldownDays}
                        onChange={(e) => updateMultiStepForm({ cooldownDays: e.target.value })}
                        className="w-24 rounded-md border border-border bg-background px-2 py-1 text-sm"
                      />
                    </Field>
                    <p className="text-xs text-muted-foreground">
                      Nach Abschluss springt die Quest zurück auf Schritt 1 statt zu enden - Schritt
                      1 ist bis zum Ablauf des Cooldowns gesperrt, alle Töten-Zähler in der Kette
                      werden bei jedem Abschluss automatisch zurückgesetzt.
                    </p>
                  </>
                )}
                <div className="flex items-center justify-between">
                  <span className="text-xs font-medium text-muted-foreground">
                    Schritte ({multiStepForm.steps.length})
                  </span>
                  <Button variant="outline" size="sm" onClick={addStep}>
                    <Plus className="size-3.5" />
                    Schritt
                  </Button>
                </div>
                {multiStepForm.steps.map((step, index) => (
                  <div key={index} className="space-y-1 rounded-md border border-border p-2">
                    <div className="flex items-center justify-between">
                      <span className="text-xs font-medium">Schritt {index + 1}</span>
                      {multiStepForm.steps.length > 1 && (
                        <Button variant="ghost" size="icon-sm" onClick={() => removeStep(index)}>
                          <X className="size-3.5" />
                        </Button>
                      )}
                    </div>
                    <Field label="Art">
                      <select
                        value={step.kind}
                        onChange={(e) => updateStep(index, { kind: e.target.value as StepKind })}
                        className="w-full rounded-md border border-border bg-background px-2 py-1 text-sm"
                      >
                        {Object.entries(STEP_LABELS).map(([value, label]) => (
                          <option key={value} value={value}>
                            {label}
                          </option>
                        ))}
                      </select>
                    </Field>

                    {step.kind !== "use" && (
                      <>
                        <div className="flex gap-2">
                          <Field label="NPC-VNUM">
                            <VnumInput
                              value={step.npcVnum}
                              onChange={(v) => updateStep(index, { npcVnum: v })}
                              onPick={() =>
                                openPicker("npc", (v) => updateStep(index, { npcVnum: String(v) }))
                              }
                            />
                          </Field>
                          <Field label="chat-Label">
                            <input
                              value={step.chatLabel}
                              onChange={(e) => updateStep(index, { chatLabel: e.target.value })}
                              className="w-28 rounded-md border border-border bg-background px-2 py-1 text-sm"
                            />
                          </Field>
                        </div>
                        <Field label="NPC-Titel (say_title)">
                          <input
                            value={step.npcTitle}
                            onChange={(e) => updateStep(index, { npcTitle: e.target.value })}
                            className="w-full rounded-md border border-border bg-background px-2 py-1 text-sm"
                          />
                        </Field>
                      </>
                    )}

                    {step.kind === "dialog" && (
                      <Field label="Dialogtext">
                        <textarea
                          value={step.dialogText}
                          onChange={(e) => updateStep(index, { dialogText: e.target.value })}
                          className="h-16 w-full rounded-md border border-border bg-background px-2 py-1 text-sm"
                        />
                      </Field>
                    )}

                    {step.kind === "collect" && (
                      <>
                        <div className="flex gap-2">
                          <Field label="Item-VNUM (abzugeben)">
                            <VnumInput
                              value={step.requiredItemVnum}
                              onChange={(v) => updateStep(index, { requiredItemVnum: v })}
                              onPick={() =>
                                openPicker("item", (v) =>
                                  updateStep(index, { requiredItemVnum: String(v) }),
                                )
                              }
                            />
                          </Field>
                          <Field label="Anzahl">
                            <input
                              type="number"
                              min={1}
                              value={step.requiredItemCount}
                              onChange={(e) =>
                                updateStep(index, { requiredItemCount: e.target.value })
                              }
                              className="w-20 rounded-md border border-border bg-background px-2 py-1 text-sm"
                            />
                          </Field>
                        </div>
                        <Field label="Text bei Erfolg">
                          <textarea
                            value={step.successText}
                            onChange={(e) => updateStep(index, { successText: e.target.value })}
                            className="h-14 w-full rounded-md border border-border bg-background px-2 py-1 text-sm"
                          />
                        </Field>
                        <Field label="Text wenn noch nicht genug Items">
                          <textarea
                            value={step.failText}
                            onChange={(e) => updateStep(index, { failText: e.target.value })}
                            className="h-14 w-full rounded-md border border-border bg-background px-2 py-1 text-sm"
                          />
                        </Field>
                      </>
                    )}

                    {step.kind === "kill" && (
                      <>
                        <div className="flex gap-2">
                          <Field label="Monster-VNUM">
                            <VnumInput
                              value={step.mobVnum}
                              onChange={(v) => updateStep(index, { mobVnum: v })}
                              onPick={() =>
                                openPicker("mob", (v) => updateStep(index, { mobVnum: String(v) }))
                              }
                            />
                          </Field>
                          <Field label="Anzahl zu töten">
                            <input
                              type="number"
                              min={1}
                              value={step.requiredKills}
                              onChange={(e) =>
                                updateStep(index, { requiredKills: e.target.value })
                              }
                              className="w-24 rounded-md border border-border bg-background px-2 py-1 text-sm"
                            />
                          </Field>
                        </div>
                        <Field label="Fortschrittstext (%d = aktuell, %d = Ziel)">
                          <input
                            value={step.progressText}
                            onChange={(e) => updateStep(index, { progressText: e.target.value })}
                            className="w-full rounded-md border border-border bg-background px-2 py-1 text-sm"
                          />
                        </Field>
                        <Field label="Text bei Erfolg">
                          <textarea
                            value={step.successText}
                            onChange={(e) => updateStep(index, { successText: e.target.value })}
                            className="h-14 w-full rounded-md border border-border bg-background px-2 py-1 text-sm"
                          />
                        </Field>
                      </>
                    )}

                    {step.kind === "use" && (
                      <>
                        <Field label="Item-VNUM (zu benutzen)">
                          <VnumInput
                            value={step.useItemVnum}
                            onChange={(v) => updateStep(index, { useItemVnum: v })}
                            onPick={() =>
                              openPicker("item", (v) => updateStep(index, { useItemVnum: String(v) }))
                            }
                          />
                        </Field>
                        <Field label="Text beim Benutzen">
                          <textarea
                            value={step.useText}
                            onChange={(e) => updateStep(index, { useText: e.target.value })}
                            className="h-14 w-full rounded-md border border-border bg-background px-2 py-1 text-sm"
                          />
                        </Field>
                      </>
                    )}

                    <div className="space-y-1 rounded-md border border-border p-2">
                      <span className="text-xs font-medium text-muted-foreground">
                        Zwischenbelohnung (optional)
                      </span>
                      <div className="flex gap-2">
                        <Field label="Item-VNUM">
                          <VnumInput
                            value={step.rewardItemVnum}
                            onChange={(v) => updateStep(index, { rewardItemVnum: v })}
                            onPick={() =>
                              openPicker("item", (v) =>
                                updateStep(index, { rewardItemVnum: String(v) }),
                              )
                            }
                          />
                        </Field>
                        <Field label="Anzahl">
                          <input
                            type="number"
                            min={1}
                            value={step.rewardItemCount}
                            onChange={(e) =>
                              updateStep(index, { rewardItemCount: e.target.value })
                            }
                            className="w-20 rounded-md border border-border bg-background px-2 py-1 text-sm"
                          />
                        </Field>
                      </div>
                      <Field label="Yang">
                        <input
                          type="number"
                          value={step.rewardMoney}
                          onChange={(e) => updateStep(index, { rewardMoney: e.target.value })}
                          className="w-32 rounded-md border border-border bg-background px-2 py-1 text-sm"
                        />
                      </Field>
                      <span className="text-xs font-medium text-muted-foreground">
                        Feste Bonus-Attribute (optional, überschreibt den Zufalls-Roll - z.B. "Max.
                        HP +250")
                      </span>
                      {([0, 1, 2, 3] as const).map((slot) => {
                        const typeKey = `rewardAttrType${slot}` as const;
                        const valueKey = `rewardAttrValue${slot}` as const;
                        return (
                          <div key={slot} className="flex gap-2">
                            <Field label={`Attribut ${slot + 1}`}>
                              <select
                                value={step[typeKey]}
                                onChange={(e) => updateStep(index, { [typeKey]: e.target.value })}
                                className="w-56 rounded-md border border-border bg-background px-2 py-1 text-sm"
                              >
                                {APPLY_TYPES.map((t) => (
                                  <option key={t.value} value={t.value}>
                                    {t.label}
                                  </option>
                                ))}
                              </select>
                            </Field>
                            <Field label="Wert">
                              <input
                                type="number"
                                value={step[valueKey]}
                                onChange={(e) => updateStep(index, { [valueKey]: e.target.value })}
                                className="w-24 rounded-md border border-border bg-background px-2 py-1 text-sm"
                              />
                            </Field>
                          </div>
                        );
                      })}
                    </div>
                  </div>
                ))}
                <p className="text-xs text-muted-foreground">
                  Jeder Schritt wird ein eigener Quest-Zustand - der Spieler kommt automatisch beim
                  nächsten Schritt an, sobald der vorherige abgeschlossen ist (set_state im
                  Hintergrund, kein Zutun des Spielers nötig). Beim letzten Schritt wird die
                  Zwischenbelohnung zur eigentlichen Abschlussbelohnung.
                </p>
              </div>
            )}

            {createError && <p className="text-sm text-destructive">{createError}</p>}
            <Button
              onClick={submitCreate}
              disabled={!newCategoryPreview || !newNamePreview || creatingBusy}
            >
              <Wand2 className="size-4" />
              {creatingBusy ? "Lege an…" : "Anlegen"}
            </Button>
          </div>

          <div className="flex min-w-0 flex-1 flex-col gap-1">
            <span className="text-xs font-medium text-muted-foreground">
              Vorschau (nach dem Anlegen als Code editierbar)
            </span>
            <pre className="min-h-0 flex-1 overflow-auto rounded-md border border-border bg-muted/30 p-2 font-mono text-xs">
              {preview}
            </pre>
          </div>
        </div>
      </Modal>

      {/* NPC/Mob/Item-Picker */}
      {picker && (
        <Modal onClose={() => setPicker(null)}>
          <EntityBrowser
            kind={picker.kind === "item" ? "item" : "mob"}
            pickLabel="Übernehmen"
            autoFocus
            maxHeightClass="max-h-64"
            initialQuery={picker.initialQuery}
            onPick={(r) => pickValue(r.vnum)}
          />
        </Modal>
      )}
    </>
  );
}
