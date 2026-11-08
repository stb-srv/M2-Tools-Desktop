//! Pfad-Helfer des TGA Konverters (Windows- und Unix-Trenner).

/// Teilt in (Ordner inkl. abschließendem Trenner, Dateiname).
pub fn split_path(path: &str) -> (&str, &str) {
    match path.rfind(['/', '\\']) {
        Some(i) => (&path[..=i], &path[i + 1..]),
        None => ("", path),
    }
}

/// Dateiname ohne Endung; ein führender Punkt zählt nicht als Endung.
pub fn base_name_no_ext(filename: &str) -> &str {
    match filename.rfind('.') {
        Some(dot) if dot > 0 => &filename[..dot],
        _ => filename,
    }
}

/// Gleicher Ordner, gleicher Name, Endung `.tga`.
pub fn default_dest_path(source_path: &str) -> String {
    let (dir, filename) = split_path(source_path);
    format!("{dir}{}.tga", base_name_no_ext(filename))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn splits_both_separators() {
        assert_eq!(split_path("C:\\a\\b.png"), ("C:\\a\\", "b.png"));
        assert_eq!(split_path("/a/b/c.jpg"), ("/a/b/", "c.jpg"));
        assert_eq!(split_path("C:\\a/b\\c.jpg"), ("C:\\a/b\\", "c.jpg"));
        assert_eq!(split_path("c.jpg"), ("", "c.jpg"));
    }

    #[test]
    fn strips_extension() {
        assert_eq!(base_name_no_ext("icon.png"), "icon");
        assert_eq!(base_name_no_ext("a.b.png"), "a.b");
        assert_eq!(base_name_no_ext(".hidden"), ".hidden");
        assert_eq!(base_name_no_ext("noext"), "noext");
    }

    #[test]
    fn default_dest() {
        assert_eq!(default_dest_path("C:\\img\\icon.png"), "C:\\img\\icon.tga");
        assert_eq!(default_dest_path("/x/icon.tga"), "/x/icon.tga");
        assert_eq!(default_dest_path("Bild.jpeg"), "Bild.tga");
    }
}
