use serde::Serialize;

// Backups are scattered next to whatever they came from, not centralized -
// `ssh::write_remote_file_with_backup`/`delete_remote_file_with_backup`
// both create a `m2manager_backups` folder right beside the file being
// touched (see ssh.rs), so a quest edit in `share/quest/Biologie/` backs up
// into `share/quest/Biologie/m2manager_backups/`, a mob-drop edit backs up
// next to `mob_drop_item.txt`, and so on. The Backup-Browser just walks the
// remote tree looking for `m2manager_backups` folders rather than assuming
// one central location.
//
// Filenames follow one of two patterns (see ssh.rs):
//   <original_name>.<YYYYMMDD_HHMMSS>.bak       (overwritten)
//   <original_name>.<YYYYMMDD_HHMMSS>.deleted   (deleted)

/// Recovers the original filename from a backup filename, or `None` if it
/// doesn't match the pattern this app itself writes (e.g. some unrelated
/// file a user dropped into the folder by hand).
pub fn original_name(backup_filename: &str) -> Option<String> {
    let without_kind = backup_filename
        .strip_suffix(".bak")
        .or_else(|| backup_filename.strip_suffix(".deleted"))?;

    let dot = without_kind.rfind('.')?;
    let (name, timestamp) = without_kind.split_at(dot);
    let timestamp = &timestamp[1..];

    let is_valid_timestamp = timestamp.len() == 15
        && timestamp.as_bytes()[8] == b'_'
        && timestamp[..8].bytes().all(|b| b.is_ascii_digit())
        && timestamp[9..].bytes().all(|b| b.is_ascii_digit());

    if is_valid_timestamp && !name.is_empty() {
        Some(name.to_string())
    } else {
        None
    }
}

/// Given the full path of a `.bak`/`.deleted` file inside a
/// `m2manager_backups` folder, returns where it would be restored to (one
/// directory up, original filename) - shared by the restore command and the
/// diff-preview command so the two can never disagree about the target.
pub fn target_path_for_backup(backup_path: &str) -> Result<String, String> {
    let (backup_dir, filename) = backup_path
        .rfind('/')
        .map(|idx| (&backup_path[..idx], &backup_path[idx + 1..]))
        .ok_or_else(|| "Ungültiger Backup-Pfad.".to_string())?;
    let original_filename = original_name(filename).ok_or_else(|| {
        format!("'{filename}' sieht nicht wie eine von M2Manager erzeugte Backup-Datei aus.")
    })?;
    let target_dir = backup_dir
        .rfind('/')
        .map(|idx| &backup_dir[..idx])
        .ok_or_else(|| "Konnte den Zielordner nicht bestimmen.".to_string())?;
    Ok(format!("{target_dir}/{original_filename}"))
}

#[derive(Debug, Clone, Serialize)]
pub struct BackupDiff {
    pub target_path: String,
    pub backup_content: String,
    /// `None` if the target no longer exists (e.g. a `.deleted` backup whose
    /// file was never re-created) - the frontend shows that as "would be
    /// newly created" rather than treating it as a read error.
    pub current_content: Option<String>,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn computes_target_path_from_backup_path() {
        assert_eq!(
            target_path_for_backup(
                "/usr/home/game/share/quest/Biologie/m2manager_backups/Biochecker.lua.20260730_143012.bak"
            ),
            Ok("/usr/home/game/share/quest/Biologie/Biochecker.lua".to_string())
        );
    }

    #[test]
    fn recovers_original_name_from_overwrite_backup() {
        assert_eq!(
            original_name("Biochecker.lua.20260730_143012.bak"),
            Some("Biochecker.lua".to_string())
        );
    }

    #[test]
    fn recovers_original_name_from_delete_backup() {
        assert_eq!(
            original_name("mob_drop_item.txt.20260101_000000.deleted"),
            Some("mob_drop_item.txt".to_string())
        );
    }

    #[test]
    fn rejects_unrelated_filenames() {
        assert_eq!(original_name("readme.txt"), None);
        assert_eq!(original_name("something.bak"), None);
        assert_eq!(original_name("file.2026-07-30.bak"), None);
    }
}
