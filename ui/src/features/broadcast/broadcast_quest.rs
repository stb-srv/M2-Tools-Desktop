//! Erzeugt die eine Quest-Datei "Broadcast/Broadcast_System.lua" aus der lokal
//! gespeicherten Nachrichtenliste (Port von `broadcastQuest.ts`). Direkt gegen
//! den echten Server-Quellcode geprüft (game-src/source/game/src):
//! - notice_all() (questlua_global.cpp:847) ist ein echter globaler Broadcast.
//! - server_loop_timer(name, seconds) (questlua_global.cpp:154) ist ein echter,
//!   spielerunabhängiger, wiederholender Server-Timer - feuert
//!   `when <name>.server_timer`.
//! - game.get_event_flag/set_event_flag (questlua_game.cpp:16/26) sind globale
//!   In-Memory-Flags, hier genutzt, um den Loop-Timer jeder Nachricht genau
//!   einmal pro Serverlauf scharfzuschalten (beim ersten `login` nach einem
//!   Reload/Neustart).
//!
//! Timer-/Flag-Name enthalten die Revisionsnummer (wird bei jeder Änderung
//! erhöht, siehe broadcast.rs::update_message) - CQuestManager::Reload() leert
//! seine Name->Timer-ID-Tabelle, bricht aber bereits anstehende C++-Timer nicht
//! ab; mit neuem Namen findet ein veraltetes Ereignis nach einem Reload nichts
//! mehr, statt falsch auszulösen.

use serde::Deserialize;

#[derive(Clone, Debug, PartialEq, Deserialize)]
pub struct BroadcastMessage {
    pub id: i64,
    pub text: String,
    pub interval_minutes: i64,
    pub enabled: bool,
    pub revision: i64,
    pub created_at: String,
}

// Gleiche Maskierung wie Quest Builders luaString() - Ansagen sind einzeilig,
// Zeilenumbrüche werden deshalb zu Leerzeichen statt zum "[ENTER]"-Marker.
fn lua_string(text: &str) -> String {
    let escaped = text.replace('\\', "\\\\").replace('"', "\\\"").replace("\r\n", " ").replace('\n', " ");
    format!("\"{escaped}\"")
}

fn timer_name(m: &BroadcastMessage) -> String {
    format!("bc_{}_{}", m.id, m.revision)
}

fn flag_name(m: &BroadcastMessage) -> String {
    format!("bc_armed_{}_{}", m.id, m.revision)
}

pub fn generate_broadcast_quest(messages: &[BroadcastMessage]) -> String {
    let active: Vec<&BroadcastMessage> = messages.iter().filter(|m| m.enabled).collect();

    let mut lines: Vec<String> = vec!["quest broadcast_system begin".into(), "\tstate start begin".into()];

    if !active.is_empty() {
        lines.push("\t\twhen login begin".into());
        for m in &active {
            let flag = flag_name(m);
            let timer = timer_name(m);
            let seconds = m.interval_minutes * 60;
            lines.push(format!("\t\t\tif game.get_event_flag(\"{flag}\") == 0 then"));
            lines.push(format!("\t\t\t\tgame.set_event_flag(\"{flag}\", 1)"));
            lines.push(format!("\t\t\t\tserver_loop_timer(\"{timer}\", {seconds})"));
            lines.push("\t\t\tend".into());
        }
        lines.push("\t\tend".into());

        for m in &active {
            lines.push(String::new());
            lines.push(format!("\t\twhen {}.server_timer begin", timer_name(m)));
            lines.push(format!("\t\t\tnotice_all({})", lua_string(&m.text)));
            lines.push("\t\tend".into());
        }
    }

    lines.push("\tend".into());
    lines.push("end".into());

    lines.join("\n") + "\n"
}

#[cfg(test)]
mod tests {
    use super::*;

    // Gleicher struktureller Smoke-Test wie bei den Quest-Vorlagen - jedes
    // `begin`/`if` braucht genau ein passendes `end`, sonst lehnt der
    // qc_x64-Compiler des Servers die Datei ab.
    fn count_keyword(text: &str, keyword: &str) -> usize {
        let is_word = |c: char| c.is_ascii_alphanumeric() || c == '_';
        let bytes: Vec<char> = text.chars().collect();
        let kw: Vec<char> = keyword.chars().collect();
        let mut n = 0;
        let mut i = 0;
        while i + kw.len() <= bytes.len() {
            if bytes[i..i + kw.len()] == kw[..]
                && (i == 0 || !is_word(bytes[i - 1]))
                && (i + kw.len() == bytes.len() || !is_word(bytes[i + kw.len()]))
            {
                n += 1;
                i += kw.len();
            } else {
                i += 1;
            }
        }
        n
    }

    fn expect_balanced(lua: &str) {
        let opens = count_keyword(lua, "begin") + count_keyword(lua, "if");
        let ends = count_keyword(lua, "end");
        assert_eq!(ends, opens);
    }

    fn message(f: impl FnOnce(&mut BroadcastMessage)) -> BroadcastMessage {
        let mut m = BroadcastMessage {
            id: 1,
            text: "Willkommen auf dem Server!".into(),
            interval_minutes: 30,
            enabled: true,
            revision: 1,
            created_at: "2026-08-09T00:00:00Z".into(),
        };
        f(&mut m);
        m
    }

    #[test]
    fn wraps_everything_in_exactly_one_quest_state_block_and_stays_balanced() {
        let lua = generate_broadcast_quest(&[message(|_| {})]);
        assert!(lua.starts_with("quest broadcast_system begin"));
        assert!(lua.contains("state start begin"));
        expect_balanced(&lua);
    }

    #[test]
    fn emits_a_login_armed_guard_and_a_server_timer_handler_using_the_interval_in_seconds() {
        let lua = generate_broadcast_quest(&[message(|m| {
            m.id = 7;
            m.revision = 3;
            m.interval_minutes = 15;
        })]);
        assert!(lua.contains("game.get_event_flag(\"bc_armed_7_3\")"));
        assert!(lua.contains("game.set_event_flag(\"bc_armed_7_3\", 1)"));
        assert!(lua.contains("server_loop_timer(\"bc_7_3\", 900)"));
        assert!(lua.contains("when bc_7_3.server_timer begin"));
        assert!(lua.contains("notice_all(\"Willkommen auf dem Server!\")"));
    }

    #[test]
    fn leaves_out_disabled_messages_entirely() {
        let lua = generate_broadcast_quest(&[message(|m| {
            m.id = 1;
            m.enabled = false;
        })]);
        assert!(!lua.contains("bc_1_1"));
        assert!(!lua.contains("when login begin"));
        expect_balanced(&lua);
    }

    #[test]
    fn escapes_quotes_and_backslashes_and_collapses_newlines() {
        let lua = generate_broadcast_quest(&[message(|m| m.text = "Sag \"Hallo\"\\Willkommen!\nZeile 2".into())]);
        assert!(lua.contains("notice_all(\"Sag \\\"Hallo\\\"\\\\Willkommen! Zeile 2\")"));
    }

    #[test]
    fn handles_several_messages_with_independent_timers_in_one_login_block() {
        let lua = generate_broadcast_quest(&[
            message(|m| {
                m.id = 1;
                m.revision = 1;
                m.interval_minutes = 10;
            }),
            message(|m| {
                m.id = 2;
                m.revision = 1;
                m.interval_minutes = 20;
                m.text = "Zweite Nachricht".into();
            }),
        ]);
        assert!(lua.contains("server_loop_timer(\"bc_1_1\", 600)"));
        assert!(lua.contains("server_loop_timer(\"bc_2_1\", 1200)"));
        assert!(lua.contains("when bc_1_1.server_timer begin"));
        assert!(lua.contains("when bc_2_1.server_timer begin"));
        expect_balanced(&lua);
    }
}
