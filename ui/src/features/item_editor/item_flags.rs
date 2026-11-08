//! Referenzdaten für den Item Editor (Port von `itemFlags.ts`).
//!
//! Stand 2026-07-29: alles außer `value_hints` ist direkt aus dem
//! Gameserver-Quellcode dieses Nutzers übernommen
//! (`source/common/{length.h,item_length.h}`), mit aktivem
//! `ENABLE_WOLFMAN` - das verschiebt die Nummerierung mehrerer Enums.
//!
//! - item_proto.type - `enum EItemTypes`, source/common/item_length.h.
//! - item_proto.subtype - Bedeutung hängt vom Typ ab (`E<Type>SubTypes`).
//!   Nicht gelistete Typen fallen in der UI auf ein Zahlenfeld zurück.
//! - item_proto.flag / wearflag / antiflag - Bitmasken aus item_length.h.
//! - item_proto.immuneflag - in der DB ein MySQL-SET('PARA','CURSE','STUN',
//!   'SLEEP','SLOW','POISON','TERROR'); die Rust-Schicht (db/item.rs) wandelt
//!   in eine Bitmaske in genau dieser Reihenfolge um - Reihenfolge nicht ändern.
//! - item_proto.limittype0/1 - `enum ELimitTypes` (Lücke: REAL_TIME = 7).
//! - item_proto.applytype0-3 - `enum EApplyTypes` in length.h.

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct FlagOption {
    pub value: i64,
    pub label: &'static str,
}

pub const ITEM_TYPES: &[FlagOption] = &[
    FlagOption { value: 0, label: "NONE" },
    FlagOption { value: 1, label: "WEAPON" },
    FlagOption { value: 2, label: "ARMOR" },
    FlagOption { value: 3, label: "USE (Verbrauchsgegenstand)" },
    FlagOption { value: 4, label: "AUTOUSE" },
    FlagOption { value: 5, label: "MATERIAL" },
    FlagOption { value: 6, label: "SPECIAL" },
    FlagOption { value: 7, label: "TOOL" },
    FlagOption { value: 8, label: "LOTTERY" },
    FlagOption { value: 9, label: "ELK (Gold-Item)" },
    FlagOption { value: 10, label: "METIN (Stein)" },
    FlagOption { value: 11, label: "CONTAINER" },
    FlagOption { value: 12, label: "FISH" },
    FlagOption { value: 13, label: "ROD (Angel)" },
    FlagOption { value: 14, label: "RESOURCE" },
    FlagOption { value: 15, label: "CAMPFIRE" },
    FlagOption { value: 16, label: "UNIQUE" },
    FlagOption { value: 17, label: "SKILLBOOK" },
    FlagOption { value: 18, label: "QUEST" },
    FlagOption { value: 19, label: "POLYMORPH" },
    FlagOption { value: 20, label: "TREASURE_BOX" },
    FlagOption { value: 21, label: "TREASURE_KEY" },
    FlagOption { value: 22, label: "SKILLFORGET" },
    FlagOption { value: 23, label: "GIFTBOX" },
    FlagOption { value: 24, label: "PICK" },
    FlagOption { value: 25, label: "HAIR" },
    FlagOption { value: 26, label: "TOTEM" },
    FlagOption { value: 27, label: "BLEND" },
    FlagOption { value: 28, label: "COSTUME" },
    FlagOption { value: 29, label: "DS (Dragon Soul)" },
    FlagOption { value: 30, label: "SPECIAL_DS" },
    FlagOption { value: 31, label: "EXTRACT" },
    FlagOption { value: 32, label: "SECONDARY_COIN" },
    FlagOption { value: 33, label: "RING" },
    FlagOption { value: 34, label: "BELT" },
    FlagOption { value: 35, label: "GACHA" },
];

pub const ITEM_FLAGS: &[FlagOption] = &[
    FlagOption { value: 1 << 0, label: "Verfeinerbar" },
    FlagOption { value: 1 << 1, label: "Speichern (persistent)" },
    FlagOption { value: 1 << 2, label: "Stapelbar" },
    FlagOption { value: 1 << 3, label: "Anzahl pro 1 Gold" },
    FlagOption { value: 1 << 4, label: "Langsame Abfrage (Slow Query)" },
    FlagOption { value: 1 << 6, label: "Unique" },
    FlagOption { value: 1 << 7, label: "Herstellungsmenge (Makecount)" },
    FlagOption { value: 1 << 8, label: "Nicht entfernbar" },
    FlagOption { value: 1 << 9, label: "Bestätigung bei Benutzung" },
    FlagOption { value: 1 << 10, label: "Quest-Nutzung" },
    FlagOption { value: 1 << 11, label: "Quest-Nutzung (mehrfach)" },
    FlagOption { value: 1 << 12, label: "Quest-Vergabe" },
    FlagOption { value: 1 << 13, label: "Log" },
    FlagOption { value: 1 << 14, label: "Anwendbar (Applicable)" },
];

pub const WEAR_FLAGS: &[FlagOption] = &[
    FlagOption { value: 1 << 0, label: "Rüstung (Körper)" },
    FlagOption { value: 1 << 1, label: "Kopf/Helm" },
    FlagOption { value: 1 << 2, label: "Schuhe" },
    FlagOption { value: 1 << 3, label: "Armband" },
    FlagOption { value: 1 << 4, label: "Waffe" },
    FlagOption { value: 1 << 5, label: "Halskette" },
    FlagOption { value: 1 << 6, label: "Ohrring" },
    FlagOption { value: 1 << 7, label: "Amulett (Unique)" },
    FlagOption { value: 1 << 8, label: "Pfeile" },
    FlagOption { value: 1 << 9, label: "Schild" },
    FlagOption { value: 1 << 10, label: "Ring" },
    FlagOption { value: 1 << 11, label: "Gürtel" },
    FlagOption { value: 1 << 12, label: "Anhänger (Pendant)" },
    FlagOption { value: 1 << 13, label: "Kostüm: Körper" },
    FlagOption { value: 1 << 14, label: "Kostüm: Haar" },
    FlagOption { value: 1 << 15, label: "Kostüm: Accessoire" },
    FlagOption { value: 1 << 16, label: "Kostüm: Waffe" },
    FlagOption { value: 1 << 17, label: "Kostüm: Reittier" },
    FlagOption { value: 1 << 18, label: "Kostüm: Begleiter" },
];

pub const ANTI_FLAGS: &[FlagOption] = &[
    FlagOption { value: 1 << 0, label: "Keine Frau" },
    FlagOption { value: 1 << 1, label: "Kein Mann" },
    FlagOption { value: 1 << 2, label: "Kein Krieger (Warrior)" },
    FlagOption { value: 1 << 3, label: "Kein Assassine" },
    FlagOption { value: 1 << 4, label: "Kein Sura" },
    FlagOption { value: 1 << 5, label: "Kein Schamane" },
    FlagOption { value: 1 << 6, label: "Nicht aufhebbar (GET)" },
    FlagOption { value: 1 << 7, label: "Nicht droppbar (DROP)" },
    FlagOption { value: 1 << 8, label: "Nicht verkaufbar (SELL)" },
    FlagOption { value: 1 << 9, label: "Kein Reich A" },
    FlagOption { value: 1 << 10, label: "Kein Reich B" },
    FlagOption { value: 1 << 11, label: "Kein Reich C" },
    FlagOption { value: 1 << 12, label: "Nicht speicherbar (SAVE)" },
    FlagOption { value: 1 << 13, label: "Nicht verschenkbar (GIVE)" },
    FlagOption { value: 1 << 14, label: "Kein PK-Drop" },
    FlagOption { value: 1 << 15, label: "Nicht stapelbar (STACK)" },
    FlagOption { value: 1 << 16, label: "Nicht in eigenem Shop (MYSHOP)" },
    FlagOption { value: 1 << 17, label: "Nicht im Safe (SAFEBOX)" },
    FlagOption { value: 1 << 18, label: "Kein Wolfman" },
];

pub const IMMUNE_FLAGS: &[FlagOption] = &[
    FlagOption { value: 1 << 0, label: "Paralyse" },
    FlagOption { value: 1 << 1, label: "Fluch (Curse)" },
    FlagOption { value: 1 << 2, label: "Betäubung (Stun)" },
    FlagOption { value: 1 << 3, label: "Schlaf (Sleep)" },
    FlagOption { value: 1 << 4, label: "Verlangsamung (Slow)" },
    FlagOption { value: 1 << 5, label: "Gift (Poison)" },
    FlagOption { value: 1 << 6, label: "Schrecken (Terror)" },
];

pub const LIMIT_TYPES: &[FlagOption] = &[
    FlagOption { value: 1, label: "Level" },
    FlagOption { value: 2, label: "STR" },
    FlagOption { value: 3, label: "DEX" },
    FlagOption { value: 4, label: "INT" },
    FlagOption { value: 5, label: "CON" },
    FlagOption { value: 7, label: "Echtzeit (Real Time)" },
    FlagOption { value: 8, label: "Echtzeit ab erster Nutzung" },
    FlagOption { value: 9, label: "Timer ab Anlegen (Wear)" },
];

pub const APPLY_TYPES: &[FlagOption] = &[
    FlagOption { value: 0, label: "Keine" },
    FlagOption { value: 1, label: "Max. HP" },
    FlagOption { value: 2, label: "Max. SP" },
    FlagOption { value: 3, label: "CON (Konstitution)" },
    FlagOption { value: 4, label: "INT (Intelligenz)" },
    FlagOption { value: 5, label: "STR (Stärke)" },
    FlagOption { value: 6, label: "DEX (Geschicklichkeit)" },
    FlagOption { value: 7, label: "Angriffstempo" },
    FlagOption { value: 8, label: "Laufgeschwindigkeit" },
    FlagOption { value: 9, label: "Zaubertempo" },
    FlagOption { value: 10, label: "HP-Regeneration" },
    FlagOption { value: 11, label: "SP-Regeneration" },
    FlagOption { value: 12, label: "Gift-Chance" },
    FlagOption { value: 13, label: "Betäubung-Chance" },
    FlagOption { value: 14, label: "Verlangsamung-Chance" },
    FlagOption { value: 15, label: "Kritische-Treffer-Chance" },
    FlagOption { value: 16, label: "Durchdringen-Chance" },
    FlagOption { value: 17, label: "Schadensbonus ggü. Menschen" },
    FlagOption { value: 18, label: "Schadensbonus ggü. Tieren" },
    FlagOption { value: 19, label: "Schadensbonus ggü. Orks" },
    FlagOption { value: 20, label: "Schadensbonus ggü. Milgyo" },
    FlagOption { value: 21, label: "Schadensbonus ggü. Untoten" },
    FlagOption { value: 22, label: "Schadensbonus ggü. Teufeln" },
    FlagOption { value: 23, label: "HP stehlen" },
    FlagOption { value: 24, label: "SP stehlen" },
    FlagOption { value: 25, label: "Mana-Burn-Chance" },
    FlagOption { value: 26, label: "SP-Erholung bei Schaden" },
    FlagOption { value: 27, label: "Blocken" },
    FlagOption { value: 28, label: "Ausweichen" },
    FlagOption { value: 29, label: "Resistenz Schwert" },
    FlagOption { value: 30, label: "Resistenz Zweihand" },
    FlagOption { value: 31, label: "Resistenz Dolch" },
    FlagOption { value: 32, label: "Resistenz Glocke" },
    FlagOption { value: 33, label: "Resistenz Fächer" },
    FlagOption { value: 34, label: "Resistenz Bogen" },
    FlagOption { value: 35, label: "Resistenz Feuer" },
    FlagOption { value: 36, label: "Resistenz Elektrizität" },
    FlagOption { value: 37, label: "Resistenz Magie" },
    FlagOption { value: 38, label: "Resistenz Wind" },
    FlagOption { value: 39, label: "Nahkampf-Reflektion" },
    FlagOption { value: 40, label: "Fluch-Reflektion" },
    FlagOption { value: 41, label: "Gift-Reduzierung" },
    FlagOption { value: 42, label: "SP-Erholung bei Kill" },
    FlagOption { value: 43, label: "EXP-Bonus (verdoppeln)" },
    FlagOption { value: 44, label: "Gold-Bonus (verdoppeln)" },
    FlagOption { value: 45, label: "Item-Drop-Bonus" },
    FlagOption { value: 46, label: "Trank-Bonus" },
    FlagOption { value: 47, label: "HP-Erholung bei Kill" },
    FlagOption { value: 48, label: "Immun gegen Betäubung" },
    FlagOption { value: 49, label: "Immun gegen Verlangsamung" },
    FlagOption { value: 50, label: "Immun gegen Sturz" },
    FlagOption { value: 51, label: "Skill (Sonderfähigkeit)" },
    FlagOption { value: 52, label: "Bogenreichweite" },
    FlagOption { value: 53, label: "Angriffsgrad-Bonus (ATT)" },
    FlagOption { value: 54, label: "Verteidigungsgrad-Bonus (DEF)" },
    FlagOption { value: 55, label: "Magie-Angriffsgrad" },
    FlagOption { value: 56, label: "Magie-Verteidigungsgrad" },
    FlagOption { value: 57, label: "Fluch-Chance" },
    FlagOption { value: 58, label: "Max. Ausdauer" },
    FlagOption { value: 59, label: "Schadensbonus ggü. Kriegern (PvP)" },
    FlagOption { value: 60, label: "Schadensbonus ggü. Assassinen (PvP)" },
    FlagOption { value: 61, label: "Schadensbonus ggü. Sura (PvP)" },
    FlagOption { value: 62, label: "Schadensbonus ggü. Schamanen (PvP)" },
    FlagOption { value: 63, label: "Schadensbonus ggü. Monstern" },
    FlagOption { value: 64, label: "Mall: Angriffsbonus" },
    FlagOption { value: 65, label: "Mall: Verteidigungsbonus" },
    FlagOption { value: 66, label: "Mall: EXP-Bonus" },
    FlagOption { value: 67, label: "Mall: Item-Bonus" },
    FlagOption { value: 68, label: "Mall: Gold-Bonus" },
    FlagOption { value: 69, label: "Max. HP %" },
    FlagOption { value: 70, label: "Max. SP %" },
    FlagOption { value: 71, label: "Skill-Schadensbonus" },
    FlagOption { value: 72, label: "Normaler-Treffer-Schadensbonus" },
    FlagOption { value: 73, label: "Skill-Verteidigungsbonus" },
    FlagOption { value: 74, label: "Normaler-Treffer-Verteidigungsbonus" },
    FlagOption { value: 75, label: "PC-Bang EXP-Bonus" },
    FlagOption { value: 76, label: "PC-Bang Drop-Bonus" },
    FlagOption { value: 77, label: "HP-Extraktion %" },
    FlagOption { value: 78, label: "Resistenz Krieger (PvP)" },
    FlagOption { value: 79, label: "Resistenz Assassine (PvP)" },
    FlagOption { value: 80, label: "Resistenz Sura (PvP)" },
    FlagOption { value: 81, label: "Resistenz Schamane (PvP)" },
    FlagOption { value: 82, label: "Energie" },
    FlagOption { value: 83, label: "Verteidigungsgrad" },
    FlagOption { value: 84, label: "Kostüm-Attribut-Bonus" },
    FlagOption { value: 85, label: "Magie-Angriffsbonus %" },
    FlagOption { value: 86, label: "Nahkampf-Magie-Angriffsbonus %" },
    FlagOption { value: 87, label: "Resistenz Eis" },
    FlagOption { value: 88, label: "Resistenz Erde" },
    FlagOption { value: 89, label: "Resistenz Dunkelheit" },
    FlagOption { value: 90, label: "Anti-Kritisch-Chance" },
    FlagOption { value: 91, label: "Anti-Durchdringen-Chance" },
    // ENABLE_WOLFMAN block - active on this server, do not remove/renumber.
    FlagOption { value: 92, label: "Blutungs-Reduzierung" },
    FlagOption { value: 93, label: "Blutungs-Chance" },
    FlagOption { value: 94, label: "Schadensbonus ggü. Wolfman (PvP)" },
    FlagOption { value: 95, label: "Resistenz Wolfman (PvP)" },
    FlagOption { value: 96, label: "Resistenz Kralle" },
    FlagOption { value: 97, label: "Magie-Widerstand-Reduzierung" },
    FlagOption { value: 98, label: "Resistenz Mensch" },
    FlagOption { value: 99, label: "Acce-Drain-Rate" },
    FlagOption { value: 100, label: "Verzauberung Elektro" },
    FlagOption { value: 101, label: "Verzauberung Feuer" },
    FlagOption { value: 102, label: "Verzauberung Eis" },
    FlagOption { value: 103, label: "Verzauberung Wind" },
    FlagOption { value: 104, label: "Verzauberung Erde" },
    FlagOption { value: 105, label: "Verzauberung Dunkelheit" },
];

pub fn subtypes_by_type(item_type: i64) -> Option<&'static [FlagOption]> {
    Some(match item_type {
        1 => &[
            FlagOption { value: 0, label: "Schwert" },
            FlagOption { value: 1, label: "Dolch" },
            FlagOption { value: 2, label: "Bogen" },
            FlagOption { value: 3, label: "Zweihand" },
            FlagOption { value: 4, label: "Glocke" },
            FlagOption { value: 5, label: "Fächer" },
            FlagOption { value: 6, label: "Pfeil" },
            FlagOption { value: 7, label: "Reitspeer" },
            FlagOption { value: 8, label: "Kralle (Wolfman)" },
            FlagOption { value: 9, label: "Köcher" },
            FlagOption { value: 10, label: "Blumenstrauß" },
        ],
        2 => &[
            FlagOption { value: 0, label: "Körper" },
            FlagOption { value: 1, label: "Kopf" },
            FlagOption { value: 2, label: "Schild" },
            FlagOption { value: 3, label: "Armband" },
            FlagOption { value: 4, label: "Schuhe" },
            FlagOption { value: 5, label: "Halskette" },
            FlagOption { value: 6, label: "Ohrring" },
            FlagOption { value: 7, label: "Anhänger (Pendant)" },
        ],
        3 => &[
            FlagOption { value: 0, label: "Trank (Potion)" },
            FlagOption { value: 1, label: "Talisman" },
            FlagOption { value: 2, label: "Tuning" },
            FlagOption { value: 3, label: "Bewegen (Move)" },
            FlagOption { value: 4, label: "Schatztruhe" },
            FlagOption { value: 5, label: "Geldbeutel" },
            FlagOption { value: 6, label: "Köder" },
            FlagOption { value: 7, label: "Fähigkeit erhöhen" },
            FlagOption { value: 8, label: "Affect (Buff)" },
            FlagOption { value: 9, label: "Stein erzeugen" },
            FlagOption { value: 10, label: "Speziell" },
            FlagOption { value: 11, label: "Trank ohne Delay" },
            FlagOption { value: 12, label: "Clear" },
            FlagOption { value: 13, label: "Unsichtbarkeit" },
            FlagOption { value: 14, label: "Detachment" },
            FlagOption { value: 15, label: "Eimer (Bucket)" },
            FlagOption { value: 16, label: "Trank (fortlaufend)" },
            FlagOption { value: 17, label: "Sockel reinigen" },
            FlagOption { value: 18, label: "Attribut ändern" },
            FlagOption { value: 19, label: "Attribut hinzufügen" },
            FlagOption { value: 20, label: "Accessoire-Sockel hinzufügen" },
            FlagOption { value: 21, label: "In Accessoire-Sockel einsetzen" },
            FlagOption { value: 22, label: "Attribut hinzufügen 2" },
            FlagOption { value: 23, label: "Rezept" },
            FlagOption { value: 24, label: "Attribut ändern 2" },
            FlagOption { value: 25, label: "Binden" },
            FlagOption { value: 26, label: "Entbinden" },
            FlagOption { value: 27, label: "Zeitaufladung (prozentual)" },
            FlagOption { value: 28, label: "Zeitaufladung (fest)" },
            FlagOption { value: 29, label: "In Gürtel-Sockel einsetzen" },
            FlagOption { value: 30, label: "In Ring-Sockel einsetzen" },
            FlagOption { value: 31, label: "Kostüm-Attribut zurücksetzen" },
            FlagOption { value: 32, label: "Kostüm-Attribut ändern" },
            FlagOption { value: 33, label: "Blume" },
        ],
        4 => &[
            FlagOption { value: 0, label: "Trank" },
            FlagOption { value: 1, label: "Fähigkeit erhöhen" },
            FlagOption { value: 2, label: "Bombe" },
            FlagOption { value: 3, label: "Gold" },
            FlagOption { value: 4, label: "Geldbeutel" },
            FlagOption { value: 5, label: "Schatztruhe" },
        ],
        5 => &[
            FlagOption { value: 0, label: "Leder" },
            FlagOption { value: 1, label: "Blut" },
            FlagOption { value: 2, label: "Wurzel" },
            FlagOption { value: 3, label: "Nadel" },
            FlagOption { value: 4, label: "Juwel" },
            FlagOption { value: 5, label: "DS-Verfeinerung (normal)" },
            FlagOption { value: 6, label: "DS-Verfeinerung (gesegnet)" },
            FlagOption { value: 7, label: "DS-Verfeinerung (heilig)" },
        ],
        6 => &[
            FlagOption { value: 0, label: "Karte" },
            FlagOption { value: 1, label: "Schlüssel" },
            FlagOption { value: 2, label: "Dokument" },
            FlagOption { value: 3, label: "Geist (Spirit)" },
        ],
        7 => &[
            FlagOption { value: 0, label: "Angelrute" },
        ],
        8 => &[
            FlagOption { value: 0, label: "Los (Ticket)" },
            FlagOption { value: 1, label: "Sofort-Los" },
        ],
        10 => &[
            FlagOption { value: 0, label: "Normal" },
            FlagOption { value: 1, label: "Gold" },
        ],
        12 => &[
            FlagOption { value: 0, label: "Lebendig" },
            FlagOption { value: 1, label: "Tot" },
        ],
        14 => &[
            FlagOption { value: 0, label: "Fischgräte" },
            FlagOption { value: 1, label: "Wasserstein-Stück" },
            FlagOption { value: 2, label: "Wasserstein" },
            FlagOption { value: 3, label: "Blutperle" },
            FlagOption { value: 4, label: "Blaue Perle" },
            FlagOption { value: 5, label: "Weiße Perle" },
            FlagOption { value: 6, label: "Eimer" },
            FlagOption { value: 7, label: "Kristall" },
            FlagOption { value: 8, label: "Edelstein" },
            FlagOption { value: 9, label: "Stein" },
            FlagOption { value: 10, label: "Metin" },
            FlagOption { value: 11, label: "Erz" },
        ],
        16 => &[
            FlagOption { value: 0, label: "Keine" },
            FlagOption { value: 1, label: "Buch" },
            FlagOption { value: 2, label: "Spezial-Reittier" },
            FlagOption { value: 3, label: "Spezial-Reittier (Mount)" },
        ],
        28 => &[
            FlagOption { value: 0, label: "Körper" },
            FlagOption { value: 1, label: "Haar" },
            FlagOption { value: 2, label: "Accessoire" },
            FlagOption { value: 3, label: "Waffe" },
            FlagOption { value: 4, label: "Reittier" },
            FlagOption { value: 5, label: "Begleiter (Pet)" },
        ],
        31 => &[
            FlagOption { value: 0, label: "Dragon Soul" },
            FlagOption { value: 1, label: "Dragon Heart" },
        ],
        _ => return None,
    })
}

pub fn value_hints(item_type: i64) -> Option<&'static str> {
    match item_type {
        1 => Some("Waffe (100% verifiziert, Server + Client-Tooltip stimmen überein): value3/value4 = Schaden min/max, value5 = Zusatz-Angriffskraft (wird im Client zu beiden addiert angezeigt: \"min+value5 ~ max+value4\"). value0 = Angriffstempo (<80 sehr schnell, ≤95 schnell, ≤105 normal, ≤120 langsam, sonst sehr langsam). value1/value2 = optionaler Magie-Angriff min/max (nur bei magischen Waffen wie Fächer/Glocke ungleich 0, sonst 0 lassen). Bei Zweihandwaffen (Subtyp \"Zweihand\") zieht der Client automatisch 10 vom Angriffstempo ab (TWO_HANDED_WEAPON_ATT_SPEED_DECREASE_VALUE in constinfo.py) - value0=120 wird im Spiel dann als 110 angezeigt/gerechnet, das ist Absicht und kein Fehler."),
        2 => Some("Rüstung (verifiziert): value0 = Magie-Verteidigung, value1 = Verteidigung (Grade), value5 = Zusatz-Verteidigung (im Client verdoppelt zur Grade addiert angezeigt). value2-4 werden im Tooltip nicht verwendet."),
        3 => Some("Verbrauchsgegenstand: Bedeutung hängt vom Subtyp ab (z.B. Trank: value0=Effekt-Typ, value1=Dauer, value2=Betrag) - am zuverlässigsten per Referenz-Item übernehmen"),
        _ => None,
    }
}

/// value0-5-Beschriftungen - nur für WEAPON (1) und ARMOR (2) verifiziert
/// (battle.cpp + uitooltip.py).
pub fn value_labels_by_type(item_type: i64) -> Option<[&'static str; 6]> {
    match item_type {
        1 => Some([
            "Angriffstempo",
            "Magie-Angriff Min (optional)",
            "Magie-Angriff Max (optional)",
            "Schaden Min",
            "Schaden Max",
            "Zusatz-Angriffskraft",
        ]),
        2 => Some([
            "Magie-Verteidigung",
            "Verteidigung (Grade)",
            "value2 (nicht in Tooltip verwendet)",
            "value3 (nicht in Tooltip verwendet)",
            "value4 (nicht in Tooltip verwendet)",
            "Zusatz-Verteidigung",
        ]),
        _ => None,
    }
}

/// Bezeichnung eines Item-Typs (`ITEM_TYPES`), falls bekannt.
pub fn item_type_label(item_type: i64) -> Option<&'static str> {
    ITEM_TYPES.iter().find(|t| t.value == item_type).map(|t| t.label)
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct WeaponDamage {
    pub min: i64,
    pub max: i64,
}

/// Tooltip-Formel des Clients (uitooltip.py, __AppendAttackPowerInfo):
/// `min = value3+value5`, `max = value4+value5`.
pub fn weapon_display_damage(value3: i64, value4: i64, value5: i64) -> WeaponDamage {
    WeaponDamage { min: value3 + value5, max: value4 + value5 }
}

/// Angriffstempo-Kategorien des Client-Tooltips (niedriger = schneller).
pub fn weapon_attack_speed_label(value0: i64) -> &'static str {
    if value0 < 80 {
        "Sehr schnell"
    } else if value0 <= 95 {
        "Schnell"
    } else if value0 <= 105 {
        "Normal"
    } else if value0 <= 120 {
        "Langsam"
    } else {
        "Sehr langsam"
    }
}

/// EWeaponSubTypes::WEAPON_TWO_HANDED.
pub const WEAPON_SUBTYPE_TWO_HANDED: i64 = 3;

/// Der Client zieht bei Zweihandwaffen pauschal 10 vom Angriffstempo ab
/// (constinfo.py: TWO_HANDED_WEAPON_ATT_SPEED_DECREASE_VALUE) - Ursache für
/// "value0=120 wird im Spiel als 110 angezeigt" (vnum 3210).
pub fn weapon_effective_attack_speed(value0: i64, subtype: i64) -> i64 {
    if subtype == WEAPON_SUBTYPE_TWO_HANDED {
        value0 - 10
    } else {
        value0
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn weapon_display_damage_adds_value5_to_both() {
        assert_eq!(weapon_display_damage(10, 20, 5), WeaponDamage { min: 15, max: 25 });
    }

    #[test]
    fn weapon_display_damage_matches_vnum_3210() {
        assert_eq!(weapon_display_damage(10, 20, 0), WeaponDamage { min: 10, max: 20 });
    }

    #[test]
    fn weapon_attack_speed_labels() {
        for (value0, label) in [(79, "Sehr schnell"), (95, "Schnell"), (105, "Normal"), (120, "Langsam"), (121, "Sehr langsam")] {
            assert_eq!(weapon_attack_speed_label(value0), label, "value0={value0}");
        }
    }

    #[test]
    fn effective_attack_speed_one_handed_unchanged() {
        assert_eq!(weapon_effective_attack_speed(120, 0), 120);
    }

    #[test]
    fn effective_attack_speed_two_handed_minus_10() {
        assert_eq!(weapon_effective_attack_speed(120, WEAPON_SUBTYPE_TWO_HANDED), 110);
    }

    #[test]
    fn tables_are_complete() {
        assert_eq!(ITEM_TYPES.len(), 36);
        assert_eq!(APPLY_TYPES.len(), 106);
        assert_eq!(WEAR_FLAGS.len(), 19);
        assert_eq!(ANTI_FLAGS.len(), 19);
        assert_eq!(IMMUNE_FLAGS.len(), 7);
        assert_eq!(subtypes_by_type(1).map(|s| s.len()), Some(11));
        assert_eq!(subtypes_by_type(7).map(|s| s.len()), Some(1));
        assert!(subtypes_by_type(9).is_none());
        assert_eq!(item_type_label(3), Some("USE (Verbrauchsgegenstand)"));
        assert!(value_hints(1).unwrap().contains("\"min+value5 ~ max+value4\""));
    }
}
