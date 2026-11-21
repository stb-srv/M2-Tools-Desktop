//! Shell-Hilfen für Befehle, die per SSH auf dem Server laufen.

/// Setzt einen Wert in einfache Anführungszeichen (`'` wird zu `'\''`), damit
/// er in der Remote-Shell nie als Syntax interpretiert wird.
pub fn shell_single_quote(value: &str) -> String {
    format!("'{}'", value.replace('\'', "'\\''"))
}

/// Standardbefehl des Metin2-Steuerskripts: `index.sh` im Arbeitsverzeichnis
/// mit einer Menüauswahl auf stdin.
pub fn index_sh_command(workdir: &str, choice: u32) -> String {
    format!("cd {} && echo {choice} | sh index.sh", shell_single_quote(workdir))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn quotes_plain_and_special_values() {
        assert_eq!(shell_single_quote("/usr/home/game"), "'/usr/home/game'");
        assert_eq!(shell_single_quote("a'b"), "'a'\\''b'");
        assert_eq!(shell_single_quote("x; rm -rf /"), "'x; rm -rf /'");
    }

    #[test]
    fn index_command_quotes_workdir() {
        assert_eq!(index_sh_command("/usr/home/game", 4), "cd '/usr/home/game' && echo 4 | sh index.sh");
        assert_eq!(index_sh_command("/a b/$(x)", 1), "cd '/a b/$(x)' && echo 1 | sh index.sh");
    }
}
