// Reference data for the Item Editor. As of 2026-07-29 everything in this
// file except VALUE_HINTS is transcribed directly from this user's own
// gameserver source (`C:\Users\DevSteven\Downloads\Fliege V3 - Fratello\...\
// Source\game-src\source\common\{length.h,item_length.h}`), not a generic
// Metin2 guess - verified `#define ENABLE_WOLFMAN` is active in
// source/common/service.h, which matters because it shifts several enums'
// numbering. A hand-typed reference list the user supplied for APPLY_TYPES
// turned out to be wrong for this core from value 3 onward, which is why
// everything here is sourced from the actual enum instead of memory/guessing.

export interface FlagOption {
  value: number;
  label: string;
}

// item_proto.type - `enum EItemTypes`, source/common/item_length.h.
export const ITEM_TYPES: FlagOption[] = [
  { value: 0, label: "NONE" },
  { value: 1, label: "WEAPON" },
  { value: 2, label: "ARMOR" },
  { value: 3, label: "USE (Verbrauchsgegenstand)" },
  { value: 4, label: "AUTOUSE" },
  { value: 5, label: "MATERIAL" },
  { value: 6, label: "SPECIAL" },
  { value: 7, label: "TOOL" },
  { value: 8, label: "LOTTERY" },
  { value: 9, label: "ELK (Gold-Item)" },
  { value: 10, label: "METIN (Stein)" },
  { value: 11, label: "CONTAINER" },
  { value: 12, label: "FISH" },
  { value: 13, label: "ROD (Angel)" },
  { value: 14, label: "RESOURCE" },
  { value: 15, label: "CAMPFIRE" },
  { value: 16, label: "UNIQUE" },
  { value: 17, label: "SKILLBOOK" },
  { value: 18, label: "QUEST" },
  { value: 19, label: "POLYMORPH" },
  { value: 20, label: "TREASURE_BOX" },
  { value: 21, label: "TREASURE_KEY" },
  { value: 22, label: "SKILLFORGET" },
  { value: 23, label: "GIFTBOX" },
  { value: 24, label: "PICK" },
  { value: 25, label: "HAIR" },
  { value: 26, label: "TOTEM" },
  { value: 27, label: "BLEND" },
  { value: 28, label: "COSTUME" },
  { value: 29, label: "DS (Dragon Soul)" },
  { value: 30, label: "SPECIAL_DS" },
  { value: 31, label: "EXTRACT" },
  { value: 32, label: "SECONDARY_COIN" },
  { value: 33, label: "RING" },
  { value: 34, label: "BELT" },
  { value: 35, label: "GACHA" },
];

// item_proto.subtype - meaning depends entirely on item_proto.type. Each
// array below is `E<Type>SubTypes` from source/common/item_length.h. Types
// not listed here either have no meaningful subtype or weren't looked up yet
// - the UI falls back to a raw number field for those.
export const SUBTYPES_BY_TYPE: Record<number, FlagOption[]> = {
  // WEAPON (1) - EWeaponSubTypes. WEAPON_CLAW only exists because
  // ENABLE_WOLFMAN is active on this server.
  1: [
    { value: 0, label: "Schwert" },
    { value: 1, label: "Dolch" },
    { value: 2, label: "Bogen" },
    { value: 3, label: "Zweihand" },
    { value: 4, label: "Glocke" },
    { value: 5, label: "Fächer" },
    { value: 6, label: "Pfeil" },
    { value: 7, label: "Reitspeer" },
    { value: 8, label: "Kralle (Wolfman)" },
    { value: 9, label: "Köcher" },
    { value: 10, label: "Blumenstrauß" },
  ],
  // ARMOR (2) - EArmorSubTypes.
  2: [
    { value: 0, label: "Körper" },
    { value: 1, label: "Kopf" },
    { value: 2, label: "Schild" },
    { value: 3, label: "Armband" },
    { value: 4, label: "Schuhe" },
    { value: 5, label: "Halskette" },
    { value: 6, label: "Ohrring" },
    { value: 7, label: "Anhänger (Pendant)" },
  ],
  // USE (3) - EUseSubTypes.
  3: [
    { value: 0, label: "Trank (Potion)" },
    { value: 1, label: "Talisman" },
    { value: 2, label: "Tuning" },
    { value: 3, label: "Bewegen (Move)" },
    { value: 4, label: "Schatztruhe" },
    { value: 5, label: "Geldbeutel" },
    { value: 6, label: "Köder" },
    { value: 7, label: "Fähigkeit erhöhen" },
    { value: 8, label: "Affect (Buff)" },
    { value: 9, label: "Stein erzeugen" },
    { value: 10, label: "Speziell" },
    { value: 11, label: "Trank ohne Delay" },
    { value: 12, label: "Clear" },
    { value: 13, label: "Unsichtbarkeit" },
    { value: 14, label: "Detachment" },
    { value: 15, label: "Eimer (Bucket)" },
    { value: 16, label: "Trank (fortlaufend)" },
    { value: 17, label: "Sockel reinigen" },
    { value: 18, label: "Attribut ändern" },
    { value: 19, label: "Attribut hinzufügen" },
    { value: 20, label: "Accessoire-Sockel hinzufügen" },
    { value: 21, label: "In Accessoire-Sockel einsetzen" },
    { value: 22, label: "Attribut hinzufügen 2" },
    { value: 23, label: "Rezept" },
    { value: 24, label: "Attribut ändern 2" },
    { value: 25, label: "Binden" },
    { value: 26, label: "Entbinden" },
    { value: 27, label: "Zeitaufladung (prozentual)" },
    { value: 28, label: "Zeitaufladung (fest)" },
    { value: 29, label: "In Gürtel-Sockel einsetzen" },
    { value: 30, label: "In Ring-Sockel einsetzen" },
    { value: 31, label: "Kostüm-Attribut zurücksetzen" },
    { value: 32, label: "Kostüm-Attribut ändern" },
    { value: 33, label: "Blume" },
  ],
  // AUTOUSE (4) - EAutoUseSubTypes.
  4: [
    { value: 0, label: "Trank" },
    { value: 1, label: "Fähigkeit erhöhen" },
    { value: 2, label: "Bombe" },
    { value: 3, label: "Gold" },
    { value: 4, label: "Geldbeutel" },
    { value: 5, label: "Schatztruhe" },
  ],
  // MATERIAL (5) - EMaterialSubTypes.
  5: [
    { value: 0, label: "Leder" },
    { value: 1, label: "Blut" },
    { value: 2, label: "Wurzel" },
    { value: 3, label: "Nadel" },
    { value: 4, label: "Juwel" },
    { value: 5, label: "DS-Verfeinerung (normal)" },
    { value: 6, label: "DS-Verfeinerung (gesegnet)" },
    { value: 7, label: "DS-Verfeinerung (heilig)" },
  ],
  // SPECIAL (6) - ESpecialSubTypes.
  6: [
    { value: 0, label: "Karte" },
    { value: 1, label: "Schlüssel" },
    { value: 2, label: "Dokument" },
    { value: 3, label: "Geist (Spirit)" },
  ],
  // TOOL (7) - EToolSubTypes.
  7: [{ value: 0, label: "Angelrute" }],
  // LOTTERY (8) - ELotterySubTypes.
  8: [
    { value: 0, label: "Los (Ticket)" },
    { value: 1, label: "Sofort-Los" },
  ],
  // METIN (10) - EMetinSubTypes.
  10: [
    { value: 0, label: "Normal" },
    { value: 1, label: "Gold" },
  ],
  // FISH (12) - EFishSubTypes.
  12: [
    { value: 0, label: "Lebendig" },
    { value: 1, label: "Tot" },
  ],
  // RESOURCE (14) - EResourceSubTypes.
  14: [
    { value: 0, label: "Fischgräte" },
    { value: 1, label: "Wasserstein-Stück" },
    { value: 2, label: "Wasserstein" },
    { value: 3, label: "Blutperle" },
    { value: 4, label: "Blaue Perle" },
    { value: 5, label: "Weiße Perle" },
    { value: 6, label: "Eimer" },
    { value: 7, label: "Kristall" },
    { value: 8, label: "Edelstein" },
    { value: 9, label: "Stein" },
    { value: 10, label: "Metin" },
    { value: 11, label: "Erz" },
  ],
  // UNIQUE (16) - EUniqueSubTypes.
  16: [
    { value: 0, label: "Keine" },
    { value: 1, label: "Buch" },
    { value: 2, label: "Spezial-Reittier" },
    { value: 3, label: "Spezial-Reittier (Mount)" },
  ],
  // COSTUME (28) - ECostumeSubTypes (shares numeric space with EArmorSubTypes
  // via COSTUME_BODY = ARMOR_BODY etc. in the source).
  28: [
    { value: 0, label: "Körper" },
    { value: 1, label: "Haar" },
    { value: 2, label: "Accessoire" },
    { value: 3, label: "Waffe" },
    { value: 4, label: "Reittier" },
    { value: 5, label: "Begleiter (Pet)" },
  ],
  // EXTRACT (31) - EExtractSubTypes.
  31: [
    { value: 0, label: "Dragon Soul" },
    { value: 1, label: "Dragon Heart" },
  ],
};

// item_proto.flag - bitmask, `enum EItemFlag` in item_length.h.
export const ITEM_FLAGS: FlagOption[] = [
  { value: 1 << 0, label: "Verfeinerbar" },
  { value: 1 << 1, label: "Speichern (persistent)" },
  { value: 1 << 2, label: "Stapelbar" },
  { value: 1 << 3, label: "Anzahl pro 1 Gold" },
  { value: 1 << 4, label: "Langsame Abfrage (Slow Query)" },
  { value: 1 << 6, label: "Unique" },
  { value: 1 << 7, label: "Herstellungsmenge (Makecount)" },
  { value: 1 << 8, label: "Nicht entfernbar" },
  { value: 1 << 9, label: "Bestätigung bei Benutzung" },
  { value: 1 << 10, label: "Quest-Nutzung" },
  { value: 1 << 11, label: "Quest-Nutzung (mehrfach)" },
  { value: 1 << 12, label: "Quest-Vergabe" },
  { value: 1 << 13, label: "Log" },
  { value: 1 << 14, label: "Anwendbar (Applicable)" },
];

// item_proto.wearflag - bitmask, `enum EItemWearableFlag` in item_length.h.
export const WEAR_FLAGS: FlagOption[] = [
  { value: 1 << 0, label: "Rüstung (Körper)" },
  { value: 1 << 1, label: "Kopf/Helm" },
  { value: 1 << 2, label: "Schuhe" },
  { value: 1 << 3, label: "Armband" },
  { value: 1 << 4, label: "Waffe" },
  { value: 1 << 5, label: "Halskette" },
  { value: 1 << 6, label: "Ohrring" },
  { value: 1 << 7, label: "Amulett (Unique)" },
  { value: 1 << 8, label: "Pfeile" },
  { value: 1 << 9, label: "Schild" },
  { value: 1 << 10, label: "Ring" },
  { value: 1 << 11, label: "Gürtel" },
  { value: 1 << 12, label: "Anhänger (Pendant)" },
  { value: 1 << 13, label: "Kostüm: Körper" },
  { value: 1 << 14, label: "Kostüm: Haar" },
  { value: 1 << 15, label: "Kostüm: Accessoire" },
  { value: 1 << 16, label: "Kostüm: Waffe" },
  { value: 1 << 17, label: "Kostüm: Reittier" },
  { value: 1 << 18, label: "Kostüm: Begleiter" },
];

// item_proto.antiflag - bitmask, `enum EItemAntiFlag` in item_length.h.
// WOLFMAN bit only exists because ENABLE_WOLFMAN is active on this server.
export const ANTI_FLAGS: FlagOption[] = [
  { value: 1 << 0, label: "Keine Frau" },
  { value: 1 << 1, label: "Kein Mann" },
  { value: 1 << 2, label: "Kein Krieger (Warrior)" },
  { value: 1 << 3, label: "Kein Assassine" },
  { value: 1 << 4, label: "Kein Sura" },
  { value: 1 << 5, label: "Kein Schamane" },
  { value: 1 << 6, label: "Nicht aufhebbar (GET)" },
  { value: 1 << 7, label: "Nicht droppbar (DROP)" },
  { value: 1 << 8, label: "Nicht verkaufbar (SELL)" },
  { value: 1 << 9, label: "Kein Reich A" },
  { value: 1 << 10, label: "Kein Reich B" },
  { value: 1 << 11, label: "Kein Reich C" },
  { value: 1 << 12, label: "Nicht speicherbar (SAVE)" },
  { value: 1 << 13, label: "Nicht verschenkbar (GIVE)" },
  { value: 1 << 14, label: "Kein PK-Drop" },
  { value: 1 << 15, label: "Nicht stapelbar (STACK)" },
  { value: 1 << 16, label: "Nicht in eigenem Shop (MYSHOP)" },
  { value: 1 << 17, label: "Nicht im Safe (SAFEBOX)" },
  { value: 1 << 18, label: "Kein Wolfman" },
];

// item_proto.immuneflag - bitmask, `enum EImmuneFlags` in length.h.
// Correction (2026-07-29): item_proto.immuneflag is NOT an integer bitmask
// of `EImmuneFlags` on this server - `DESCRIBE` shows it's a MySQL
// `SET('PARA','CURSE','STUN','SLEEP','SLOW','POISON','TERROR')`, a
// different, DB-schema-specific vocabulary (no FALL/REFLECT, has PARA/SLEEP
// instead). The Rust layer (`db/item.rs`) converts between this SET and a
// bitmask for the JSON boundary, using the SET's own declaration order for
// bit positions - these values must stay in that exact order.
export const IMMUNE_FLAGS: FlagOption[] = [
  { value: 1 << 0, label: "Paralyse" },
  { value: 1 << 1, label: "Fluch (Curse)" },
  { value: 1 << 2, label: "Betäubung (Stun)" },
  { value: 1 << 3, label: "Schlaf (Sleep)" },
  { value: 1 << 4, label: "Verlangsamung (Slow)" },
  { value: 1 << 5, label: "Gift (Poison)" },
  { value: 1 << 6, label: "Schrecken (Terror)" },
];

// item_proto.limittype0/1 - `enum ELimitTypes` in item_length.h.
// Note the real enum has a gap: LIMIT_REAL_TIME is explicitly = 7, not 6.
export const LIMIT_TYPES: FlagOption[] = [
  { value: 1, label: "Level" },
  { value: 2, label: "STR" },
  { value: 3, label: "DEX" },
  { value: 4, label: "INT" },
  { value: 5, label: "CON" },
  { value: 7, label: "Echtzeit (Real Time)" },
  { value: 8, label: "Echtzeit ab erster Nutzung" },
  { value: 9, label: "Timer ab Anlegen (Wear)" },
];

// item_proto.applytype0-3 - `enum EApplyTypes` in length.h.
export const APPLY_TYPES: FlagOption[] = [
  { value: 0, label: "Keine" },
  { value: 1, label: "Max. HP" },
  { value: 2, label: "Max. SP" },
  { value: 3, label: "CON (Konstitution)" },
  { value: 4, label: "INT (Intelligenz)" },
  { value: 5, label: "STR (Stärke)" },
  { value: 6, label: "DEX (Geschicklichkeit)" },
  { value: 7, label: "Angriffstempo" },
  { value: 8, label: "Laufgeschwindigkeit" },
  { value: 9, label: "Zaubertempo" },
  { value: 10, label: "HP-Regeneration" },
  { value: 11, label: "SP-Regeneration" },
  { value: 12, label: "Gift-Chance" },
  { value: 13, label: "Betäubung-Chance" },
  { value: 14, label: "Verlangsamung-Chance" },
  { value: 15, label: "Kritische-Treffer-Chance" },
  { value: 16, label: "Durchdringen-Chance" },
  { value: 17, label: "Schadensbonus ggü. Menschen" },
  { value: 18, label: "Schadensbonus ggü. Tieren" },
  { value: 19, label: "Schadensbonus ggü. Orks" },
  { value: 20, label: "Schadensbonus ggü. Milgyo" },
  { value: 21, label: "Schadensbonus ggü. Untoten" },
  { value: 22, label: "Schadensbonus ggü. Teufeln" },
  { value: 23, label: "HP stehlen" },
  { value: 24, label: "SP stehlen" },
  { value: 25, label: "Mana-Burn-Chance" },
  { value: 26, label: "SP-Erholung bei Schaden" },
  { value: 27, label: "Blocken" },
  { value: 28, label: "Ausweichen" },
  { value: 29, label: "Resistenz Schwert" },
  { value: 30, label: "Resistenz Zweihand" },
  { value: 31, label: "Resistenz Dolch" },
  { value: 32, label: "Resistenz Glocke" },
  { value: 33, label: "Resistenz Fächer" },
  { value: 34, label: "Resistenz Bogen" },
  { value: 35, label: "Resistenz Feuer" },
  { value: 36, label: "Resistenz Elektrizität" },
  { value: 37, label: "Resistenz Magie" },
  { value: 38, label: "Resistenz Wind" },
  { value: 39, label: "Nahkampf-Reflektion" },
  { value: 40, label: "Fluch-Reflektion" },
  { value: 41, label: "Gift-Reduzierung" },
  { value: 42, label: "SP-Erholung bei Kill" },
  { value: 43, label: "EXP-Bonus (verdoppeln)" },
  { value: 44, label: "Gold-Bonus (verdoppeln)" },
  { value: 45, label: "Item-Drop-Bonus" },
  { value: 46, label: "Trank-Bonus" },
  { value: 47, label: "HP-Erholung bei Kill" },
  { value: 48, label: "Immun gegen Betäubung" },
  { value: 49, label: "Immun gegen Verlangsamung" },
  { value: 50, label: "Immun gegen Sturz" },
  { value: 51, label: "Skill (Sonderfähigkeit)" },
  { value: 52, label: "Bogenreichweite" },
  { value: 53, label: "Angriffsgrad-Bonus (ATT)" },
  { value: 54, label: "Verteidigungsgrad-Bonus (DEF)" },
  { value: 55, label: "Magie-Angriffsgrad" },
  { value: 56, label: "Magie-Verteidigungsgrad" },
  { value: 57, label: "Fluch-Chance" },
  { value: 58, label: "Max. Ausdauer" },
  { value: 59, label: "Schadensbonus ggü. Kriegern (PvP)" },
  { value: 60, label: "Schadensbonus ggü. Assassinen (PvP)" },
  { value: 61, label: "Schadensbonus ggü. Sura (PvP)" },
  { value: 62, label: "Schadensbonus ggü. Schamanen (PvP)" },
  { value: 63, label: "Schadensbonus ggü. Monstern" },
  { value: 64, label: "Mall: Angriffsbonus" },
  { value: 65, label: "Mall: Verteidigungsbonus" },
  { value: 66, label: "Mall: EXP-Bonus" },
  { value: 67, label: "Mall: Item-Bonus" },
  { value: 68, label: "Mall: Gold-Bonus" },
  { value: 69, label: "Max. HP %" },
  { value: 70, label: "Max. SP %" },
  { value: 71, label: "Skill-Schadensbonus" },
  { value: 72, label: "Normaler-Treffer-Schadensbonus" },
  { value: 73, label: "Skill-Verteidigungsbonus" },
  { value: 74, label: "Normaler-Treffer-Verteidigungsbonus" },
  { value: 75, label: "PC-Bang EXP-Bonus" },
  { value: 76, label: "PC-Bang Drop-Bonus" },
  { value: 77, label: "HP-Extraktion %" },
  { value: 78, label: "Resistenz Krieger (PvP)" },
  { value: 79, label: "Resistenz Assassine (PvP)" },
  { value: 80, label: "Resistenz Sura (PvP)" },
  { value: 81, label: "Resistenz Schamane (PvP)" },
  { value: 82, label: "Energie" },
  { value: 83, label: "Verteidigungsgrad" },
  { value: 84, label: "Kostüm-Attribut-Bonus" },
  { value: 85, label: "Magie-Angriffsbonus %" },
  { value: 86, label: "Nahkampf-Magie-Angriffsbonus %" },
  { value: 87, label: "Resistenz Eis" },
  { value: 88, label: "Resistenz Erde" },
  { value: 89, label: "Resistenz Dunkelheit" },
  { value: 90, label: "Anti-Kritisch-Chance" },
  { value: 91, label: "Anti-Durchdringen-Chance" },
  // ENABLE_WOLFMAN block - active on this server, do not remove/renumber.
  { value: 92, label: "Blutungs-Reduzierung" },
  { value: 93, label: "Blutungs-Chance" },
  { value: 94, label: "Schadensbonus ggü. Wolfman (PvP)" },
  { value: 95, label: "Resistenz Wolfman (PvP)" },
  { value: 96, label: "Resistenz Kralle" },
  { value: 97, label: "Magie-Widerstand-Reduzierung" },
  { value: 98, label: "Resistenz Mensch" },
  { value: 99, label: "Acce-Drain-Rate" },
  { value: 100, label: "Verzauberung Elektro" },
  { value: 101, label: "Verzauberung Feuer" },
  { value: 102, label: "Verzauberung Eis" },
  { value: 103, label: "Verzauberung Wind" },
  { value: 104, label: "Verzauberung Erde" },
  { value: 105, label: "Verzauberung Dunkelheit" },
];

// item_proto.value0-5 - for most types this genuinely could not be reduced
// to a clean lookup table: `CItem::GetValue(idx)` means something completely
// different per item type AND subtype (e.g. every USE subtype - there are 34
// of them - reads value0-2 differently: potions use value0 as an
// affect-type switch, refine scrolls use value0 as a scroll-type constant,
// etc.), confirmed by reading source/game/src/char_item.cpp. A full mapping
// would mean a per-subtype form, not a dropdown - out of scope for now.
//
// WEAPON (1) and ARMOR (2) are the exception: fully verified, both
// server-side (source/game/src/battle.cpp: Item_GetDamage, CalcMeleeDamage -
// exactly what a hit actually rolls against in combat) AND client-side
// (Client/root/uitooltip.py: __AppendAttackPowerInfo, __AppendMagicAttackInfo,
// __AppendMagicDefenceInfo, the armor branch around "defGrade"/"defBonus" -
// exactly what the player sees in the tooltip). Both sources agree, so this
// is as close to 100% certain as this project gets without a live test.
export const VALUE_LABELS_BY_TYPE: Record<number, string[]> = {
  1: [
    "Angriffstempo",
    "Magie-Angriff Min (optional)",
    "Magie-Angriff Max (optional)",
    "Schaden Min",
    "Schaden Max",
    "Zusatz-Angriffskraft",
  ],
  2: [
    "Magie-Verteidigung",
    "Verteidigung (Grade)",
    "value2 (nicht in Tooltip verwendet)",
    "value3 (nicht in Tooltip verwendet)",
    "value4 (nicht in Tooltip verwendet)",
    "Zusatz-Verteidigung",
  ],
};

export const VALUE_HINTS: Record<number, string> = {
  1: "Waffe (100% verifiziert, Server + Client-Tooltip stimmen überein): value3/value4 = Schaden min/max, value5 = Zusatz-Angriffskraft (wird im Client zu beiden addiert angezeigt: \"min+value5 ~ max+value4\"). value0 = Angriffstempo (<80 sehr schnell, ≤95 schnell, ≤105 normal, ≤120 langsam, sonst sehr langsam). value1/value2 = optionaler Magie-Angriff min/max (nur bei magischen Waffen wie Fächer/Glocke ungleich 0, sonst 0 lassen). Bei Zweihandwaffen (Subtyp \"Zweihand\") zieht der Client automatisch 10 vom Angriffstempo ab (TWO_HANDED_WEAPON_ATT_SPEED_DECREASE_VALUE in constinfo.py) - value0=120 wird im Spiel dann als 110 angezeigt/gerechnet, das ist Absicht und kein Fehler.",
  2: "Rüstung (verifiziert): value0 = Magie-Verteidigung, value1 = Verteidigung (Grade), value5 = Zusatz-Verteidigung (im Client verdoppelt zur Grade addiert angezeigt). value2-4 werden im Tooltip nicht verwendet.",
  3: "Verbrauchsgegenstand: Bedeutung hängt vom Subtyp ab (z.B. Trank: value0=Effekt-Typ, value1=Dauer, value2=Betrag) - am zuverlässigsten per Referenz-Item übernehmen",
};

/** Client tooltip formula for weapon damage display (uitooltip.py,
 * __AppendAttackPowerInfo) - `min = value3+value5`, `max = value4+value5`. */
export function weaponDisplayDamage(value3: number, value4: number, value5: number) {
  return { min: value3 + value5, max: value4 + value5 };
}

/** Client tooltip attack-speed category thresholds (uitooltip.py,
 * __AppendAttackSpeedInfo). Lower value0 = faster. */
export function weaponAttackSpeedLabel(value0: number): string {
  if (value0 < 80) return "Sehr schnell";
  if (value0 <= 95) return "Schnell";
  if (value0 <= 105) return "Normal";
  if (value0 <= 120) return "Langsam";
  return "Sehr langsam";
}

// EWeaponSubTypes::WEAPON_TWO_HANDED (see SUBTYPES_BY_TYPE[1]).
export const WEAPON_SUBTYPE_TWO_HANDED = 3;

/** Client-side (Client/root/constinfo.py: TWO_HANDED_WEAPON_ATT_SPEED_
 * DECREASE_VALUE, passed to app.SetTwoHandedWeaponAttSpeedDecreaseValue)
 * subtracts a flat 10 from two-handed weapons' effective attack speed -
 * confirmed against the user's real client config, not a guess. Verified
 * this is the exact source of a reported "value0=120 shows as 110 in-game"
 * discrepancy (vnum 3210, a two-handed weapon). */
export function weaponEffectiveAttackSpeed(value0: number, subtype: number): number {
  return subtype === WEAPON_SUBTYPE_TWO_HANDED ? value0 - 10 : value0;
}
