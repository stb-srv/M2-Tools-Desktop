import { describe, expect, it } from "vitest";
import {
  buildDraftSteps,
  classifyClause,
  pickBestMatch,
  resolveDescription,
  splitIntoClauses,
  type NameCandidate,
  type NameLookup,
} from "./questDescriptionParser";

describe("splitIntoClauses", () => {
  it("splits on hard punctuation (period, newline, semicolon)", () => {
    expect(splitIntoClauses("Rede mit Hans. Sammle Items\nTöte den Boss; fertig")).toEqual([
      "Rede mit Hans",
      "Sammle Items",
      "Töte den Boss",
      "fertig",
    ]);
  });

  it("additionally splits on embedded dann/danach/anschließend connectors", () => {
    expect(splitIntoClauses("Sammle 10 Wolfsfelle, dann töte 5 Wölfe, danach rede mit Hans")).toEqual([
      "Sammle 10 Wolfsfelle",
      "töte 5 Wölfe",
      "rede mit Hans",
    ]);
  });

  it("splits on a bare comma when the next part starts a recognizable new action (no 'dann' needed)", () => {
    // Regression: live-tested against the exact example shown in the UI
    // placeholder - a bare comma between "Rede mit Hans" and "sammle 10
    // Wolfsfelle" must still split, since real users won't repeat "dann"
    // before every action in a list.
    expect(splitIntoClauses("Rede mit Hans, sammle 10 Wolfsfelle, dann bekommt man 100 Yang.")).toEqual([
      "Rede mit Hans",
      "sammle 10 Wolfsfelle",
      "bekommt man 100 Yang",
    ]);
  });

  it("does not split on a bare comma without a connector word", () => {
    expect(splitIntoClauses("Rede mit Hans, dem Schmied")).toEqual(["Rede mit Hans, dem Schmied"]);
  });
});

describe("classifyClause", () => {
  it("recognizes a dialog clause and extracts the NPC name", () => {
    const result = classifyClause("Rede mit Hans");
    expect(result.kind).toBe("dialog");
    expect(result.recognized).toBe(true);
    expect(result.npcPhrase).toBe("Hans");
  });

  it("recognizes a collect clause with quantity and item phrase", () => {
    const result = classifyClause("Sammle 10 Wolfsfelle");
    expect(result.kind).toBe("collect");
    expect(result.quantity).toBe(10);
    expect(result.targetPhrase).toBe("Wolfsfelle");
    expect(result.npcPhrase).toBeNull();
  });

  it("recognizes a kill clause with quantity and mob phrase", () => {
    const result = classifyClause("töte 5 Wölfe");
    expect(result.kind).toBe("kill");
    expect(result.quantity).toBe(5);
    expect(result.targetPhrase).toBe("Wölfe");
  });

  it("recognizes a use clause without needing an NPC", () => {
    const result = classifyClause("benutze den Trank");
    expect(result.kind).toBe("use");
    expect(result.targetPhrase).toBe("Trank");
    expect(result.npcPhrase).toBeNull();
  });

  it("extracts an explicit NPC via 'bei <Name>' in a collect clause", () => {
    const result = classifyClause("gib die Felle bei Hans ab");
    expect(result.kind).toBe("collect");
    expect(result.npcPhrase).toBe("Hans");
    expect(result.targetPhrase).toBe("Felle");
  });

  it("falls back to a raw dialog step when no known verb is recognized", () => {
    const result = classifyClause("Das ist eine seltsame Formulierung ohne Verb");
    expect(result.kind).toBe("dialog");
    expect(result.recognized).toBe(false);
    expect(result.isRewardOnly).toBe(false);
    expect(result.rawText).toBe("Das ist eine seltsame Formulierung ohne Verb");
  });

  it("classifies a reward-only clause (money + item) without a recognized action verb", () => {
    const result = classifyClause("Man bekommt 100 Yang und ein Schwert");
    expect(result.recognized).toBe(false);
    expect(result.isRewardOnly).toBe(true);
    expect(result.rewardMoney).toBe(100);
    expect(result.rewardItemPhrase).toBe("Schwert");
  });

  it("attaches reward info directly when the same clause also has an action verb", () => {
    const result = classifyClause("töte den Boss und bekomme dafür 100 Yang");
    expect(result.kind).toBe("kill");
    expect(result.recognized).toBe(true);
    expect(result.isRewardOnly).toBe(false);
    expect(result.rewardMoney).toBe(100);
  });
});

describe("buildDraftSteps", () => {
  it("merges a trailing reward-only clause into the previous step instead of creating a new one", () => {
    const { drafts, notes } = buildDraftSteps(
      "Rede mit Hans. Sammle 10 Wolfsfelle, dann töte 5 Wölfe. Man bekommt 100 Yang und ein Schwert.",
    );
    expect(drafts).toHaveLength(3);
    const lastDraft = drafts[drafts.length - 1];
    expect(lastDraft.kind).toBe("kill");
    expect(lastDraft.rewardMoney).toBe(100);
    expect(lastDraft.rewardItemPhrase).toBe("Schwert");
    expect(notes.some((n) => n.includes("keinem Schritt zugeordnet"))).toBe(false);
  });

  it("carries the last known NPC forward to steps that don't name one explicitly", () => {
    const { drafts } = buildDraftSteps("Rede mit Hans. Sammle 10 Wolfsfelle.");
    expect(drafts[0].npcPhrase).toBe("Hans");
    expect(drafts[1].npcPhrase).toBe("Hans");
  });

  it("does not require an NPC for use steps", () => {
    const { drafts, notes } = buildDraftSteps("benutze den Trank");
    expect(drafts[0].npcPhrase).toBeNull();
    expect(notes.some((n) => n.includes("kein NPC erkannt"))).toBe(false);
  });

  it("flags a step with no NPC found anywhere in the description", () => {
    const { notes } = buildDraftSteps("Sammle 10 Wolfsfelle.");
    expect(notes.some((n) => n.includes("kein NPC erkannt"))).toBe(true);
  });

  it("flags a step with no recognized quantity", () => {
    const { notes } = buildDraftSteps("Rede mit Hans. Sammle Wolfsfelle.");
    expect(notes.some((n) => n.includes("keine Anzahl erkannt"))).toBe(true);
  });

  it("notes an unresolvable trailing reward when there is no previous step at all", () => {
    const { drafts, notes } = buildDraftSteps("Man bekommt 100 Yang.");
    expect(drafts).toHaveLength(0);
    expect(notes.some((n) => n.includes("keinem Schritt zugeordnet"))).toBe(true);
  });
});

describe("pickBestMatch", () => {
  const candidates: NameCandidate[] = [
    { vnum: 1, name: "Wolfsfell" },
    { vnum: 2, name: "Wolfsfellstück" },
    { vnum: 3, name: "Zerrissenes Wolfsfell" },
  ];

  it("prefers an exact match", () => {
    expect(pickBestMatch("wolfsfell", candidates)).toEqual({ vnum: 1, name: "Wolfsfell" });
  });

  it("falls back to the shortest substring match when nothing is exact", () => {
    const noExact: NameCandidate[] = [
      { vnum: 2, name: "Wolfsfellstück" },
      { vnum: 3, name: "Zerrissenes Wolfsfell" },
    ];
    expect(pickBestMatch("wolfsfell", noExact)).toEqual({ vnum: 2, name: "Wolfsfellstück" });
  });

  it("returns null when nothing matches at all", () => {
    expect(pickBestMatch("drachenschuppe", candidates)).toBeNull();
  });

  it("returns null when two candidates are equally ambiguous (same-length substring matches)", () => {
    const tie: NameCandidate[] = [
      { vnum: 4, name: "AB Fell" },
      { vnum: 5, name: "CD Fell" },
    ];
    expect(pickBestMatch("fell", tie)).toBeNull();
  });

  it("returns null for an empty query", () => {
    expect(pickBestMatch("", candidates)).toBeNull();
  });
});

describe("resolveDescription", () => {
  it("resolves names via the injected lookup and fills the matching QuestStep fields", async () => {
    const lookup = async (phrase: string, kind: "npc" | "item" | "mob") => {
      if (kind === "npc" && phrase === "Hans") return { vnum: 9001, name: "Hans" };
      if (kind === "item" && phrase === "Wolfsfelle") return { vnum: 500, name: "Wolfsfell" };
      return null;
    };

    const { steps, notes } = await resolveDescription("Rede mit Hans. Sammle 10 Wolfsfelle.", lookup);

    expect(steps).toHaveLength(2);
    expect(steps[0].kind).toBe("dialog");
    expect(steps[0].npcVnum).toBe("9001");
    expect(steps[1].kind).toBe("collect");
    expect(steps[1].npcVnum).toBe("9001");
    expect(steps[1].requiredItemVnum).toBe("500");
    expect(steps[1].requiredItemCount).toBe("10");
    expect(notes).toEqual([]);
  });

  it("leaves a field empty and adds a note when the lookup can't resolve a name", async () => {
    const lookup = async () => null;
    const { steps, notes } = await resolveDescription("Sammle 10 Wolfsfelle bei Hans.", lookup);

    expect(steps[0].npcVnum).toBe("");
    expect(steps[0].requiredItemVnum).toBe("");
    expect(notes.some((n) => n.includes('NPC "Hans"'))).toBe(true);
    expect(notes.some((n) => n.includes('Item "Wolfsfelle"'))).toBe(true);
  });

  it("returns a structured unresolved entry pointing at the exact field and query that failed", async () => {
    const lookup: NameLookup = async () => null;
    const { unresolved } = await resolveDescription("Sammle 10 Wolfsfelle bei Hans.", lookup);

    expect(unresolved).toContainEqual({ stepIndex: 0, field: "npcVnum", kind: "npc", query: "Hans" });
    expect(unresolved).toContainEqual({
      stepIndex: 0,
      field: "requiredItemVnum",
      kind: "item",
      query: "Wolfsfelle",
    });
  });

  it("uses the useItemVnum field (not requiredItemVnum) for an unresolved use-step item", async () => {
    const lookup: NameLookup = async () => null;
    const { unresolved } = await resolveDescription("benutze den Trank", lookup);

    expect(unresolved).toContainEqual({ stepIndex: 0, field: "useItemVnum", kind: "item", query: "Trank" });
  });

  it("carries repeatable/cooldownDays through into the resolved result", async () => {
    const lookup: NameLookup = async () => null;
    const { repeatable, cooldownDays } = await resolveDescription(
      "Rede mit Hans, alle 4 Tage wiederholbar.",
      lookup,
    );
    expect(repeatable).toBe(true);
    expect(cooldownDays).toBe(4);
  });
});

describe("REWARD_MARKERS coverage (via classifyClause's reward detection)", () => {
  const cases = [
    "Man erhalte 100 Yang",
    "Man erhältst 100 Yang",
    "Man erhaltet 100 Yang",
    "Man bekomme 100 Yang",
    "Man bekommst 100 Yang",
    "Man kriege 100 Yang",
    "Man kriegst 100 Yang",
    "Man kriegt 100 Yang",
  ];
  for (const clause of cases) {
    it(`recognizes "${clause}" as a reward-only clause`, () => {
      const result = classifyClause(clause);
      expect(result.isRewardOnly).toBe(true);
      expect(result.rewardMoney).toBe(100);
    });
  }
});

describe("repeatable-quest detection ('alle N Tage')", () => {
  it("splits an 'alle N Tage' remark into its own clause even without 'dann'", () => {
    expect(splitIntoClauses("Sammle 10 Felle, du kannst das alle 4 Tage machen")).toEqual([
      "Sammle 10 Felle",
      "du kannst das alle 4 Tage machen",
    ]);
  });

  it("extracts repeatable/cooldownDays and does not turn the clause into a step", () => {
    const { drafts, repeatable, cooldownDays } = buildDraftSteps(
      "Rede mit Hans. Du kannst diese Quest alle 4 Tage machen.",
    );
    expect(drafts).toHaveLength(1);
    expect(repeatable).toBe(true);
    expect(cooldownDays).toBe(4);
  });

  it("recognizes 'Tagen' (dative plural) as well as 'Tage'", () => {
    const { repeatable, cooldownDays } = buildDraftSteps("Wiederholbar alle 7 Tagen.");
    expect(repeatable).toBe(true);
    expect(cooldownDays).toBe(7);
  });
});

describe("real-world phrasing: 'bei <NPC>' cleanly separates NPC from item+quantity", () => {
  it("extracts quantity, item, and NPC all correctly when the sentence uses 'bei <NPC> ab'", () => {
    const { drafts } = buildDraftSteps("gib 20 Orkzähne bei Gemi ab, dann bekommst du 100 Yang");
    expect(drafts).toHaveLength(1);
    expect(drafts[0].kind).toBe("collect");
    expect(drafts[0].quantity).toBe(20);
    expect(drafts[0].targetPhrase).toBe("Orkzähne");
    expect(drafts[0].npcPhrase).toBe("Gemi");
    expect(drafts[0].rewardMoney).toBe(100);
  });
});

describe("regression: the exact user bug report sentence", () => {
  // "Rede mit dem Gemi, bringe der Gemi, 20 Orkzähne und erhalte dafür
  // 250HP zusätzlich dauerhaft, du kannst diese quest alle 4 Tage machen"
  // - live-reported: the NPC/item search failed because the whole rest of
  // the sentence got glued onto the item search. This locks in what the
  // fix actually achieves: clean clause boundaries and correct
  // repeatable-detection. It does NOT claim "Orkzähne" now resolves
  // perfectly - splitting the indirect object ("der Gemi") from the
  // direct object ("20 Orkzähne") across a bare comma is a genuine
  // grammar-level ambiguity no keyword matcher fully resolves; the
  // "Suchen" fallback (QuestBuilder.tsx) is the safety net for exactly
  // this case.
  const SENTENCE =
    "Rede mit dem Gemi, bringe der Gemi, 20 Orkzähne und erhalte dafür 250HP zusätzlich dauerhaft, du kannst diese quest alle 4 Tage machen";

  it("splits into clean clauses instead of gluing the whole tail onto one clause", () => {
    const clauses = splitIntoClauses(SENTENCE);
    expect(clauses).toEqual([
      "Rede mit dem Gemi",
      "bringe der Gemi",
      "20 Orkzähne und erhalte dafür 250HP zusätzlich dauerhaft",
      "du kannst diese quest alle 4 Tage machen",
    ]);
  });

  it("correctly extracts repeatable/cooldownDays instead of treating it as noise", () => {
    const { repeatable, cooldownDays, drafts } = buildDraftSteps(SENTENCE);
    expect(repeatable).toBe(true);
    expect(cooldownDays).toBe(4);
    // the repeatable clause must not itself become a bogus 3rd step
    expect(drafts).toHaveLength(2);
  });

  it("resolves the first step's NPC correctly via the fuzzy lookup", async () => {
    const lookup: NameLookup = async (phrase, kind) => {
      if (kind === "npc" && phrase === "Gemi") return { vnum: 9042, name: "Gemischwarenhändlerin" };
      return null;
    };
    const { steps } = await resolveDescription(SENTENCE, lookup);
    expect(steps[0].kind).toBe("dialog");
    expect(steps[0].npcVnum).toBe("9042");
  });
});
