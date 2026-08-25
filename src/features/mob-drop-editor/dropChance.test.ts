import { describe, expect, it } from "vitest";
import { formatRealDropChance, realDropChancePercent, simulateDrops } from "./dropChance";

describe("realDropChancePercent", () => {
  it("matches the derivation from the server source (percent / 4)", () => {
    expect(realDropChancePercent(10)).toBe(2.5);
    expect(realDropChancePercent(1)).toBe(0.25);
    expect(realDropChancePercent(100)).toBe(25);
  });
});

describe("formatRealDropChance", () => {
  it("formats without a misleading extra factor of 100", () => {
    // Regression test: an earlier version divided by 400 and appended "%"
    // directly, showing "0.03%" for value 10 instead of the correct 2.5%.
    expect(formatRealDropChance(10)).toBe("2.5%");
    expect(formatRealDropChance(1)).toBe("0.25%");
  });

  it("trims trailing zeros", () => {
    expect(formatRealDropChance(4)).toBe("1%");
  });
});

describe("simulateDrops", () => {
  it("uses the same real-chance formula as the rest of the editor (percent / 4, as a fraction)", () => {
    // percent=4 -> realDropChancePercent=1% -> p=0.01 per kill
    const [result] = simulateDrops([{ item_vnum: 1, percent: 4 }], 1);
    expect(result.expectedCount).toBeCloseTo(0.01, 10);
  });

  it("computes P(at least one) as 1 - (1-p)^N", () => {
    const [result] = simulateDrops([{ item_vnum: 1, percent: 4 }], 100);
    // p = 0.01, N = 100 -> 1 - 0.99^100 ≈ 63.397%
    expect(result.chanceAtLeastOne).toBeCloseTo(63.397, 2);
    expect(result.expectedCount).toBeCloseTo(1, 10);
  });

  it("returns 0 for both fields at 0 kills, and clamps negative/non-finite kill counts to 0", () => {
    for (const kills of [0, -5, NaN]) {
      const [result] = simulateDrops([{ item_vnum: 1, percent: 4 }], kills);
      expect(result.chanceAtLeastOne).toBe(0);
      expect(result.expectedCount).toBe(0);
    }
  });

  it("caps the per-kill probability at 100% for percent values at/above the file max (400)", () => {
    const [result] = simulateDrops([{ item_vnum: 1, percent: 400 }], 1);
    expect(result.chanceAtLeastOne).toBeCloseTo(100, 10);
    expect(result.expectedCount).toBeCloseTo(1, 10);
  });

  it("handles multiple items independently", () => {
    const results = simulateDrops(
      [
        { item_vnum: 1, percent: 4 },
        { item_vnum: 2, percent: 40 },
      ],
      100,
    );
    expect(results).toHaveLength(2);
    expect(results[0].item_vnum).toBe(1);
    expect(results[1].item_vnum).toBe(2);
    expect(results[1].expectedCount).toBeGreaterThan(results[0].expectedCount);
  });
});
