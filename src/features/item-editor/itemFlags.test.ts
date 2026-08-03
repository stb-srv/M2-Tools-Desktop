import { describe, expect, it } from "vitest";
import {
  WEAPON_SUBTYPE_TWO_HANDED,
  weaponDisplayDamage,
  weaponAttackSpeedLabel,
  weaponEffectiveAttackSpeed,
} from "./itemFlags";

describe("weaponDisplayDamage", () => {
  it("adds value5 (Zusatz-Angriffskraft) to both min and max", () => {
    expect(weaponDisplayDamage(10, 20, 5)).toEqual({ min: 15, max: 25 });
  });

  it("matches vnum 3210's real DB values (value3=10 value4=20 value5=0)", () => {
    expect(weaponDisplayDamage(10, 20, 0)).toEqual({ min: 10, max: 20 });
  });
});

describe("weaponAttackSpeedLabel", () => {
  it.each([
    [79, "Sehr schnell"],
    [95, "Schnell"],
    [105, "Normal"],
    [120, "Langsam"],
    [121, "Sehr langsam"],
  ])("value0=%i -> %s", (value0, label) => {
    expect(weaponAttackSpeedLabel(value0)).toBe(label);
  });
});

describe("weaponEffectiveAttackSpeed", () => {
  it("leaves one-handed weapons unchanged", () => {
    expect(weaponEffectiveAttackSpeed(120, 0)).toBe(120);
  });

  it("subtracts 10 for two-handed weapons (verified vnum 3210: 120 db -> 110 in-game)", () => {
    expect(weaponEffectiveAttackSpeed(120, WEAPON_SUBTYPE_TWO_HANDED)).toBe(110);
  });
});
