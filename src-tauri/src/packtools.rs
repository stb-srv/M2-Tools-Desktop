use std::path::{Path, PathBuf};
use std::process::Stdio;
use tauri::{AppHandle, Emitter};
use tokio::io::{AsyncBufReadExt, BufReader};
use tokio::process::Command;

/// Writes an item icon as `.tga` at the exact path the game client (and
/// `icons.rs`'s read-side lookup) expects for a brand-new item: no
/// `item_list.txt` entry exists yet for a custom vnum, so the plain
/// `{vnum:05}.tga` convention is what actually gets picked up.
pub fn write_icon_tga(client_path: &str, vnum: u32, source_image_path: &str) -> Result<PathBuf, String> {
    let path = Path::new(client_path)
        .join("pack")
        .join("icon")
        .join("icon")
        .join("item")
        .join(format!("{vnum:05}.tga"));

    crate::imageconv::convert_to_tga(source_image_path, &path)?;
    Ok(path)
}

/// Copies `path` into a `m2manager_backups` folder next to it, timestamped,
/// before it gets overwritten. Returns `None` if there was nothing to back up.
pub fn backup_file(path: &Path) -> Result<Option<PathBuf>, String> {
    if !path.exists() {
        return Ok(None);
    }
    let parent = path.parent().ok_or("Datei hat kein übergeordnetes Verzeichnis")?;
    let backup_dir = parent.join("m2manager_backups");
    std::fs::create_dir_all(&backup_dir).map_err(|e| e.to_string())?;

    let filename = path
        .file_name()
        .and_then(|f| f.to_str())
        .ok_or("Ungültiger Dateiname")?;
    let timestamp = chrono::Local::now().format("%Y%m%d_%H%M%S");
    let dest = backup_dir.join(format!("{filename}.{timestamp}.bak"));

    std::fs::copy(path, &dest).map_err(|e| e.to_string())?;
    Ok(Some(dest))
}

async fn run_streamed(
    app: &AppHandle,
    event: &str,
    program: &Path,
    args: &[&str],
    cwd: &Path,
) -> Result<String, String> {
    let mut child = Command::new(program)
        .args(args)
        .current_dir(cwd)
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .map_err(|e| format!("{} konnte nicht gestartet werden: {e}", program.display()))?;

    let stdout = child.stdout.take().ok_or("Kein stdout-Handle")?;
    let stderr = child.stderr.take().ok_or("Kein stderr-Handle")?;

    let mut out_lines = BufReader::new(stdout).lines();
    let mut err_lines = BufReader::new(stderr).lines();
    let mut output = String::new();

    loop {
        tokio::select! {
            line = out_lines.next_line() => {
                match line.map_err(|e| e.to_string())? {
                    Some(l) => {
                        let _ = app.emit(event, &l);
                        output.push_str(&l);
                        output.push('\n');
                    }
                    None => break,
                }
            }
            line = err_lines.next_line() => {
                if let Some(l) = line.map_err(|e| e.to_string())? {
                    let _ = app.emit(event, &l);
                    output.push_str(&l);
                    output.push('\n');
                }
            }
        }
    }

    let status = child.wait().await.map_err(|e| e.to_string())?;
    if !status.success() {
        return Err(format!(
            "{} beendete sich mit Fehlercode {:?}\n{output}",
            program.display(),
            status.code()
        ));
    }

    Ok(output)
}

/// Packs `<pack_dir>/<folder_name>` back into `<folder_name>.epk`/`.eix`,
/// e.g. `folder_name = "icon"` after a new icon was dropped into
/// `icon/icon/item/`. Syntax verified live: `EterPackConsoleLz4.exe <folder>`
/// run with cwd = the folder's parent, exit code 0 on success.
pub async fn run_eterpack_pack(
    app: &AppHandle,
    tool_path: &str,
    folder_name: &str,
) -> Result<(), String> {
    let tool = Path::new(tool_path);
    let cwd = tool
        .parent()
        .ok_or("EterPackConsoleLz4-Pfad hat kein übergeordnetes Verzeichnis")?;

    run_streamed(app, "item-editor-output", tool, &[folder_name], cwd).await?;

    let epk = cwd.join(format!("{folder_name}.epk"));
    let eix = cwd.join(format!("{folder_name}.eix"));
    if !epk.exists() || !eix.exists() {
        return Err(format!(
            "Packen abgeschlossen, aber {epk:?}/{eix:?} wurden nicht gefunden."
        ));
    }
    Ok(())
}

/// Runs Mysql2Proto in pack mode (DB -> client proto file) for item_proto
/// only (`-pi`: pack phase, item proto). It connects using its own
/// `Mysql2Proto.json` in `tool_dir` and writes `item_proto` there.
pub async fn run_mysql2proto(app: &AppHandle, tool_dir: &str) -> Result<PathBuf, String> {
    let dir = Path::new(tool_dir);
    let exe = dir.join("Mysql2Proto.exe");
    if !exe.exists() {
        return Err(format!("Mysql2Proto.exe nicht gefunden unter {tool_dir}"));
    }

    run_streamed(app, "item-editor-output", &exe, &["-pi"], dir).await?;

    let generated = dir.join("item_proto");
    if !generated.exists() {
        return Err("Mysql2Proto lief durch, aber item_proto wurde nicht erzeugt.".into());
    }
    Ok(generated)
}

/// Finds every `locale/*/item_proto` under the client path, backs each up,
/// and overwrites it with the freshly generated file. Returns the list of
/// updated paths.
pub fn replace_client_item_proto(
    client_path: &str,
    generated_proto_path: &str,
) -> Result<Vec<String>, String> {
    let locale_dir = Path::new(client_path).join("locale");
    let entries = std::fs::read_dir(&locale_dir)
        .map_err(|e| format!("Locale-Ordner {locale_dir:?} nicht lesbar: {e}"))?;

    let mut updated = Vec::new();
    for entry in entries.flatten() {
        let path = entry.path();
        if !path.is_dir() {
            continue;
        }
        let target = path.join("item_proto");
        if !target.exists() {
            continue;
        }
        backup_file(&target)?;
        std::fs::copy(generated_proto_path, &target).map_err(|e| e.to_string())?;
        updated.push(target.display().to_string());
    }

    if updated.is_empty() {
        return Err(format!(
            "Keine item_proto-Datei unter {locale_dir:?}/<lang>/ gefunden."
        ));
    }
    Ok(updated)
}

#[cfg(test)]
mod tests {
    use super::*;

    // Uses a scratch temp dir standing in for the client folder - never
    // touches the real client, unlike the icon *read* tests in icons.rs
    // which intentionally verify against the real machine.
    #[test]
    fn writes_icon_and_backs_up_existing_file() {
        let scratch = std::env::temp_dir().join(format!(
            "m2manager_packtools_test_{}",
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_nanos()
        ));
        std::fs::create_dir_all(&scratch).expect("create scratch dir");
        let client_path = scratch.to_str().unwrap();

        // Real client icon file used purely as readable source image input.
        let source = r"C:\Users\DevSteven\Desktop\Client\pack\icon\icon\item\money.tga";
        let written = write_icon_tga(client_path, 500001, source).expect("write_icon_tga failed");
        assert!(written.exists());
        assert_eq!(written.file_name().unwrap(), "500001.tga");
        image::open(&written).expect("written tga should be readable");

        let backup = backup_file(&written).expect("backup_file failed");
        assert!(backup.is_some());
        assert!(backup.unwrap().exists());

        std::fs::remove_dir_all(&scratch).ok();
    }
}
