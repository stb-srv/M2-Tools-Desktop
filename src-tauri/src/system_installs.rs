use rusqlite::Connection;
use serde::{Deserialize, Serialize};

/// Wo eine vom System-Installer geschriebene Datei tatsächlich landet -
/// siehe system_patch.rs-Kommentar und die Modul-Planung: Server-Quellcode
/// immer live über SSH (wie jedes andere Server-Datei-Werkzeug in diesem
/// Projekt), Client-Quellcode lokal im `binary_src_path`-Checkout
/// (Kompilieren bleibt manuell, kein Client-Build-Tool vorhanden),
/// Client-Installationsdateien im bestehenden `client_path`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum TargetKind {
    LiveServer,
    LocalClientSource,
    LocalClientInstall,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum FileAction {
    Patched,
    Created,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct InstalledFile {
    pub target_path: String,
    pub target_kind: TargetKind,
    /// `None` nur bei `action == Created` (keine vorherige Datei zum
    /// Sichern) - sonst immer gesetzt, das Backup ist die Grundlage für
    /// "Rückgängig machen".
    pub backup_path: Option<String>,
    pub action: FileAction,
}

#[derive(Debug, Clone, Serialize)]
pub struct SystemInstall {
    pub id: i64,
    pub system_name: String,
    pub created_at: String,
    pub files: Vec<InstalledFile>,
}

/// Wird einmal aufgerufen, nachdem alle automatisch übernehmbaren Blöcke
/// eines Systems erfolgreich geschrieben wurden (siehe
/// `commands::run_system_install`) - reine Buchführung, kein Datei-I/O
/// hier. Vorbild: `import_history::record_batch`.
pub fn record_install(conn: &Connection, system_name: &str, files: &[InstalledFile]) -> Result<i64, String> {
    let files_json = serde_json::to_string(files).map_err(|e| e.to_string())?;
    let created_at = chrono::Local::now().to_rfc3339();
    conn.execute(
        "INSERT INTO system_installs (system_name, created_at, files_json) VALUES (?1, ?2, ?3)",
        rusqlite::params![system_name, created_at, files_json],
    )
    .map_err(|e| e.to_string())?;
    Ok(conn.last_insert_rowid())
}

pub fn list_installs(conn: &Connection) -> Result<Vec<SystemInstall>, String> {
    let mut stmt = conn
        .prepare("SELECT id, system_name, created_at, files_json FROM system_installs ORDER BY id DESC")
        .map_err(|e| e.to_string())?;
    let rows = stmt
        .query_map([], |row| {
            let files_json: String = row.get(3)?;
            Ok(SystemInstall {
                id: row.get(0)?,
                system_name: row.get(1)?,
                created_at: row.get(2)?,
                files: serde_json::from_str(&files_json).unwrap_or_default(),
            })
        })
        .map_err(|e| e.to_string())?;
    rows.collect::<Result<Vec<_>, _>>().map_err(|e| e.to_string())
}

pub fn get_install(conn: &Connection, id: i64) -> Result<Option<SystemInstall>, String> {
    Ok(list_installs(conn)?.into_iter().find(|i| i.id == id))
}

/// Wird erst NACH einem erfolgreichen Rückgängig-machen aufgerufen (jede
/// Datei aus ihrem Backup wiederhergestellt bzw. gelöscht) - der Verlaufs-
/// Eintrag selbst hat danach keinen Zweck mehr. Vorbild:
/// `import_history::delete_batch_record`.
pub fn delete_install_record(conn: &Connection, id: i64) -> Result<(), String> {
    conn.execute("DELETE FROM system_installs WHERE id = ?1", [id])
        .map_err(|e| e.to_string())?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn scratch_conn() -> Connection {
        let conn = Connection::open_in_memory().expect("open in-memory db");
        conn.execute_batch(
            "CREATE TABLE system_installs (
                id INTEGER PRIMARY KEY AUTOINCREMENT,
                system_name TEXT NOT NULL,
                created_at TEXT NOT NULL,
                files_json TEXT NOT NULL
            );",
        )
        .expect("create table");
        conn
    }

    fn sample_files() -> Vec<InstalledFile> {
        vec![
            InstalledFile {
                target_path: "/usr/home/source/server/game/src/cmd_gm.cpp".into(),
                target_kind: TargetKind::LiveServer,
                backup_path: Some("/usr/home/source/server/game/src/m2manager_backups/cmd_gm.cpp.20260101.bak".into()),
                action: FileAction::Patched,
            },
            InstalledFile {
                target_path: "root/uiadminpanel_module/aslan_create_item.py".into(),
                target_kind: TargetKind::LocalClientInstall,
                backup_path: None,
                action: FileAction::Created,
            },
        ]
    }

    #[test]
    fn records_and_lists_and_deletes_an_install() {
        let conn = scratch_conn();
        let id = record_install(&conn, "InGame-Admin: Create Item", &sample_files()).expect("record failed");

        let installs = list_installs(&conn).expect("list failed");
        assert_eq!(installs.len(), 1);
        assert_eq!(installs[0].id, id);
        assert_eq!(installs[0].system_name, "InGame-Admin: Create Item");
        assert_eq!(installs[0].files.len(), 2);
        assert_eq!(installs[0].files[0].target_kind, TargetKind::LiveServer);
        assert_eq!(installs[0].files[1].action, FileAction::Created);
        assert!(installs[0].files[1].backup_path.is_none());

        let fetched = get_install(&conn, id).expect("get failed").expect("install missing");
        assert_eq!(fetched.system_name, "InGame-Admin: Create Item");

        delete_install_record(&conn, id).expect("delete failed");
        assert!(list_installs(&conn).expect("list failed").is_empty());
    }

    #[test]
    fn lists_newest_first() {
        let conn = scratch_conn();
        record_install(&conn, "First", &sample_files()).unwrap();
        record_install(&conn, "Second", &sample_files()).unwrap();
        let installs = list_installs(&conn).expect("list failed");
        assert_eq!(installs[0].system_name, "Second");
        assert_eq!(installs[1].system_name, "First");
    }
}
