import { describe, expect, it } from "vitest";
import { findDuplicateItemsInMobs, findDuplicateMobs } from "./duplicates";
import type { MobDropGroup } from "./types";

function group(overrides: Partial<MobDropGroup>): MobDropGroup {
  return { name: "Mob", mob_vnum: 101, drop_type: "drop", items: [], ...overrides };
}

describe("findDuplicateItemsInMobs", () => {
  it("finds nothing when every mob's items are unique", () => {
    const groups = [
      group({ items: [{ item_vnum: 1, count: 1, percent: 10 }, { item_vnum: 2, count: 1, percent: 5 }] }),
    ];
    expect(findDuplicateItemsInMobs(groups)).toEqual([]);
  });

  it("finds an item vnum entered twice within one mob", () => {
    const groups = [
      group({
        name: "Wolf",
        mob_vnum: 101,
        items: [
          { item_vnum: 42, count: 1, percent: 10 },
          { item_vnum: 99, count: 1, percent: 5 },
          { item_vnum: 42, count: 1, percent: 20 },
        ],
      }),
    ];
    expect(findDuplicateItemsInMobs(groups)).toEqual([
      { groupIndex: 0, mobName: "Wolf", mobVnum: 101, itemVnum: 42, itemIndices: [0, 2] },
    ]);
  });

  it("collects all positions when the same vnum appears three times", () => {
    const groups = [
      group({
        items: [
          { item_vnum: 5, count: 1, percent: 1 },
          { item_vnum: 5, count: 1, percent: 2 },
          { item_vnum: 5, count: 1, percent: 3 },
        ],
      }),
    ];
    expect(findDuplicateItemsInMobs(groups)[0].itemIndices).toEqual([0, 1, 2]);
  });

  it("keeps duplicates scoped per mob, not across the whole file", () => {
    const groups = [
      group({ mob_vnum: 101, items: [{ item_vnum: 7, count: 1, percent: 1 }] }),
      group({ mob_vnum: 102, items: [{ item_vnum: 7, count: 1, percent: 1 }] }),
    ];
    expect(findDuplicateItemsInMobs(groups)).toEqual([]);
  });
});

describe("findDuplicateMobs", () => {
  it("finds nothing when every mob_vnum is unique", () => {
    const groups = [group({ mob_vnum: 101 }), group({ mob_vnum: 102 })];
    expect(findDuplicateMobs(groups)).toEqual([]);
  });

  it("finds a mob_vnum that appears as more than one group entry", () => {
    const groups = [group({ mob_vnum: 101 }), group({ mob_vnum: 102 }), group({ mob_vnum: 101 })];
    expect(findDuplicateMobs(groups)).toEqual([{ mobVnum: 101, groupIndices: [0, 2] }]);
  });
});

describe("both checks together", () => {
  it("let a single mob show up in both findings independently", () => {
    const groups = [
      group({
        mob_vnum: 101,
        items: [
          { item_vnum: 1, count: 1, percent: 1 },
          { item_vnum: 1, count: 1, percent: 1 },
        ],
      }),
      group({ mob_vnum: 101, items: [] }),
    ];
    expect(findDuplicateItemsInMobs(groups)).toEqual([
      { groupIndex: 0, mobName: "Mob", mobVnum: 101, itemVnum: 1, itemIndices: [0, 1] },
    ]);
    expect(findDuplicateMobs(groups)).toEqual([{ mobVnum: 101, groupIndices: [0, 1] }]);
  });
});
