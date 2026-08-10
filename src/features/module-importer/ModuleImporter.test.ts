import { describe, expect, it } from "vitest";
import { fitToByteLimit, fitToByteLimitWithSuffix, ITEM_NAME_MAX_BYTES } from "./ModuleImporter";

describe("fitToByteLimitWithSuffix", () => {
  it("never truncates the suffix, even when the base name alone reaches the byte limit", () => {
    // Realer Nutzerbericht: eine Aufwertungs-Kette bis +16 wurde bis +9
    // korrekt benannt, aber +10 hieß plötzlich "+1", und jede weitere Stufe
    // ebenfalls - weil das alte `fitToByteLimit(base + suffix)` einfach das
    // letzte Zeichen der GESAMTEN Zeichenkette abgeschnitten hat, sobald der
    // Name insgesamt über 24 Byte lag (genau die letzte Ziffer der Stufe).
    const longBase = "Sehr Langer Waffenname XX"; // schon > 24 Byte für sich allein
    const names = Array.from({ length: 17 }, (_, level) =>
      fitToByteLimitWithSuffix(longBase, level > 0 ? ` +${level}` : ""),
    );

    // Jeder Name muss einzigartig sein - keine zwei Stufen dürfen kollidieren.
    expect(new Set(names).size).toBe(names.length);

    // Die Stufen-Suffixe selbst müssen immer vollständig erhalten bleiben.
    for (let level = 1; level <= 16; level++) {
      expect(names[level].endsWith(` +${level}`)).toBe(true);
    }
  });

  it("still respects the overall byte limit by shrinking the base, not the suffix", () => {
    const longBase = "Ein Extrem Langer Basis-Item-Name Der Weit Ueber Das Limit Hinausgeht";
    const result = fitToByteLimitWithSuffix(longBase, " +16");
    expect(new TextEncoder().encode(result).length).toBeLessThanOrEqual(ITEM_NAME_MAX_BYTES);
    expect(result.endsWith(" +16")).toBe(true);
  });

  it("matches plain fitToByteLimit when there is no suffix", () => {
    const base = "Kurzer Name";
    expect(fitToByteLimitWithSuffix(base, "")).toBe(fitToByteLimit(base));
  });
});
