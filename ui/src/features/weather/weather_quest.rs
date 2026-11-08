//! Erzeugt die eine Quest-Datei "Weather/Weather_System.lua" aus dem lokal
//! gespeicherten Schalter-Zustand. Direkt gegen den echten Server-Quellcode
//! geprüft (nicht aus allgemeinem Metin2-Wissen geraten):
//! - Das Event-Flag "day" bedeutet das Gegenteil seines Namens:
//!   `questmanager.cpp:1172-1189` zeigt Wert 1 -> "DayMode dark" (Nacht
//!   erzwungen), Wert 0 -> "DayMode light" (Tag). Nach außen gibt es nur
//!   `night_enabled`, die Umkehrung passiert hier.
//! - "xmas_snow" (`xmas_event.cpp:13-27`) ist der Schnee-Partikeleffekt,
//!   1 = an, 0 = aus - ohne Umkehrung.
//! - game.get_event_flag/set_event_flag rufen das echte
//!   `CQuestManager::SetEventFlag` auf, das sofort an alle verbundenen
//!   Clients geht und in der DB landet. Neu verbindende Spieler synchronisiert
//!   der Server selbst (`BroadcastEventFlagOnLogin`), daher muss die Quest die
//!   Flags nur einmal setzen.
//! - "weather_rev" sorgt dafür, dass das pro Revision nur einmal passiert
//!   (gleicher Mechanismus wie das Broadcast-System) - sonst würde jeder
//!   Login beide Flags erneut an alle senden.

use serde::Deserialize;

#[derive(Clone, Debug, PartialEq, Deserialize)]
pub struct WeatherState {
    pub night_enabled: bool,
    pub snow_enabled: bool,
    pub revision: i64,
}

pub fn generate_weather_quest(state: &WeatherState) -> String {
    let day_flag_value = if state.night_enabled { 1 } else { 0 };
    let snow_flag_value = if state.snow_enabled { 1 } else { 0 };
    let rev = state.revision;

    let lines = [
        "quest weather_system begin".to_string(),
        "\tstate start begin".to_string(),
        "\t\twhen login begin".to_string(),
        format!("\t\t\tif game.get_event_flag(\"weather_rev\") ~= {rev} then"),
        format!("\t\t\t\tgame.set_event_flag(\"day\", {day_flag_value})"),
        format!("\t\t\t\tgame.set_event_flag(\"xmas_snow\", {snow_flag_value})"),
        format!("\t\t\t\tgame.set_event_flag(\"weather_rev\", {rev})"),
        "\t\t\tend".to_string(),
        "\t\tend".to_string(),
        "\tend".to_string(),
        "end".to_string(),
    ];

    lines.join("\n") + "\n"
}

/// Kopierbarer Weg für sofortige Wirkung ohne auf den nächsten Login zu
/// warten - dieselben GM-Befehle, die die Quest intern nutzt
/// (`cmd.cpp:189,286`), beide nur GM_HIGH_WIZARD.
pub fn gm_commands_for(night_enabled: bool, snow_enabled: bool) -> Vec<String> {
    vec![
        format!("/eventflag day {}", if night_enabled { 1 } else { 0 }),
        format!("/xmas_snow {}", if snow_enabled { 1 } else { 0 }),
    ]
}

#[cfg(test)]
mod tests {
    use super::*;

    // Gleicher struktureller Test wie beim Broadcast-Quest: jedes
    // `begin`/`if` braucht genau ein passendes `end`, sonst lehnt der
    // qc_x64-Compiler des Servers die Datei ab.
    fn count_keyword(text: &str, keyword: &str) -> usize {
        // Wortgrenzen wie `\b` in JS-Regex (Wortzeichen = [A-Za-z0-9_]).
        let is_word = |c: char| c.is_ascii_alphanumeric() || c == '_';
        let bytes: Vec<char> = text.chars().collect();
        let kw: Vec<char> = keyword.chars().collect();
        let mut count = 0;
        let mut i = 0;
        while i + kw.len() <= bytes.len() {
            if bytes[i..i + kw.len()] == kw[..] {
                let before_ok = i == 0 || !is_word(bytes[i - 1]);
                let after_ok = i + kw.len() == bytes.len() || !is_word(bytes[i + kw.len()]);
                if before_ok && after_ok {
                    count += 1;
                    i += kw.len();
                    continue;
                }
            }
            i += 1;
        }
        count
    }

    fn expect_balanced(lua: &str) {
        let opens = count_keyword(lua, "begin") + count_keyword(lua, "if");
        let ends = count_keyword(lua, "end");
        assert_eq!(ends, opens);
    }

    fn st(night: bool, snow: bool, rev: i64) -> WeatherState {
        WeatherState { night_enabled: night, snow_enabled: snow, revision: rev }
    }

    #[test]
    fn wraps_everything_in_one_quest_state_block_and_stays_balanced() {
        let lua = generate_weather_quest(&st(false, false, 1));
        assert!(lua.starts_with("quest weather_system begin"));
        assert!(lua.contains("state start begin"));
        expect_balanced(&lua);
    }

    #[test]
    fn inverts_night_enabled_into_day_flag() {
        let night = generate_weather_quest(&st(true, false, 5));
        assert!(night.contains("game.set_event_flag(\"day\", 1)"));
        let day = generate_weather_quest(&st(false, false, 5));
        assert!(day.contains("game.set_event_flag(\"day\", 0)"));
    }

    #[test]
    fn maps_snow_enabled_onto_xmas_snow() {
        let on = generate_weather_quest(&st(false, true, 2));
        assert!(on.contains("game.set_event_flag(\"xmas_snow\", 1)"));
        let off = generate_weather_quest(&st(false, false, 2));
        assert!(off.contains("game.set_event_flag(\"xmas_snow\", 0)"));
    }

    #[test]
    fn gates_application_behind_revision_guard() {
        let lua = generate_weather_quest(&st(true, true, 4));
        assert!(lua.contains("game.get_event_flag(\"weather_rev\") ~= 4"));
        assert!(lua.contains("game.set_event_flag(\"weather_rev\", 4)"));
        expect_balanced(&lua);
    }

    #[test]
    fn returns_matching_gm_commands() {
        let s = st(true, false, 1);
        assert_eq!(gm_commands_for(s.night_enabled, s.snow_enabled), vec!["/eventflag day 1", "/xmas_snow 0"]);
    }
}
