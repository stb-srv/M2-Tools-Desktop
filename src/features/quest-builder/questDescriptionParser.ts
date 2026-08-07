// "Semi-KI"-Freitext-Assistent für den Mehrschritt-Baukasten: reine
// Mustererkennung (Schlüsselwörter/Regex + Fuzzy-Abgleich gegen die echte
// Item-/Mob-/NPC-Datenbank), KEINE echte KI/Sprachmodell-Anbindung. Erkennt
// aus einer Freitext-Beschreibung einzelne Schritte (dialog/collect/kill/
// use - dieselben 4 StepKind-Werte wie im manuellen Baukasten) und füllt
// damit exakt dieselbe MultiStepFormState-Struktur, die der Nutzer danach
// wie gewohnt im bestehenden Baukasten prüft/bearbeitet, bevor irgendetwas
// als Lua erzeugt wird - nichts wird stillschweigend übernommen.
//
// Zwei Schichten (Vorbild: quest.rs pure+getestet, commands.rs dünner
// Tauri-Wrapper): `buildDraftSteps` ist reine Text-Verarbeitung ohne DB-
// Zugriff, vollständig synchron testbar. `resolveDescription` orchestriert
// die Namensauflösung über eine injizierte `NameLookup`-Funktion (in
// QuestBuilder.tsx ein dünner Wrapper um die bereits vorhandenen
// search_items/search_mobs-Befehle) - dadurch auch die Auflösung mit einer
// gefakten Lookup-Funktion testbar, ohne echten Tauri/DB-Zugriff.

import { DEFAULT_STEP, type QuestStep, type StepKind } from "./questTemplates";

const KILL_VERBS = /\b(töte|tote|besiege|besiegt|erledige|erledigt|erlege|erlegt|jage|jagt)\b/i;
const COLLECT_VERBS = /\b(sammle|sammel|sammelt|gib|abgeben|bring|bringe|bringt|liefere|liefert)\b/i;
const USE_VERBS = /\b(benutze|benutzt|benutz|verwende|verwendet|nutze|nutzt)\b/i;
const DIALOG_VERBS = /\b(rede|redet|sprich|sprechen|unterhalte|unterhalt|besuche|besuch)\b/i;

const REWARD_MARKERS =
  /\b(belohnung|bekomme|bekommst|bekommt|bekommen|erhalte|erhältst|erhaelst|erhält|erhaelt|erhaltet|erhalten|kriege|kriegst|kriegt|kriegen)\b/i;
const YANG_PATTERN = /(\d+)\s*yang/i;
const QUANTITY_PATTERN = /\b(\d+)\b(?!\s*yang)/i;
const NPC_BEI_PATTERN = /\bbei\s+([A-ZÄÖÜ][\p{L}]*(?:\s+[A-ZÄÖÜ][\p{L}]*)*)/u;
// "alle 4 Tage" / "alle 4 Tagen" - wiederholbare Quest mit Cooldown, siehe
// MultiStepFormState.repeatable/cooldownDays in questTemplates.ts.
const REPEATABLE_PATTERN = /\balle\s+(\d+)\s+tage?n?\b/i;

const FILLER_WORDS = new Set([
  "mit",
  "den",
  "die",
  "das",
  "dem",
  "der",
  "einen",
  "eine",
  "ein",
  "x",
  "stück",
  "stücke",
  "mal",
  "ab",
  "für",
  "dafür",
  "und",
  "dann",
  "danach",
  "anschließend",
  "man",
  "es",
  "gibt",
]);

function stripFillers(text: string): string {
  return text
    .split(/\s+/)
    .filter((w) => w && !FILLER_WORDS.has(w.toLowerCase().replace(/[.,;:!?]+$/, "")))
    .join(" ")
    .trim();
}

function stripWord(text: string, match: string): string {
  return text.replace(match, " ");
}

const CONNECTOR_PREFIX = /^(?:und\s+)?(?:dann|danach|anschließend)\b\s*/i;

// Ob ein Komma-getrennter Textteil eine neue Klausel beginnt (statt nur
// ein Nebensatz/Zusatz zur vorherigen zu sein, z.B. "Hans, dem Schmied"):
// er zählt als neue Klausel, wenn er selbst (nach Entfernen eines
// führenden "dann"/"danach"/"anschließend") ein bekanntes Aktions- oder
// Belohnungs-Muster enthält - reine Heuristik, kein echtes Satzverständnis.
function looksLikeNewClause(fragment: string): boolean {
  const stripped = fragment.replace(CONNECTOR_PREFIX, "");
  return (
    KILL_VERBS.test(stripped) ||
    COLLECT_VERBS.test(stripped) ||
    USE_VERBS.test(stripped) ||
    DIALOG_VERBS.test(stripped) ||
    REWARD_MARKERS.test(stripped) ||
    YANG_PATTERN.test(stripped) ||
    REPEATABLE_PATTERN.test(stripped)
  );
}

export function splitIntoClauses(text: string): string[] {
  const hardSplit = text
    .split(/[.\n;]+/)
    .map((s) => s.trim())
    .filter(Boolean);

  const clauses: string[] = [];
  for (const sentence of hardSplit) {
    const commaParts = sentence
      .split(",")
      .map((s) => s.trim())
      .filter(Boolean);

    let current = "";
    for (const part of commaParts) {
      if (current === "") {
        current = part;
      } else if (looksLikeNewClause(part)) {
        clauses.push(current);
        current = part.replace(CONNECTOR_PREFIX, "").trim();
      } else {
        current = `${current}, ${part}`;
      }
    }
    if (current) clauses.push(current);
  }
  return clauses;
}

export interface RewardExtract {
  itemPhrase: string | null;
  money: number | null;
  hadMarker: boolean;
}

export function extractReward(clause: string): RewardExtract {
  const hasMarker = REWARD_MARKERS.test(clause);
  const yangMatch = clause.match(YANG_PATTERN);
  const money = yangMatch ? Number(yangMatch[1]) : null;

  if (!hasMarker && money === null) {
    return { itemPhrase: null, money: null, hadMarker: false };
  }

  let rest = clause;
  if (yangMatch) rest = stripWord(rest, yangMatch[0]);
  rest = rest.replace(REWARD_MARKERS, " ");
  rest = stripFillers(rest).replace(/[.,;:!?]+$/, "").trim();

  return { itemPhrase: rest.length > 0 ? rest : null, money, hadMarker: hasMarker };
}

export interface ClauseParse {
  kind: StepKind;
  recognized: boolean;
  isRewardOnly: boolean;
  rawText: string;
  quantity: number | null;
  targetPhrase: string | null;
  npcPhrase: string | null;
  rewardItemPhrase: string | null;
  rewardMoney: number | null;
}

function extractQuantity(clause: string): number | null {
  const m = clause.match(QUANTITY_PATTERN);
  return m ? Number(m[1]) : null;
}

function extractNpcPhrase(clause: string): { phrase: string | null; withoutNpc: string } {
  const m = clause.match(NPC_BEI_PATTERN);
  if (!m) return { phrase: null, withoutNpc: clause };
  return { phrase: m[1].trim(), withoutNpc: stripWord(clause, m[0]) };
}

export function classifyClause(clause: string): ClauseParse {
  const reward = extractReward(clause);

  let verbMatch: RegExpMatchArray | null;
  let kind: StepKind;

  if ((verbMatch = clause.match(KILL_VERBS))) {
    kind = "kill";
  } else if ((verbMatch = clause.match(COLLECT_VERBS))) {
    kind = "collect";
  } else if ((verbMatch = clause.match(USE_VERBS))) {
    kind = "use";
  } else if ((verbMatch = clause.match(DIALOG_VERBS))) {
    kind = "dialog";
  } else {
    // Kein bekanntes Aktions-Verb erkannt: entweder eine reine
    // Belohnungs-Klausel (wird vom Aufrufer in den vorherigen Schritt
    // gemergt) oder ein kompletter Fallback auf einen Dialog-Schritt mit
    // dem Rohtext, damit nichts verloren geht.
    return {
      kind: "dialog",
      recognized: false,
      isRewardOnly: reward.hadMarker || reward.money !== null,
      rawText: clause,
      quantity: null,
      targetPhrase: null,
      npcPhrase: null,
      rewardItemPhrase: reward.itemPhrase,
      rewardMoney: reward.money,
    };
  }

  const quantity = kind === "collect" || kind === "kill" ? extractQuantity(clause) : null;
  const { phrase: npcPhrase, withoutNpc } = extractNpcPhrase(clause);

  let rest = withoutNpc.replace(verbMatch[0], " ");
  if (quantity !== null) rest = rest.replace(String(quantity), " ");
  const targetPhrase = stripFillers(rest).replace(/[.,;:!?]+$/, "").trim() || null;

  return {
    kind,
    recognized: true,
    isRewardOnly: false,
    rawText: clause,
    quantity,
    targetPhrase: kind === "dialog" ? null : targetPhrase,
    npcPhrase: kind === "dialog" ? targetPhrase : npcPhrase,
    rewardItemPhrase: reward.itemPhrase,
    rewardMoney: reward.money,
  };
}

export interface DraftStepsResult {
  drafts: ClauseParse[];
  notes: string[];
  repeatable: boolean;
  cooldownDays: number | null;
}

export function buildDraftSteps(text: string): DraftStepsResult {
  const clauses = splitIntoClauses(text);
  const drafts: ClauseParse[] = [];
  const notes: string[] = [];
  let carriedNpcPhrase: string | null = null;
  let repeatable = false;
  let cooldownDays: number | null = null;

  clauses.forEach((clause) => {
    const repeatableMatch = clause.match(REPEATABLE_PATTERN);
    if (repeatableMatch) {
      repeatable = true;
      cooldownDays = Number(repeatableMatch[1]);
      return;
    }

    const parsed = classifyClause(clause);

    if (parsed.isRewardOnly) {
      const prev = drafts[drafts.length - 1];
      if (prev) {
        if (parsed.rewardItemPhrase) prev.rewardItemPhrase = parsed.rewardItemPhrase;
        if (parsed.rewardMoney !== null) prev.rewardMoney = parsed.rewardMoney;
      } else {
        notes.push(`Belohnung "${clause.trim()}" konnte keinem Schritt zugeordnet werden.`);
      }
      return;
    }

    if (!parsed.recognized) {
      notes.push(
        `Schritt ${drafts.length + 1}: keine bekannte Aktion erkannt - als Dialogtext übernommen, bitte prüfen.`,
      );
    }

    // "use"-Schritte brauchen keinen NPC (when ITEM.use braucht keinen
    // NPC-Trigger) - dialog/collect/kill dagegen schon; ein im Text nicht
    // erneut genannter NPC übernimmt automatisch den zuletzt erkannten
    // (derselbe NPC bleibt über mehrere Schritte hinweg Ansprechpartner,
    // bis ein neuer Name erkannt wird).
    if (parsed.kind !== "use") {
      if (!parsed.npcPhrase) parsed.npcPhrase = carriedNpcPhrase;
      else carriedNpcPhrase = parsed.npcPhrase;
      if (!parsed.npcPhrase) {
        notes.push(`Schritt ${drafts.length + 1}: kein NPC erkannt - bitte manuell auswählen.`);
      }
    }

    if ((parsed.kind === "collect" || parsed.kind === "kill") && parsed.quantity === null) {
      notes.push(`Schritt ${drafts.length + 1}: keine Anzahl erkannt, "1" übernommen - bitte prüfen.`);
    }

    drafts.push(parsed);
  });

  return { drafts, notes, repeatable, cooldownDays };
}

export interface NameCandidate {
  vnum: number;
  name: string;
}

// shop.rs::search_items/search_mobs sortieren nicht nach Relevanz (reines
// `LIKE %query%`, quellcode-verifiziert) - diese Bewertung übernimmt das
// hier: exakte Übereinstimmung > beginnt mit der Abfrage > eindeutig
// kürzester Teilstring-Treffer. Mehrdeutig oder kein Treffer -> null, das
// Feld bleibt leer statt falsch geraten zu werden.
export function pickBestMatch(query: string, candidates: NameCandidate[]): NameCandidate | null {
  if (candidates.length === 0) return null;
  const q = query.trim().toLowerCase();
  if (!q) return null;

  const exact = candidates.filter((c) => c.name.trim().toLowerCase() === q);
  if (exact.length === 1) return exact[0];
  if (exact.length > 1) return null;

  const prefix = candidates.filter((c) => c.name.trim().toLowerCase().startsWith(q));
  if (prefix.length === 1) return prefix[0];

  const substring = candidates.filter((c) => c.name.trim().toLowerCase().includes(q));
  const pool = prefix.length > 1 ? prefix : substring;
  if (pool.length === 0) return null;

  const sorted = [...pool].sort((a, b) => a.name.length - b.name.length);
  if (sorted.length > 1 && sorted[0].name.length === sorted[1].name.length) return null;
  return sorted[0];
}

export type LookupKind = "npc" | "item" | "mob";
export type NameLookup = (phrase: string, kind: LookupKind) => Promise<NameCandidate | null>;

// Welches QuestStep-Feld ein fehlgeschlagener Namens-Lookup betrifft -
// erlaubt der UI, direkt einen "Suchen"-Knopf für genau dieses Feld
// anzubieten (openPicker mit der erkannten Namensphrase vorausgefüllt),
// statt nur eine Textnotiz anzuzeigen.
export type UnresolvedField = "npcVnum" | "requiredItemVnum" | "useItemVnum" | "mobVnum" | "rewardItemVnum";

export interface UnresolvedLookup {
  stepIndex: number;
  field: UnresolvedField;
  kind: LookupKind;
  query: string;
}

export interface ResolveResult {
  steps: QuestStep[];
  notes: string[];
  unresolved: UnresolvedLookup[];
  repeatable: boolean;
  cooldownDays: number | null;
}

export async function resolveDescription(text: string, lookup: NameLookup): Promise<ResolveResult> {
  const { drafts, notes, repeatable, cooldownDays } = buildDraftSteps(text);
  const allNotes = [...notes];
  const unresolved: UnresolvedLookup[] = [];

  const steps = await Promise.all(
    drafts.map(async (draft, index) => {
      const step: QuestStep = { ...DEFAULT_STEP, kind: draft.kind };

      const resolveField = async (
        phrase: string,
        kind: LookupKind,
        field: UnresolvedField,
        label: string,
      ): Promise<NameCandidate | null> => {
        const match = await lookup(phrase, kind);
        if (match) return match;
        allNotes.push(`Schritt ${index + 1}: ${label} "${phrase}" nicht eindeutig gefunden.`);
        unresolved.push({ stepIndex: index, field, kind, query: phrase });
        return null;
      };

      if (draft.kind === "dialog") {
        step.dialogText = draft.rawText;
        if (draft.npcPhrase) {
          const match = await resolveField(draft.npcPhrase, "npc", "npcVnum", "NPC");
          if (match) step.npcVnum = String(match.vnum);
        }
      } else {
        if (draft.npcPhrase) {
          const match = await resolveField(draft.npcPhrase, "npc", "npcVnum", "NPC");
          if (match) step.npcVnum = String(match.vnum);
        }

        if (draft.kind === "collect") {
          step.requiredItemCount = String(draft.quantity ?? 1);
          if (draft.targetPhrase) {
            const match = await resolveField(draft.targetPhrase, "item", "requiredItemVnum", "Item");
            if (match) step.requiredItemVnum = String(match.vnum);
          }
        } else if (draft.kind === "use") {
          if (draft.targetPhrase) {
            const match = await resolveField(draft.targetPhrase, "item", "useItemVnum", "Item");
            if (match) step.useItemVnum = String(match.vnum);
          }
        } else if (draft.kind === "kill") {
          step.requiredKills = String(draft.quantity ?? 1);
          if (draft.targetPhrase) {
            const match = await resolveField(draft.targetPhrase, "mob", "mobVnum", "Monster");
            if (match) step.mobVnum = String(match.vnum);
          }
        }
      }

      if (draft.rewardMoney !== null) step.rewardMoney = String(draft.rewardMoney);
      if (draft.rewardItemPhrase) {
        const match = await resolveField(draft.rewardItemPhrase, "item", "rewardItemVnum", "Belohnungs-Item");
        if (match) step.rewardItemVnum = String(match.vnum);
      }

      return step;
    }),
  );

  return { steps, notes: allNotes, unresolved, repeatable, cooldownDays };
}
