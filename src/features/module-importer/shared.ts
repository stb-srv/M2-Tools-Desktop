import { invoke } from "@tauri-apps/api/core";

// Same shape as the local (unexported) ItemProtoInput in ItemEditor.tsx -
// duplicated rather than imported, matching that file's own convention of
// keeping this interface private to whoever builds create_item_proto
// payloads.
export interface ItemProtoInput {
  vnum: number;
  vnum_range: number;
  name: string;
  locale_name: string;
  type: number;
  subtype: number;
  weight: number;
  size: number;
  antiflag: number;
  flag: number;
  wearflag: number;
  immuneflag: number;
  gold: number;
  shop_buy_price: number;
  refined_vnum: number;
  refine_set: number;
  magic_pct: number;
  limittype0: number;
  limitvalue0: number;
  limittype1: number;
  limitvalue1: number;
  applytype0: number;
  applyvalue0: number;
  applytype1: number;
  applyvalue1: number;
  applytype2: number;
  applyvalue2: number;
  applytype3: number;
  applyvalue3: number;
  value0: number;
  value1: number;
  value2: number;
  value3: number;
  value4: number;
  value5: number;
  socket0: number;
  socket1: number;
  socket2: number;
  socket3: number;
  socket4: number;
  socket5: number;
  specular: number;
  socket_pct: number;
  addon_type: number;
}

export function emptyItem(vnum: number, type: number): ItemProtoInput {
  return {
    vnum,
    vnum_range: 0,
    name: "",
    locale_name: "",
    type,
    subtype: 0,
    weight: 0,
    size: 1,
    antiflag: 0,
    flag: 0,
    wearflag: 0,
    immuneflag: 0,
    gold: 0,
    shop_buy_price: 0,
    refined_vnum: 0,
    refine_set: 0,
    magic_pct: 0,
    limittype0: 0,
    limitvalue0: 0,
    limittype1: 0,
    limitvalue1: 0,
    applytype0: 0,
    applyvalue0: 0,
    applytype1: 0,
    applyvalue1: 0,
    applytype2: 0,
    applyvalue2: 0,
    applytype3: 0,
    applyvalue3: 0,
    value0: 0,
    value1: 0,
    value2: 0,
    value3: 0,
    value4: 0,
    value5: 0,
    socket0: 0,
    socket1: 0,
    socket2: 0,
    socket3: 0,
    socket4: 0,
    socket5: 0,
    specular: 0,
    socket_pct: 0,
    addon_type: 0,
  };
}

export interface ItemSearchResult {
  vnum: number;
  name: string;
}

// Mirrors modulescan.rs's Rust structs (Serialize) 1:1.
export interface WeaponVariant {
  key: string;
  label: string;
  subtype: number; // -1 = unrecognized, manual pick needed
  is_sura_model: boolean;
  model_source_abs: string;
  texture_sources_abs: string[];
  icon_source_abs: string | null;
}

export interface ArmorPiece {
  race: string; // warrior | assassin | sura | shaman
  label: string;
  male_model_abs: string | null;
  male_textures_abs: string[];
  female_model_abs: string | null;
  female_textures_abs: string[];
  icon_source_abs: string | null;
}

export interface ScannedModule {
  module_name: string;
  weapons: WeaponVariant[];
  armors: ArmorPiece[];
  icon_pool: string[];
  effect_dirs: string[];
  notices: string[];
}

export const RACE_LABELS: Record<string, string> = {
  warrior: "Krieger",
  assassin: "Ninja",
  sura: "Sura",
  shaman: "Schamane",
};

// Friendlier German labels for known suffixes - falls back to the raw key
// (still readable, e.g. "custom_glaive") for anything unrecognized.
const VARIANT_LABELS: Record<string, string> = {
  sword: "Schwert",
  blade: "Schwert",
  saber: "Schwert",
  sabre: "Schwert",
  katana: "Schwert",
  "1h": "Einhand",
  sura: "Sura-Variante",
  dagger: "Dolch",
  knife: "Dolch",
  bow: "Bogen",
  "2hand": "Zweihand",
  twohand: "Zweihand",
  "2h": "Zweihand",
  spear: "Zweihand",
  glaive: "Zweihand",
  bell: "Glocke",
  staff: "Glocke (Stab)",
  fan: "Fächer",
  arrow: "Pfeil",
  claw: "Kralle",
};

export function fileNameOf(absPath: string): string {
  return absPath.split(/[\\/]/).pop() ?? absPath;
}

export function labelForVariant(v: WeaponVariant): string {
  for (const [needle, label] of Object.entries(VARIANT_LABELS)) {
    if (v.label.includes(needle)) return v.is_sura_model && needle !== "sura" ? `${label} (Sura)` : label;
  }
  return v.label;
}

// item_proto.name/.locale_name are a fixed-width varbinary column
// (ITEM_NAME_MAX_LEN = 24 in the client's ItemData.h) - MySQL rejects the
// insert outright ("Data too long") rather than truncating, so anything
// built from user-editable base name + suffix must be clamped client-side
// before it ever reaches create_item_proto. Byte length matters, not char
// length, in case the base name contains multi-byte UTF-8 characters.
export const ITEM_NAME_MAX_BYTES = 24;

export function fitToByteLimit(s: string, maxBytes = ITEM_NAME_MAX_BYTES): string {
  const encoder = new TextEncoder();
  if (encoder.encode(s).length <= maxBytes) return s;
  let truncated = s;
  while (truncated.length > 0 && encoder.encode(truncated).length > maxBytes) {
    truncated = truncated.slice(0, -1);
  }
  return truncated;
}

// Wie `fitToByteLimit`, kürzt aber garantiert nur `base`, nie `suffix` (z.B.
// " +16") - reales Nutzerproblem: bei einem Basisnamen, der schon nah am
// Limit lag, hat `fitToByteLimit(base + suffix)` einfach das letzte Zeichen
// der GESAMTEN Zeichenkette abgeschnitten, sobald "+10" statt "+9" (ein
// Zeichen länger) drüber lag - das trifft immer genau die letzte Ziffer der
// Stufenzahl. "+10" wurde so zu "+1", "+11" zu "+1", "+12" zu "+1" usw. -
// jede zweistellige Stufe kollidierte auf denselben abgeschnittenen Namen.
export function fitToByteLimitWithSuffix(base: string, suffix: string, maxBytes = ITEM_NAME_MAX_BYTES): string {
  const encoder = new TextEncoder();
  const budget = Math.max(0, maxBytes - encoder.encode(suffix).length);
  return fitToByteLimit(base, budget) + suffix;
}

// Aufwertungs-Kette (Refine): verified against this server's real stock
// chains (Schwert 10-19, Mönchsplattenpanzer 11200-11209) - both use this
// *exact* cost/prob progression per step, suggesting it's this core's
// standard template rather than a per-item choice. Index i = cost/chance
// for the recipe that takes level i to level i+1. Beyond the 9 defined
// steps, the last step's values repeat flat rather than extrapolating
// further growth - per explicit user instruction (entering e.g. max level
// 15 should just keep reusing the values already established at +9).
const STOCK_REFINE_COST = [600, 1200, 2500, 5000, 10000, 20000, 30000, 45000, 75000];
const STOCK_REFINE_PROB = [90, 90, 90, 90, 80, 60, 60, 60, 60];

export function refineCostForStep(step: number): number {
  return STOCK_REFINE_COST[Math.min(step, STOCK_REFINE_COST.length - 1)];
}
export function refineProbForStep(step: number): number {
  return STOCK_REFINE_PROB[Math.min(step, STOCK_REFINE_PROB.length - 1)];
}

/** Linear-to-base growth for a "combat" stat column: level 0 keeps the
 * reference item's exact value, level N is `base * (1 + growthPercent/100 * N)`.
 * A base of 0 stays 0 - nothing to grow if the reference never set that
 * column in the first place. */
export function scaledValue(base: number, level: number, growthPercent: number): number {
  if (level === 0 || base === 0) return base;
  return Math.round(base * (1 + (growthPercent / 100) * level));
}

/** Finds `count` *consecutive* free vnums starting at/after `rangeStart` -
 * a whole refine chain needs a contiguous block (matching how stock chains
 * are always contiguous, e.g. 10-19), which a single `next_free_item_vnum`
 * call can't guarantee on its own. Normally resolves in one round-trip per
 * slot (a freshly reserved custom vnum range has nothing in the way); only
 * loops further if something unexpected already occupies part of the
 * range. */
export async function findConsecutiveFreeVnums(rangeStart: number, count: number): Promise<number> {
  let candidate = rangeStart;
  for (;;) {
    const base = await invoke<number>("next_free_item_vnum", { rangeStart: candidate });
    let ok = true;
    for (let i = 1; i < count; i++) {
      const next = await invoke<number>("next_free_item_vnum", { rangeStart: base + i });
      if (next !== base + i) {
        candidate = next;
        ok = false;
        break;
      }
    }
    if (ok) return base;
  }
}

export type StepStatus = "pending" | "running" | "done" | "error";
export const WEAPON_WEARFLAG = 1 << 4; // "Waffe" bit, see itemFlags.ts WEAR_FLAGS

// item_proto.antiflag bits, see itemFlags.ts ANTI_FLAGS (source-verified).
const ANTIFLAG_NO_WARRIOR = 1 << 2;
const ANTIFLAG_NO_NINJA = 1 << 3;
const ANTIFLAG_NO_SURA = 1 << 4;
const ANTIFLAG_NO_SHAMAN = 1 << 5;
const ANTIFLAG_NO_WOLFMAN = 1 << 18;
export const CLASS_ANTIFLAG_MASK =
  ANTIFLAG_NO_WARRIOR | ANTIFLAG_NO_NINJA | ANTIFLAG_NO_SURA | ANTIFLAG_NO_SHAMAN | ANTIFLAG_NO_WOLFMAN;
export const RACE_ANTIFLAG: Record<string, number> = {
  warrior: ANTIFLAG_NO_WARRIOR,
  assassin: ANTIFLAG_NO_NINJA,
  sura: ANTIFLAG_NO_SURA,
  shaman: ANTIFLAG_NO_SHAMAN,
};

// Which class(es) can equip a weapon depends only on its subtype (and, for
// swords specifically, whether it's the dedicated Sura reskin) - a "Bell"
// must never be Warrior-equippable just because nothing said otherwise.
// These exact bit combinations were verified live against this server's
// real item_proto (grouped `antiflag` per `subtype` on ~2000 stock weapons,
// 2026-08-04), not guessed. item_proto.size = inventory grid footprint
// (1-3 slots tall, always 1 slot wide), depends only on weapon subtype,
// verified the same way (2026-08-05).
export function sizeForSubtype(subtype: number): number {
  switch (subtype) {
    case 0: // Sword
    case 2: // Bow
      return 2;
    case 3: // Two-Handed
    case 7: // Mount Spear
      return 3;
    default: // Dagger, Bell, Fan, Arrow, Claw, ...
      return 1;
  }
}

export function classAntiflagForWeapon(subtype: number, isSuraModel: boolean): number {
  switch (subtype) {
    case 0: // Sword
      return isSuraModel
        ? ANTIFLAG_NO_WARRIOR | ANTIFLAG_NO_NINJA | ANTIFLAG_NO_SHAMAN | ANTIFLAG_NO_WOLFMAN // Sura only
        : ANTIFLAG_NO_SHAMAN | ANTIFLAG_NO_WOLFMAN; // Warrior + Ninja + Sura
    case 1: // Dagger
    case 2: // Bow
    case 6: // Arrow
      return ANTIFLAG_NO_WARRIOR | ANTIFLAG_NO_SURA | ANTIFLAG_NO_SHAMAN | ANTIFLAG_NO_WOLFMAN; // Ninja only
    case 3: // Two-Handed
      return ANTIFLAG_NO_NINJA | ANTIFLAG_NO_SURA | ANTIFLAG_NO_SHAMAN | ANTIFLAG_NO_WOLFMAN; // Warrior only
    case 4: // Bell
    case 5: // Fan
      return ANTIFLAG_NO_WARRIOR | ANTIFLAG_NO_NINJA | ANTIFLAG_NO_SURA | ANTIFLAG_NO_WOLFMAN; // Shaman only
    case 8: // Claw
      return ANTIFLAG_NO_WARRIOR | ANTIFLAG_NO_NINJA | ANTIFLAG_NO_SURA | ANTIFLAG_NO_SHAMAN; // Wolfman only
    default:
      return 0;
  }
}

export function classLabelFor(blockedMask: number): string {
  const classes = [
    { bit: ANTIFLAG_NO_WARRIOR, label: "Krieger" },
    { bit: ANTIFLAG_NO_NINJA, label: "Ninja" },
    { bit: ANTIFLAG_NO_SURA, label: "Sura" },
    { bit: ANTIFLAG_NO_SHAMAN, label: "Schamane" },
  ]
    .filter((c) => (blockedMask & c.bit) === 0)
    .map((c) => c.label);
  return classes.length > 0 ? classes.join(" + ") : "alle Klassen";
}

export function pad5(vnum: number) {
  return String(vnum).padStart(5, "0");
}
