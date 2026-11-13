pub mod account;
pub mod economy;
pub mod event_flags;
pub mod explorer;
pub mod guild;
pub mod item;
pub mod item_explorer;
pub mod mapping;
pub mod mob;
pub mod mysql;
pub mod shop;

use sqlx::mysql::MySqlRow;
use sqlx::Row;

/// Liest eine vnum-Spalte unabhängig davon, ob sie im Schema als `INT` oder
/// `INT UNSIGNED` angelegt ist. sqlx prüft Vorzeichen streng: ein `i32` aus
/// einer `UNSIGNED`-Spalte (oder umgekehrt) ist ein Dekodierfehler, der mit
/// `.unwrap_or_default()` still zu 0 wurde. Genau das hat die Item-Übersicht
/// auf einen einzigen Treffer zusammenfallen lassen: alle Zeilen kamen mit
/// vnum 0 an, und die Liste ist nach vnum geschlüsselt.
pub fn get_vnum(row: &MySqlRow, column: &str) -> Option<u32> {
    row.try_get::<u32, _>(column)
        .ok()
        .or_else(|| row.try_get::<i32, _>(column).ok().and_then(|v| u32::try_from(v).ok()))
        .or_else(|| row.try_get::<u64, _>(column).ok().and_then(|v| u32::try_from(v).ok()))
        .or_else(|| row.try_get::<i64, _>(column).ok().and_then(|v| u32::try_from(v).ok()))
}
