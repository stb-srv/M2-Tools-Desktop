use rusqlite::Connection;
use serde::Serialize;

// Tag/Nacht & Schnee - beides sind server-weite Event-Flags, verifiziert im
// echten Server-Quellcode (game-src/source/game/src), nicht angenommen:
//
// - "day" (questlua_game.cpp: game.set_event_flag) hat eine dem Namen
//   entgegengesetzte Semantik: Wert 1 -> "DayMode dark" (Nacht erzwungen),
//   Wert 0 -> "DayMode light" (Tag) (questmanager.cpp:1172-1189). Absichtlich
//   hier gekapselt, damit das Frontend nur noch "night_enabled" kennt.
// - "xmas_snow" schaltet den Schnee-Partikeleffekt (xmas_event.cpp:13-27),
//   Wert 1 an, 0 aus.
// - `CQuestManager::SetEventFlag` sendet beide SOFORT an alle verbundenen
//   Clients und persistiert in die DB, aber nur wenn der echte Server-Code
//   läuft (z.B. per GM-Befehl oder - wie hier - per Quest-Lua). Ein reiner
//   SQL-Write in dieselbe Tabelle (siehe db/event_flags.rs) hat dagegen bis
//   zum kompletten Server-Neustart keine Wirkung, weil die DB die Flag-Tabelle
//   nur beim eigenen Boot in den Arbeitsspeicher lädt. Neu verbindende
//   Spieler werden zusätzlich automatisch vom eingebauten
//   `BroadcastEventFlagOnLogin` synchronisiert (questmanager.cpp:1374) -
//   nativer Server-Code, keine eigene Quest-Logik dafür nötig.
// - Deploy generiert deshalb eine Quest (wie das Broadcast-System), die beim
//   nächsten Login irgendeines Spielers `game.set_event_flag` einmalig
//   ausführt, per "weather_rev"-Flag gegen wiederholtes Auslösen bei jedem
//   weiteren Login abgesichert.

#[derive(Debug, Clone, Serialize)]
pub struct WeatherState {
    pub night_enabled: bool,
    pub snow_enabled: bool,
    pub revision: i64,
}

pub fn get_state(conn: &Connection) -> Result<WeatherState, String> {
    conn.execute(
        "INSERT OR IGNORE INTO weather_state (id, night_enabled, snow_enabled, revision) VALUES (1, 0, 0, 1)",
        [],
    )
    .map_err(|e| e.to_string())?;
    conn.query_row(
        "SELECT night_enabled, snow_enabled, revision FROM weather_state WHERE id = 1",
        [],
        |row| {
            let night: i64 = row.get(0)?;
            let snow: i64 = row.get(1)?;
            Ok(WeatherState {
                night_enabled: night != 0,
                snow_enabled: snow != 0,
                revision: row.get(2)?,
            })
        },
    )
    .map_err(|e| e.to_string())
}

pub fn set_state(conn: &Connection, night_enabled: bool, snow_enabled: bool) -> Result<WeatherState, String> {
    get_state(conn)?; // stellt sicher, dass die Zeile existiert
    conn.execute(
        "UPDATE weather_state SET night_enabled = ?1, snow_enabled = ?2, revision = revision + 1 WHERE id = 1",
        rusqlite::params![night_enabled as i64, snow_enabled as i64],
    )
    .map_err(|e| e.to_string())?;
    get_state(conn)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn scratch_conn() -> Connection {
        let conn = Connection::open_in_memory().expect("open in-memory db");
        conn.execute_batch(
            "CREATE TABLE weather_state (
                id INTEGER PRIMARY KEY CHECK (id = 1),
                night_enabled INTEGER NOT NULL DEFAULT 0,
                snow_enabled INTEGER NOT NULL DEFAULT 0,
                revision INTEGER NOT NULL DEFAULT 1
            );",
        )
        .expect("create table");
        conn
    }

    #[test]
    fn defaults_to_day_and_no_snow() {
        let conn = scratch_conn();
        let state = get_state(&conn).expect("get failed");
        assert!(!state.night_enabled);
        assert!(!state.snow_enabled);
        assert_eq!(state.revision, 1);
    }

    #[test]
    fn set_state_updates_and_bumps_revision() {
        let conn = scratch_conn();
        let state = set_state(&conn, true, true).expect("set failed");
        assert!(state.night_enabled);
        assert!(state.snow_enabled);
        assert_eq!(state.revision, 2);

        let state = set_state(&conn, false, true).expect("set failed");
        assert!(!state.night_enabled);
        assert!(state.snow_enabled);
        assert_eq!(state.revision, 3);
    }

    #[test]
    fn get_state_is_idempotent_and_does_not_duplicate_row() {
        let conn = scratch_conn();
        get_state(&conn).expect("get failed");
        get_state(&conn).expect("get failed");
        let count: i64 = conn
            .query_row("SELECT COUNT(*) FROM weather_state", [], |r| r.get(0))
            .expect("count failed");
        assert_eq!(count, 1);
    }
}
