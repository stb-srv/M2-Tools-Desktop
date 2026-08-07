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
    BuildTargets {
        libs: BUILD_LIBS.iter().map(|s| s.to_string()).collect(),
        bins: BUILD_BINS.iter().map(|s| s.to_string()).collect(),
    }
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
pub fn build_script(opts: &BuildOptions) -> String {
    let mut lines = vec!["set -e".to_string()];
    for target in &opts.targets {
        let clean = if opts.clean { "gmake clean && " } else { "" };
        lines.push(format!(
            "echo '=== gmake {target} ==='; (cd '{root}/{target}/src' && {clean}gmake)",
            root = opts.source_root,
        ));
    }
    lines.join("\n")
}

/// Synchronisiert den Live-Quellbaum in die Arbeitskopie (einseitig, Live ->
/// Arbeitskopie, nie umgekehrt), damit manuelle SSH-Änderungen (wie der
/// char_item.cpp-Patch für generische Aufwertungs-Schriftrollen) vor dem
/// nächsten Bauen übernommen werden. `rsync` ist auf dem echten Server
/// bestätigt vorhanden (`/usr/local/bin/rsync`).
pub fn sync_script(live_root: &str, scratch_root: &str) -> String {
    format!(
        "mkdir -p '{scratch_root}' && rsync -a --delete '{live_root}/' '{scratch_root}/'",
    )
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
        let script = build_script(&opts);
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
        let opts = BuildOptions {
            source_root: "/root".into(),
            targets: vec!["db".into()],
            clean: true,
        };
        let script = build_script(&opts);
        assert!(script.contains("gmake clean && gmake"));
    }

    #[test]
    fn sync_script_never_deletes_from_the_live_tree() {
        let script = sync_script("/usr/home/source/server", "/usr/home/m2manager_build/server");
        // rsync's --delete only ever removes files from the *destination*
        // that aren't in the source - the live tree is always the source
        // here, so it can only ever be read from, never written/pruned.
        assert!(script.contains("rsync -a --delete '/usr/home/source/server/' '/usr/home/m2manager_build/server/'"));
    }
}
