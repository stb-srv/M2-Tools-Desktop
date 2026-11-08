//! Nachbearbeitung nach `tauri-apps/tauri-action`: die Action lädt bereits ein
//! `latest.json` als Release-Asset hoch, dessen `platforms.*.url`-Felder normale
//! `browser_download_url`s sind. Dieses Programm ersetzt jede URL 1:1 durch die
//! entsprechende `api.github.com/.../releases/assets/{id}`-URL (per Dateiname aus
//! der echten Asset-Liste des soeben erstellten Release aufgelöst) und schreibt
//! das Ergebnis nach `updater/latest.json` - das committet der Workflow danach
//! auf `master`, wo die App es über eine stabile raw.githubusercontent.com-URL
//! abfragt. Die api.github.com-URLs funktionieren ohne Token (öffentliches Repo)
//! und mit dem in <= 0.17.0 eingebetteten Token.
//!
//! Aufruf: `rewrite-latest-json <releaseId>` (braucht `gh` und `GITHUB_REPOSITORY`).

use serde_json::Value;
use std::collections::HashMap;
use std::process::{exit, Command};

fn fail(msg: &str) -> ! {
    eprintln!("{msg}");
    exit(1)
}

fn gh_api(path: &str, extra: &[&str]) -> String {
    let out = Command::new("gh")
        .arg("api")
        .arg(path)
        .args(extra)
        .output()
        .unwrap_or_else(|e| fail(&format!("gh konnte nicht gestartet werden: {e}")));
    if !out.status.success() {
        fail(&format!("gh api {path} fehlgeschlagen: {}", String::from_utf8_lossy(&out.stderr)));
    }
    String::from_utf8(out.stdout).unwrap_or_else(|_| fail("gh lieferte kein UTF-8"))
}

/// Letztes Pfadsegment einer URL, %-dekodiert (wie
/// `decodeURIComponent(new URL(u).pathname.split("/").pop())`).
fn file_name_from_url(url: &str) -> String {
    let no_query = url.split(['?', '#']).next().unwrap_or(url);
    let last = no_query.rsplit('/').next().unwrap_or("");
    percent_decode(last)
}

fn percent_decode(s: &str) -> String {
    let bytes = s.as_bytes();
    let mut out = Vec::with_capacity(bytes.len());
    let mut i = 0;
    while i < bytes.len() {
        if bytes[i] == b'%' && i + 2 < bytes.len() {
            if let Ok(b) = u8::from_str_radix(&s[i + 1..i + 3], 16) {
                out.push(b);
                i += 3;
                continue;
            }
        }
        out.push(bytes[i]);
        i += 1;
    }
    String::from_utf8_lossy(&out).into_owned()
}

fn main() {
    let release_id = std::env::args().nth(1).unwrap_or_else(|| fail("Usage: rewrite-latest-json <releaseId>"));
    let repo = std::env::var("GITHUB_REPOSITORY")
        .unwrap_or_else(|_| fail("GITHUB_REPOSITORY env var not set (expected to run inside GitHub Actions)."));

    let assets: Vec<Value> = serde_json::from_str(&gh_api(&format!("repos/{repo}/releases/{release_id}/assets"), &[]))
        .unwrap_or_else(|e| fail(&format!("Asset-Liste ungültig: {e}")));
    let by_name: HashMap<String, i64> = assets
        .iter()
        .filter_map(|a| Some((a.get("name")?.as_str()?.to_string(), a.get("id")?.as_i64()?)))
        .collect();

    let latest_id = by_name.get("latest.json").copied().unwrap_or_else(|| {
        fail("No \"latest.json\" asset found on this release - check that uploadUpdaterJson is enabled and that tauri.conf.json has bundle.createUpdaterArtifacts set.")
    });
    let raw = gh_api(&format!("repos/{repo}/releases/assets/{latest_id}"), &["-H", "Accept: application/octet-stream"]);
    let mut manifest: Value = serde_json::from_str(&raw).unwrap_or_else(|e| fail(&format!("latest.json ungültig: {e}")));

    if let Some(platforms) = manifest.get_mut("platforms").and_then(Value::as_object_mut) {
        for (platform, info) in platforms.iter_mut() {
            let url = info.get("url").and_then(Value::as_str).unwrap_or_default();
            let filename = file_name_from_url(url);
            let Some(id) = by_name.get(&filename) else {
                fail(&format!("No release asset matches platform \"{platform}\"'s expected filename \"{filename}\"."));
            };
            info["url"] = Value::String(format!("https://api.github.com/repos/{repo}/releases/assets/{id}"));
        }
    }

    let pretty = serde_json::to_string_pretty(&manifest).expect("serialisierbar");
    std::fs::create_dir_all("updater").unwrap_or_else(|e| fail(&format!("updater/ anlegen: {e}")));
    std::fs::write("updater/latest.json", format!("{pretty}\n")).unwrap_or_else(|e| fail(&format!("schreiben: {e}")));
    println!("Wrote updater/latest.json with authenticated api.github.com asset URLs:");
    println!("{pretty}");
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn decodes_file_names() {
        assert_eq!(
            file_name_from_url("https://github.com/o/r/releases/download/v1/M2Manager_0.17.1_x64-setup.exe.sig"),
            "M2Manager_0.17.1_x64-setup.exe.sig"
        );
        assert_eq!(file_name_from_url("https://x/a%20b.zip?x=1"), "a b.zip");
        assert_eq!(percent_decode("100%"), "100%");
    }
}
