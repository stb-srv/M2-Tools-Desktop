//! Bekannte Event-Flags. Alle Namen/Beschreibungen/Vorgabewerte direkt aus
//! dem echten Server-Quellcode verifiziert (questmanager.cpp,
//! item_manager.cpp, char_battle.cpp, cmd_gm.cpp) - siehe EVENT.md.

pub struct FlagDef {
    pub name: &'static str,
    pub label: &'static str,
    pub hint: &'static str,
}

pub struct SeasonalFlagDef {
    pub def: FlagDef,
    pub min: i64,
    pub fallback_default: i64,
}

const fn f(name: &'static str, label: &'static str, hint: &'static str) -> FlagDef {
    FlagDef { name, label, hint }
}

const fn s(name: &'static str, label: &'static str, hint: &'static str, min: i64, fallback_default: i64) -> SeasonalFlagDef {
    SeasonalFlagDef { def: FlagDef { name, label, hint }, min, fallback_default }
}

pub const RATE_FLAGS: &[FlagDef] = &[
    f("mob_item", "Item-Drop-Rate", "Multiplikator auf die Drop-Chance jedes Items (z.B. „Doppel-Drop“ = 200)."),
    f("mob_exp", "EXP-Rate", "Multiplikator auf erhaltene Erfahrungspunkte."),
    f("mob_gold", "Yang-Betrag", "Multiplikator auf die Höhe eines Yang-Drops (nicht die Chance)."),
    f("mob_gold_pct", "Yang-Drop-Chance", "Multiplikator auf die Chance, dass überhaupt Yang droppt."),
    f("mob_dam", "Monster-Schaden", "Multiplikator auf den von Monstern verursachten Schaden."),
    f("user_dam", "Spieler-Schaden", "Multiplikator auf den von Spielern verursachten Schaden (PvP/PvE)."),
];

pub const RATE_FLAGS_PREMIUM: &[FlagDef] = &[
    f("mob_item_buyer", "Item-Drop-Rate (Premium)", "Wie mob_item, gilt aber nur für Spieler mit aktivem Premium-Item-Timer."),
    f("mob_exp_buyer", "EXP-Rate (Premium)", "Wie mob_exp, nur für Spieler mit aktivem Premium-EXP-Timer."),
    f("mob_gold_buyer", "Yang-Betrag (Premium)", "Wie mob_gold, nur für Spieler mit aktivem Premium-Gold-Timer."),
    f("mob_gold_pct_buyer", "Yang-Drop-Chance (Premium)", "Wie mob_gold_pct, nur für Spieler mit aktivem Premium-Gold-Timer."),
    f("user_dam_buyer", "Spieler-Schaden (Premium)", "Wie user_dam, nur für Spieler mit aktivem Premium-Timer."),
];

pub const SEASONAL_FLAGS: &[SeasonalFlagDef] = &[
    s("valentine_drop", "Valentinstag-Drop", "Zusätzlicher Sonder-Drop. Item-spezifisch im Quellcode verdrahtet.", 1, 2000),
    s("halloween_drop", "Halloween-Drop", "Zusätzlicher Sonder-Drop.", 100, 2000),
    s("easter_drop", "Oster-Drop", "Zusätzlicher Sonder-Drop.", 100, 2000),
    s("whiteday_drop", "White-Day-Drop", "Zusätzlicher Sonder-Drop.", 100, 2000),
    s("football_drop", "Fußball-Event-Drop", "Zusätzlicher Sonder-Drop.", 100, 2000),
    s("ramadan_drop", "Ramadan-Drop", "Zusätzlicher Sonder-Drop.", 100, 2000),
    s("icecream_drop", "Eis-Drop", "Zusätzlicher Sonder-Drop.", 100, 2000),
    s("newyear_fire", "Neujahr: Feuerwerk-Drop", "Zusätzlicher Sonder-Drop.", 100, 1000),
    s("newyear_moon", "Neujahr: Mond-Drop", "Zusätzlicher Sonder-Drop.", 100, 500),
    s(
        "new_xmas_event",
        "Weihnachts-Drop",
        "Zusätzlich nur wirksam, wenn der Spieler ein bestimmtes Item (vnum 53002/53007) besitzt - im Quellcode fest verdrahtet.",
        50,
        100,
    ),
    s("kids_day_drop", "Kindertag-Drop", "Zusätzlicher Sonder-Drop.", 100, 1000),
    s("kids_day_drop_high", "Kindertag-Drop (hochwertig)", "Zusätzlicher Sonder-Drop, seltenere Variante.", 100, 1000),
    s("medal_part_drop", "Medaillen-Teil-Drop", "Nur ab Charakterlevel 30 wirksam - im Quellcode fest verdrahtet.", 50, 100),
    s("three_skill_item", "Drei-Skill-Item-Drop", "Nur ab Level 40 gegen Boss-Monster wirksam - im Quellcode fest verdrahtet.", 1, 1000),
    s("mars_drop", "Mars-Event-Drop", "Zusätzlicher Sonder-Drop, Stärke variiert zusätzlich nach Monster-Rang.", 1000, 1500),
    s("2006_drop", "Jubiläums-Drop (2006)", "Zusätzlicher Sonder-Drop.", 100, 2000),
    s("2007_drop", "Jubiläums-Drop (2007)", "Zusätzlicher Sonder-Drop.", 100, 2000),
    s("horse_skill_book_drop", "Pferde-Skillbuch-Drop", "Zusätzlicher Sonder-Drop.", 1000, 1000000),
];

/// Nicht Teil des Drop-Rate/Saison-Mechanismus - eigene Hooks direkt in
/// `SetEventFlag` (questmanager.cpp:1132 -> xmas_event.cpp:13
/// ProcessEventFlag), mit echten Sichtbarkeits-/Spawn-Effekten. Wert 1 =
/// an, 0 = aus.
pub const SPECIAL_EVENT_FLAGS: &[FlagDef] = &[
    f("xmas_snow", "Schneefall (visuell)", "Rein clientseitiger Effekt, kein Spawn - 1 = an, 0 = aus."),
    f("xmas_song", "Weihnachtsmusik (visuell)", "Rein clientseitiger Effekt, kein Spawn - 1 = an, 0 = aus."),
    f(
        "xmas_boom",
        "Event-Helfer-NPC",
        "Spawnt/entfernt den Event-Helfer-NPC (dieselbe Funktion, die auch der GM-Befehl event_helper auslöst) - 1 = an, 0 = aus.",
    ),
    f(
        "xmas_tree",
        "Weihnachtsbaum-Mob",
        "Spawnt/entfernt einen echten Mob (MOB_XMAS_TREE) an einer festen Koordinate auf Map 61 - 1 = an, 0 = aus.",
    ),
    f("eclipse", "Sonnenfinsternis", "Reines Flag (derselbe Effekt wie der GM-Befehl eclipse) - 1 = an, 0 = aus."),
];

pub fn is_curated(name: &str) -> bool {
    RATE_FLAGS.iter().chain(RATE_FLAGS_PREMIUM).chain(SPECIAL_EVENT_FLAGS).any(|d| d.name == name)
        || SEASONAL_FLAGS.iter().any(|d| d.def.name == name)
}

/// `eventflag <name> <wert>` - leerer Wert wird zu 0.
pub fn gm_command(name: &str, value: &str) -> String {
    format!("eventflag {name} {}", if value.is_empty() { "0" } else { value })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn curated_and_gm_command() {
        assert!(is_curated("mob_item"));
        assert!(is_curated("2006_drop"));
        assert!(is_curated("eclipse"));
        assert!(!is_curated("my_flag"));
        assert_eq!(gm_command("mob_exp", ""), "eventflag mob_exp 0");
        assert_eq!(gm_command("mob_exp", "200"), "eventflag mob_exp 200");
    }
}
