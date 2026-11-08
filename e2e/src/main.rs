//! Echter E2E-Smoke-Test gegen das gebaute Desktop-Fenster.
//!
//! Startet `tauri-driver` (WebDriver-Brücke für Tauri-Fenster) gegen
//! `msedgedriver.exe`, öffnet die gebaute .exe, wechselt der Reihe nach über
//! die Befehlspalette (Strg+K) in jeden Sidebar-Bereich und prüft, dass im
//! Hauptbereich sichtbarer, stabiler Text statt einer leeren/abgestürzten
//! Seite erscheint.
//!
//! Voraussetzungen (einmalig):
//!   - `cargo install tauri-driver`
//!   - `winget install Microsoft.EdgeDriver` (passend zur installierten Edge-Version)
//!   - Release-Build: `cargo tauri build` (oder `cd ui && trunk build --release`
//!     und danach `cargo build --release` in src-tauri)
//!
//! Nutzung: `cargo run --manifest-path e2e/Cargo.toml` - baut die App NICHT neu.

use std::path::PathBuf;
use std::process::Stdio;
use std::time::Duration;
use thirtyfour::prelude::*;
use tokio::process::Command;

const TAURI_DRIVER_PORT: u16 = 4444;

// Reihenfolge/Labels wie in ui/src/store/navigation.rs + de.json.
const SECTION_LABELS: &[&str] = &[
    "Dashboard",
    "Server-Steuerung",
    "Quellcode Bauen & Einspielen",
    "Item Editor",
    "Item-Übersicht",
    "Modul-Importer",
    "Aufwertungs-Editor",
    "Cube-Editor",
    "Shop-Editor",
    "Mob-Proto-Editor",
    "Drop-Generator",
    "Quest Builder",
    "Regen-Datei-Editor",
    "Locale-Verwaltung",
    "Server-Events",
    "Account-Verwaltung",
    "Gilden-Verwaltung",
    "Wirtschafts-Dashboard",
    "Datenbank-Explorer",
    "Broadcast-System",
    "Backups",
    "Änderungsprotokoll",
    "TGA Konverter",
    "Icon-Browser",
    "3D-Modell-Viewer",
    "Einstellungen",
];

fn repo_root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("..")
}

fn find_msedgedriver() -> String {
    let candidate = PathBuf::from(std::env::var("LOCALAPPDATA").unwrap_or_default())
        .join("Microsoft")
        .join("WinGet")
        .join("Packages")
        .join("Microsoft.EdgeDriver_Microsoft.Winget.Source_8wekyb3d8bbwe")
        .join("msedgedriver.exe");
    if candidate.exists() {
        candidate.to_string_lossy().into_owned()
    } else {
        "msedgedriver.exe".into() // fällt auf PATH zurück
    }
}

/// Text des Hauptbereichs, sobald er nicht leer und über zwei Lesungen im
/// Abstand von 200ms stabil ist (bis zu 15s).
async fn stable_main_text(driver: &WebDriver) -> WebDriverResult<Option<String>> {
    let main = driver.query(By::Css("main")).wait(Duration::from_secs(5), Duration::from_millis(100)).first().await?;
    let deadline = tokio::time::Instant::now() + Duration::from_secs(15);
    while tokio::time::Instant::now() < deadline {
        let first = main.text().await.unwrap_or_default().trim().to_string();
        if !first.is_empty() {
            tokio::time::sleep(Duration::from_millis(200)).await;
            let second = main.text().await.unwrap_or_default().trim().to_string();
            if first == second {
                return Ok(Some(first));
            }
        } else {
            tokio::time::sleep(Duration::from_millis(200)).await;
        }
    }
    Ok(None)
}

async fn open_section(driver: &WebDriver, label: &str) -> Result<String, String> {
    driver
        .action_chain()
        .key_down(Key::Control)
        .send_keys("k")
        .key_up(Key::Control)
        .perform()
        .await
        .map_err(|e| e.to_string())?;
    let input = driver
        .query(By::Css("input[placeholder*='Bereich springen']"))
        .wait(Duration::from_secs(5), Duration::from_millis(100))
        .first()
        .await
        .map_err(|e| e.to_string())?;
    input.clear().await.map_err(|e| e.to_string())?;
    input.send_keys(label).await.map_err(|e| e.to_string())?;
    tokio::time::sleep(Duration::from_millis(150)).await; // gefilterte Liste
    input.send_keys(Key::Enter).await.map_err(|e| e.to_string())?;
    match stable_main_text(driver).await.map_err(|e| e.to_string())? {
        Some(text) => Ok(text.lines().next().unwrap_or("").chars().take(60).collect()),
        None => Err("Hauptbereich ist leer".into()),
    }
}

#[tokio::main]
async fn main() {
    let exe = repo_root().join("src-tauri").join("target").join("release").join("m2manager.exe");
    if !exe.exists() {
        eprintln!("Release-Build nicht gefunden: {}", exe.display());
        eprintln!("Erst `cargo tauri build` ausführen.");
        std::process::exit(1);
    }

    let msedgedriver = find_msedgedriver();
    println!("tauri-driver wird gestartet (--native-driver {msedgedriver}, Port {TAURI_DRIVER_PORT})...");
    let mut driver_process = Command::new("tauri-driver")
        .args(["--native-driver", &msedgedriver, "--port", &TAURI_DRIVER_PORT.to_string()])
        .stdout(Stdio::null())
        .stderr(Stdio::inherit())
        .spawn()
        .expect("tauri-driver konnte nicht gestartet werden (cargo install tauri-driver)");
    // tauri-driver braucht einen Moment, bevor es Verbindungen annimmt.
    tokio::time::sleep(Duration::from_secs(2)).await;

    let mut caps = Capabilities::new();
    caps.insert("browserName".into(), "wry".into());
    caps.insert("tauri:options".into(), serde_json::json!({ "application": exe.to_string_lossy() }));

    let mut results: Vec<(&str, Result<String, String>)> = vec![];
    match WebDriver::new(format!("http://localhost:{TAURI_DRIVER_PORT}"), caps).await {
        Ok(driver) => {
            println!("Fenster gestartet, warte auf die Sidebar...");
            if let Err(e) = driver.query(By::Css("aside nav")).wait(Duration::from_secs(30), Duration::from_millis(200)).first().await {
                eprintln!("Sidebar erschien nicht: {e}");
            }
            for label in SECTION_LABELS {
                let outcome = open_section(&driver, label).await;
                match &outcome {
                    Ok(d) => println!("✓ {label} → {d}"),
                    Err(e) => println!("✗ {label} → {}", e.lines().next().unwrap_or("")),
                }
                results.push((label, outcome));
            }
            let _ = driver.quit().await;
        }
        Err(e) => {
            eprintln!("WebDriver-Sitzung fehlgeschlagen: {e}");
            let _ = driver_process.kill().await;
            std::process::exit(1);
        }
    }
    let _ = driver_process.kill().await;

    let failed: Vec<&str> = results.iter().filter(|(_, r)| r.is_err()).map(|(l, _)| *l).collect();
    println!("\n{}/{} Bereiche erfolgreich geladen.", results.len() - failed.len(), results.len());
    if !failed.is_empty() {
        println!("Fehlgeschlagen: {}", failed.join(", "));
        std::process::exit(1);
    }
}
