use crate::db_backup::shell_single_quote;
use serde::{Deserialize, Serialize};

// Server-Quellcode Bauen & Einspielen - baut den echten Metin2-Server-Quellcode
// (game-src) auf dem Live-Server über SSH und spielt das Ergebnis ein.
//
// Kernprinzip, live gegen den echten Server verifiziert (nicht angenommen):
// Es wird NIEMALS direkt im Live-Quellbaum (`/usr/home/source/server`)
// gebaut - jeder Build läuft in einer separaten, dauerhaften Arbeitskopie auf
// demselben Host (rsync-synchronisiert). Das ist wichtig, weil ALLE Channels
// (Channel1-3, Channel99) UND der Loginserver per Symlink auf dieselbe eine
// `game`-Programmdatei zeigen (`/usr/home/source/server/game/game`), ebenso
// `db` (`/usr/home/source/server/db/db`) - es gibt keinen Testserver, jede
// gebaute Version betrifft sofort den kompletten Live-Server gleichzeitig.
//
// Build-Reihenfolge (kein übergeordnetes Makefile existiert): erst die 4
// Bibliotheken (beliebige Reihenfolge unter sich), danach `game`/`db`, die
// gegen alle 4 linken. `gmake` (GNU Make, nicht das BSD-eigene `make`) ist
// nötig, da die Makefiles GNU-Syntax nutzen - live bestätigt unter
// `/usr/local/bin/gmake`.

pub const BUILD_LIBS: [&str; 4] = ["libthecore", "libgame", "libpoly", "libsql"];
pub const BUILD_BINS: [&str; 2] = ["game", "db"];

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BuildTargets {
    pub libs: Vec<String>,
    pub bins: Vec<String>,
}

/// Single source of truth for the frontend's target checkboxes, so the 6
/// names never drift out of sync between Rust and TypeScript.
pub fn build_targets() -> BuildTargets {
    BuildTargets { libs: BUILD_LIBS.iter().map(|s| s.to_string()).collect(), bins: BUILD_BINS.iter().map(|s| s.to_string()).collect() }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BuildOptions {
    /// Die Arbeitskopie auf dem Server, z.B. `/usr/home/m2manager_build/server`
    /// - niemals der Live-Quellbaum.
    pub source_root: String,
    /// Teilmenge aus `BUILD_LIBS`+`BUILD_BINS`, in einer für Abhängigkeiten
    /// sicheren Reihenfolge (Bibliotheken vor den Programmdateien, die sie
    /// brauchen) - wird vom Aufrufer zusammengestellt, hier nicht erzwungen.
    pub targets: Vec<String>,
    /// Vor jedem Ziel `gmake clean` ausführen.
    pub clean: bool,
}

/// Baut das kombinierte Shell-Skript für einen vollständigen oder
/// Teil-Build. Jedes Ziel läuft in seiner eigenen Unter-Shell
/// (`(cd verzeichnis && ...)`), damit ein `cd`-Fehlschlag bei einem Ziel
/// nicht das Arbeitsverzeichnis für spätere Ziele verfälscht; `set -e` am
/// Anfang stoppt das gesamte Skript (und damit das Live-Log) sofort, sobald
/// irgendein `gmake` einen Fehlercode liefert, statt mit einem
/// halb-gebauten, verwirrenden Ergebnis weiterzumachen.
pub fn build_script(opts: &BuildOptions) -> Result<String, String> {
    if opts.source_root.trim().is_empty() || opts.source_root.split('/').any(|s| s == "..") {
        return Err("Ungültiger Arbeitskopie-Pfad.".to_string());
    }
    let mut lines = vec!["set -e".to_string()];
    for target in &opts.targets {
        // Nur bekannte Ziele - der Name landet in einem Shell-Skript.
        if !BUILD_LIBS.contains(&target.as_str()) && !BUILD_BINS.contains(&target.as_str()) {
            return Err(format!("Unbekanntes Build-Ziel '{target}'."));
        }
        let clean = if opts.clean { "gmake clean && " } else { "" };
        let dir = format!("{}/{target}/src", opts.source_root);
        lines.push(format!("echo '=== gmake {target} ==='; (cd {} && {clean}gmake)", shell_single_quote(&dir),));
    }
    Ok(lines.join("\n"))
}

fn parent_dir(path: &str) -> &str {
    match path.rfind('/') {
        Some(idx) => &path[..idx],
        None => ".",
    }
}

/// Synchronisiert den Live-Quellbaum in die Arbeitskopie (einseitig, Live ->
/// Arbeitskopie, nie umgekehrt), damit manuelle SSH-Änderungen (wie der
/// char_item.cpp-Patch für generische Aufwertungs-Schriftrollen) vor dem
/// nächsten Bauen übernommen werden. `rsync` ist auf dem echten Server
/// bestätigt vorhanden (`/usr/local/bin/rsync`).
///
/// **Bugfix (2026-08-07, live gefunden):** jedes Makefile bindet die
/// Drittanbieter-Bibliotheken (mysql/lua/boost/...) über einen relativen
/// Pfad `-I../../../extern/FreeBSD/...` ein - das setzt voraus, dass
/// `extern/` als GESCHWISTER-Ordner neben `server/` liegt, genau wie beim
/// echten Live-Baum (`/usr/home/source/{server,extern}`). Diese Funktion
/// synchronisierte bisher nur `server/` in die Arbeitskopie, ohne dass
/// deren Elternordner ein `extern/` besäße - jeder Build brach deshalb bei
/// `libsql` mit `'mysql/mysql.h' file not found` ab, sobald der relative
/// Pfad ins Leere zeigte. Fix: zusätzlich einen Symlink
/// `<Arbeitskopie-Elternordner>/extern -> <Live-Elternordner>/extern`
/// anlegen (nie kopieren - die Drittanbieter-Bibliotheken sind groß,
/// bereits fertig gebaut und werden von diesem Tool nie verändert, ein
/// Symlink reicht und spart Platz/Zeit). `ln -sfn` ist idempotent - ein
/// wiederholter Aufruf ersetzt einen vorherigen Symlink, statt sich
/// darunter zu verschachteln.
pub fn sync_script(live_root: &str, scratch_root: &str) -> Result<String, String> {
    validate_scratch_root(live_root, scratch_root)?;
    let live_parent = parent_dir(live_root);
    let scratch_parent = parent_dir(scratch_root);
    let q = shell_single_quote;
    Ok(format!(
        "mkdir -p {scratch} && rsync -a --delete {live_src} {scratch_dst} && \
         mkdir -p {scratch_parent} && ln -sfn {extern_live} {extern_scratch}",
        scratch = q(scratch_root),
        live_src = q(&format!("{}/", live_root.trim_end_matches('/'))),
        scratch_dst = q(&format!("{}/", scratch_root.trim_end_matches('/'))),
        scratch_parent = q(scratch_parent),
        extern_live = q(&format!("{live_parent}/extern")),
        extern_scratch = q(&format!("{scratch_parent}/extern")),
    ))
}

/// `rsync --delete` löscht im Ziel - der Zielpfad muss deshalb eindeutig eine
/// eigene Arbeitskopie sein: absolut, nicht Wurzel, ohne `..`, und weder
/// gleich noch ober-/unterhalb des Live-Quellbaums.
pub fn validate_scratch_root(live_root: &str, scratch_root: &str) -> Result<(), String> {
    let scratch = scratch_root.trim();
    if scratch.is_empty() {
        return Err("Der Pfad der Arbeitskopie ist leer.".to_string());
    }
    if !scratch.starts_with('/') {
        return Err(format!("Der Pfad der Arbeitskopie '{scratch}' muss absolut sein."));
    }
    if scratch.split('/').any(|s| s == "..") || live_root.split('/').any(|s| s == "..") {
        return Err("Pfade mit '..' sind nicht erlaubt.".to_string());
    }
    let s = scratch.trim_end_matches('/');
    let l = live_root.trim().trim_end_matches('/');
    if s.is_empty() {
        return Err("Die Arbeitskopie darf nicht '/' sein.".to_string());
    }
    if !l.starts_with('/') {
        return Err(format!("Der Live-Quellpfad '{live_root}' muss absolut sein."));
    }
    let inside = |a: &str, b: &str| a == b || a.starts_with(&format!("{b}/"));
    if inside(s, l) || inside(l, s) {
        return Err(format!(
            "Die Arbeitskopie '{scratch}' darf nicht gleich, über- oder unterhalb des Live-Quellbaums '{live_root}' liegen."
        ));
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn build_script_stops_on_first_failure_and_orders_targets_as_given() {
        let opts = BuildOptions {
            source_root: "/usr/home/m2manager_build/server".into(),
            targets: vec!["libthecore".into(), "libgame".into(), "game".into()],
            clean: false,
        };
        let script = build_script(&opts).unwrap();
        assert!(script.starts_with("set -e\n"), "must abort the whole script on first error");
        assert!(script.contains("(cd '/usr/home/m2manager_build/server/libthecore/src' && gmake)"));
        assert!(script.contains("(cd '/usr/home/m2manager_build/server/libgame/src' && gmake)"));
        assert!(script.contains("(cd '/usr/home/m2manager_build/server/game/src' && gmake)"));
        // libthecore before libgame before game, matching the order given.
        let i1 = script.find("libthecore/src").unwrap();
        let i2 = script.find("libgame/src").unwrap();
        let i3 = script.find("game/src").unwrap();
        assert!(i1 < i2 && i2 < i3);
    }

    #[test]
    fn build_script_adds_clean_before_gmake_when_requested() {
        let opts = BuildOptions { source_root: "/root".into(), targets: vec!["db".into()], clean: true };
        let script = build_script(&opts).unwrap();
        assert!(script.contains("gmake clean && gmake"));
    }

    #[test]
    fn sync_script_never_deletes_from_the_live_tree() {
        let script = sync_script("/usr/home/source/server", "/usr/home/m2manager_build/server").unwrap();
        // rsync's --delete only ever removes files from the *destination*
        // that aren't in the source - the live tree is always the source
        // here, so it can only ever be read from, never written/pruned.
        assert!(script.contains("rsync -a --delete '/usr/home/source/server/' '/usr/home/m2manager_build/server/'"));
    }

    #[test]
    fn sync_script_symlinks_extern_as_a_sibling_of_the_scratch_copy() {
        // Regression test for a real build failure (2026-08-07): every
        // Makefile's `-I../../../extern/FreeBSD/...` include path assumes
        // `extern/` sits next to `server/`, exactly like the live tree
        // (`/usr/home/source/{server,extern}`) - without this symlink,
        // `gmake libsql` fails with "'mysql/mysql.h' file not found" the
        // moment the scratch copy's `server/` has no `extern/` sibling.
        let script = sync_script("/usr/home/source/server", "/usr/home/m2manager_build/server").unwrap();
        assert!(script.contains("ln -sfn '/usr/home/source/extern' '/usr/home/m2manager_build/extern'"));
    }

    #[test]
    fn build_script_rejects_unknown_targets_and_quotes_paths() {
        let bad = BuildOptions { source_root: "/r".into(), targets: vec!["x; rm -rf /".into()], clean: false };
        assert!(build_script(&bad).is_err());
        let quoted = BuildOptions { source_root: "/r'; echo hi; '".into(), targets: vec!["db".into()], clean: false };
        let script = build_script(&quoted).unwrap();
        assert!(script.contains("(cd '/r'\\''; echo hi; '\\''/db/src' && gmake)"));
    }

    #[test]
    fn scratch_root_validation() {
        let live = "/usr/home/source/server";
        assert!(validate_scratch_root(live, "/usr/home/m2manager_build/server").is_ok());
        for bad in ["", "/", "relativ/pfad", "/a/../b", live, "/usr/home/source/server/", "/usr/home/source", "/usr/home/source/server/sub"]
        {
            assert!(validate_scratch_root(live, bad).is_err(), "{bad}");
        }
        assert!(sync_script(live, "/").is_err());
    }
}
