//! Gemeinsame Helfer für Tests. Integrationstests, die echte Client-Dateien,
//! eine echte Datenbank oder einen echten Server brauchen, lesen ihre Ziele
//! hier statt aus fest eingetragenen Pfaden eines bestimmten Entwickler-PCs:
//!
//! - `M2_TEST_SETTINGS_DB`: Pfad zur m2manager.sqlite, aus der die
//!   MySQL-/SSH-Zugangsdaten gelesen werden (Standard: die App-Datenbank des
//!   aktuellen Benutzers, also die Verbindung, die in M2Manager eingerichtet ist).
//! - `M2_TEST_CLIENT_DIR`: Metin2-Client-Ordner (Standard: `client_path` aus
//!   dieser Datenbank).
//!
//! Ausführen mit `cargo test -- --ignored`.

use std::path::PathBuf;

/// Die Settings-Datenbank der installierten App (bzw. `M2_TEST_SETTINGS_DB`).
pub fn settings_db_path() -> PathBuf {
    if let Ok(path) = std::env::var("M2_TEST_SETTINGS_DB") {
        return PathBuf::from(path);
    }
    let base = std::env::var_os("APPDATA")
        .map(PathBuf::from)
        .or_else(|| std::env::var_os("HOME").map(|h| PathBuf::from(h).join(".local").join("share")))
        .expect("weder APPDATA noch HOME gesetzt - M2_TEST_SETTINGS_DB angeben");
    base.join("com.m2manager.app").join("m2manager.sqlite")
}

/// Liest einen Wert aus der Settings-Datenbank (`paths`-Tabelle).
pub fn setting(key: &str) -> Option<String> {
    let conn = rusqlite::Connection::open(settings_db_path()).ok()?;
    conn.query_row("SELECT value FROM paths WHERE key = ?1", [key], |r| r.get(0))
        .ok()
}

/// Client-Ordner, falls einer bekannt ist (für Tests, die sich sonst überspringen).
pub fn try_client_dir() -> Option<PathBuf> {
    std::env::var("M2_TEST_CLIENT_DIR")
        .ok()
        .or_else(|| setting("client_path"))
        .map(PathBuf::from)
}

/// Metin2-Client-Ordner für Tests, die echte Client-Dateien lesen.
pub fn client_dir() -> PathBuf {
    try_client_dir().expect("kein Client-Ordner: M2_TEST_CLIENT_DIR setzen oder in M2Manager einen Client-Pfad einrichten")
}

/// Verbindung zur eingerichteten Dev-Datenbank (Datenbank `player`).
pub async fn mysql_pool() -> sqlx::MySqlPool {
    let host = setting("mysql_host").expect("mysql_host ist nicht eingerichtet");
    let port: u16 = setting("mysql_port").unwrap_or_else(|| "3306".into()).parse().unwrap();
    let user = setting("mysql_username").expect("mysql_username ist nicht eingerichtet");
    let password = crate::credentials::get_secret("mysql_password")
        .expect("mysql_password ist nicht im Schlüsselbund gespeichert");
    let url = format!("mysql://{user}:{password}@{host}:{port}/player");
    sqlx::mysql::MySqlPoolOptions::new()
        .max_connections(1)
        .connect(&url)
        .await
        .expect("Verbindung zur Dev-Datenbank fehlgeschlagen")
}

/// Erzeugt ein kleines Test-Bild (Verlauf mit Alpha) als Datei - Ersatz für
/// echte Client-Icons in Tests, die nur irgendein lesbares Bild brauchen.
pub fn sample_image(dir: &std::path::Path, file_name: &str) -> PathBuf {
    std::fs::create_dir_all(dir).expect("create fixture dir");
    let img = image::RgbaImage::from_fn(32, 32, |x, y| {
        image::Rgba([(x * 8) as u8, (y * 8) as u8, 128, if x < 4 { 0 } else { 255 }])
    });
    let path = dir.join(file_name);
    img.save(&path).expect("write fixture image");
    path
}

/// Eindeutiger Ordner im Temp-Verzeichnis.
pub fn scratch_dir(prefix: &str) -> PathBuf {
    std::env::temp_dir().join(format!(
        "{prefix}_{}",
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_nanos()
    ))
}
