import { describe, expect, it } from "vitest";
import { formatRealDropChance, realDropChancePercent } from "./dropChance";

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
