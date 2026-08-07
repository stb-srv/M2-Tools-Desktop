import { describe, expect, it } from "vitest";
import {
  DEFAULT_DUNGEON_FORM,
  DEFAULT_FLOOR,
  DEFAULT_FORM,
  generateDungeonQuest,
  generateQuestLua,
  type DungeonFormState,
  type QuestFormState,
} from "./questTemplates";

// Every `begin` and every plain `if` (no elseif/while/for/function used in
// these templates) needs exactly one matching `end`, or the server's own
// qc_x64 compiler will reject the file outright - a cheap structural smoke
// test that catches indentation/template bugs without a real Lua parser.
function countKeyword(text: string, keyword: string): number {
  return (text.match(new RegExp(`\\b${keyword}\\b`, "g")) ?? []).length;
}

function expectBalanced(lua: string) {
  const opens = countKeyword(lua, "begin") + countKeyword(lua, "if");
  const ends = countKeyword(lua, "end");
  expect(ends).toBe(opens);
}

describe("generateQuestLua", () => {
  const withNpc = (patch: Partial<QuestFormState>): QuestFormState => ({
    ...DEFAULT_FORM,
    npcVnum: "20349",
    npcTitle: "Testwirt",
    ...patch,
  });

  it("wraps every template in exactly one quest/state block", () => {
    const lua = generateQuestLua("test_quest", "dialog", withNpc({ dialogText: "Hallo" }));
    expect(lua).toMatch(/^quest test_quest begin/);
    expect(lua).toContain("state start begin");
    expectBalanced(lua);
  });

  it("dialog: says the configured text and gives the reward", () => {
    const lua = generateQuestLua(
      "dialog_quest",
      "dialog",
      withNpc({ dialogText: "Willkommen!", rewardItemVnum: "40", rewardItemCount: "3" }),
    );
    expect(lua).toContain('when 20349.chat."Quest" begin');
    expect(lua).toContain('say("Willkommen!")');
    expect(lua).toContain("pc.give_item2(40, 3)");
    expectBalanced(lua);
  });

  it("collect: gates the reward behind pc.count_item and removes the item", () => {
    const lua = generateQuestLua(
      "collect_quest",
      "collect",
      withNpc({ requiredItemVnum: "100", requiredItemCount: "5" }),
    );
    expect(lua).toContain("if pc.count_item(100) >= 5 then");
    expect(lua).toContain("pc.remove_item(100, 5)");
    expectBalanced(lua);
  });

  it("chance_collect: always consumes the item, only counts on a successful roll", () => {
    const lua = generateQuestLua(
      "bio_quest",
      "chance_collect",
      withNpc({
        requiredItemVnum: "200",
        itemsPerAttempt: "2",
        acceptChancePercent: "60",
        requiredSuccesses: "9",
      }),
    );
    // consumption happens unconditionally, before the roll - matches
    // Biologie/Orkzahn.lua's real pc.remove_item-then-number(1,100) order
    const removeIdx = lua.indexOf("pc.remove_item(200, 2)");
    const rollIdx = lua.indexOf("local roll = number(1, 100)");
    expect(removeIdx).toBeGreaterThan(-1);
    expect(rollIdx).toBeGreaterThan(removeIdx);
    expect(lua).toContain("if roll <= 60 then");
    expect(lua).toContain('if pc.getqf("collect_count") >= 9 then');
    expectBalanced(lua);
  });

  it("kill: tracks progress via pc.getqf/setqf keyed on the mob vnum", () => {
    const lua = generateQuestLua(
      "kill_quest",
      "kill",
      withNpc({ mobVnum: "101", requiredKills: "5" }),
    );
    expect(lua).toContain("when 101.kill begin");
    expect(lua).toContain('pc.getqf("kill_count")');
    expectBalanced(lua);
  });

  it("use: triggers on ITEM.use without needing an NPC", () => {
    const lua = generateQuestLua(
      "use_quest",
      "use",
      withNpc({ useItemVnum: "300", useText: "Bumm!" }),
    );
    expect(lua).toContain("when 300.use begin");
    expect(lua).toContain('say("Bumm!")');
    expectBalanced(lua);
  });

  it("buffed_item: gives the item via give_item2_select and force-sets only the filled attribute slots", () => {
    const lua = generateQuestLua(
      "buffed_quest",
      "buffed_item",
      withNpc({
        buffedItemVnum: "800009",
        buffedItemCount: "1",
        buffedAttrType0: "4", // INT
        buffedAttrValue0: "500",
        buffedAttrType1: "5", // STR
        buffedAttrValue1: "700",
        // slots 2/3 left at DEFAULT_FORM's "0" (Keine) - must not appear
      }),
    );
    expect(lua).toContain("pc.give_item2_select(800009, 1)");
    expect(lua).toContain("item.set_value(0, 4, 500)");
    expect(lua).toContain("item.set_value(1, 5, 700)");
    expect(lua).not.toContain("item.set_value(2,");
    expect(lua).not.toContain("item.set_value(3,");
    expectBalanced(lua);
  });

  it("escapes quotes and turns real newlines into [ENTER]", () => {
    const lua = generateQuestLua(
      "escape_quest",
      "dialog",
      withNpc({ dialogText: 'Er sagt "Hallo"\nund geht.' }),
    );
    expect(lua).toContain('say("Er sagt \\"Hallo\\"[ENTER]und geht.")');
  });
});

describe("generateDungeonQuest", () => {
  const twoFloorForm: DungeonFormState = {
    ...DEFAULT_DUNGEON_FORM,
    entryNpcVnum: "9001",
    dungeonMapIndex: "150",
    mapIndexRangeStart: "660000",
    mapIndexRangeEnd: "670000",
    floors: [
      { ...DEFAULT_FLOOR, bossVnum: "8031", regenFile: "data/dungeon/x/floor1.txt", entryX: "100", entryY: "100" },
      { ...DEFAULT_FLOOR, bossVnum: "8032", regenFile: "data/dungeon/x/floor2.txt", entryX: "200", entryY: "200" },
    ],
    rewardItemVnum: "50",
    rewardItemCount: "1",
  };

  it("is structurally balanced across every floor", () => {
    const lua = generateDungeonQuest("test_run", twoFloorForm);
    expectBalanced(lua);
  });

  it("only the last floor's boss exits the instance", () => {
    const lua = generateDungeonQuest("test_run", twoFloorForm);
    expect(lua).toContain("when 8031.kill with pc.in_dungeon()");
    expect(lua).toContain("when 8032.kill with pc.in_dungeon()");
    // intermediate floor advances state and loads the next regen file...
    expect(lua).toContain('d.regen_file("data/dungeon/x/floor2.txt")');
    expect(lua).toContain("d.jump_all(200, 200)");
    // ...only the final floor schedules the exit timer
    expect(lua).toContain("when test_run_exit.server_timer begin");
    expect(lua.match(/d\.exit_all\(\)/g)?.length).toBe(1);
  });

  it("respects the configured map-index range in every boss guard", () => {
    const lua = generateDungeonQuest("test_run", twoFloorForm);
    const guardCount = (lua.match(/pc\.get_map_index\(\) >= 660000 and pc\.get_map_index\(\) < 670000/g) ?? [])
      .length;
    expect(guardCount).toBeGreaterThanOrEqual(2);
  });

  it("falls back to a single default floor when the list is empty", () => {
    const lua = generateDungeonQuest("empty_run", { ...twoFloorForm, floors: [] });
    expectBalanced(lua);
    expect(lua).toContain("when 0.kill with pc.in_dungeon()");
  });
});
