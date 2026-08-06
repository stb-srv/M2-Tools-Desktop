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

/// Finds the weapon `.gr2` model file for `vnum`. Metin2 weapon models are
/// named by plain vnum (`{vnum:05}.gr2`) under
/// `pack/item/ymir work/item/weapon/`, and - exactly like item icons
/// (see icons.rs) - a weapon's refine chain (vnum..vnum+9) shares a single
/// model file named after the decade's base vnum, so refine levels fall
/// back to that if there's no exact-vnum file.
fn find_weapon_model(client_path: &str, vnum: u32) -> Option<PathBuf> {
    let base = Path::new(client_path)
        .join("pack")
        .join("item")
        .join("ymir work")
        .join("item")
        .join("weapon");
    let exact = base.join(format!("{vnum:05}.gr2"));
    if exact.exists() {
        return Some(exact);
    }
    let decade = base.join(format!("{:05}.gr2", (vnum / 10) * 10));
    if decade.exists() {
        return Some(decade);
    }
    None
}

/// Copies the weapon model used by `source_vnum` so `new_vnum` renders
/// identically in-game. There is no such per-vnum file convention for
/// armor (body models are swapped per race/gender inside the much larger
/// `pc_*` character model trees), so this only covers weapons.
pub fn copy_weapon_model(client_path: &str, new_vnum: u32, source_vnum: u32) -> Result<PathBuf, String> {
    let source = find_weapon_model(client_path, source_vnum)
        .ok_or_else(|| format!("Kein Waffen-Modell für vnum {source_vnum} gefunden."))?;
    let dest = Path::new(client_path)
        .join("pack")
        .join("item")
        .join("ymir work")
        .join("item")
        .join("weapon")
        .join(format!("{new_vnum:05}.gr2"));
    backup_file(&dest)?;
    std::fs::copy(&source, &dest).map_err(|e| e.to_string())?;
    Ok(dest)
}

/// Sanitizes a module/folder name for use as a filesystem path segment -
/// module names come from a user-picked folder (e.g. "FrostEdge") and get
/// embedded directly into a client filesystem path below.
fn sanitize_path_segment(name: &str) -> String {
    let cleaned: String = name
        .chars()
        .map(|c| if c.is_alphanumeric() || c == '-' || c == '_' { c } else { '_' })
        .collect();
    if cleaned.is_empty() {
        "modul".to_string()
    } else {
        cleaned
    }
}

/// Copies `sources` (sibling texture files next to a `.gr2`) alongside
/// `dest_dir`, skipping any whose filename collides with `skip_name` (the
/// model file itself, already copied by the caller). Backs up any file
/// being overwritten. Returns the copied destination paths.
fn copy_sibling_textures(dest_dir: &Path, sources: &[String], skip_name: &str) -> Result<Vec<PathBuf>, String> {
    let mut copied = Vec::new();
    for src in sources {
        let src_path = Path::new(src);
        let Some(name) = src_path.file_name() else { continue };
        if name.to_str() == Some(skip_name) {
            continue;
        }
        let dest = dest_dir.join(name);
        backup_file(&dest)?;
        std::fs::copy(src_path, &dest).map_err(|e| e.to_string())?;
        copied.push(dest);
    }
    Ok(copied)
}

/// Imports a brand-new (not vnum-cloned) weapon `.gr2` model shipped inside
/// a dropped-in asset module (see `modulescan.rs`) into the client's own
/// `pack/item/ymir work/item/weapon/` tree, namespaced under
/// `custom/<module_name>/` so multiple imported modules can't collide with
/// each other or with stock vnum-named files. The `.gr2` itself is renamed
/// to `{vnum}.gr2` on copy - matching the stock naming convention (see
/// `find_weapon_model`) instead of keeping the module author's own
/// filename (`fe_sword.gr2`, `2h.gr2`, ...), which is what a user actually
/// browsing the folder expects to see and is consistent with icons already
/// being written as `{vnum}.tga`. This is safe: `textures.rs`'s texture
/// resolver looks up a `.dds` by filename in the *model's own folder*, not
/// by any path baked into the `.gr2` itself, so renaming the model file
/// doesn't affect texture lookup - confirmed against this project's own
/// GR2 parser, and empirically true for the real modules this was tested
/// against (2026-08-06). `texture_sources` are sibling `.dds` files the
/// model references by same-directory filename - copied unrenamed
/// alongside, since their exact original filename is what both the
/// model's material and `textures.rs`'s resolver expect. Returns the
/// physical model destination path and the virtual `d:/ymir work/...`
/// path string that `upsert_item_list_entries` needs for its model-path
/// column - this is the only thing that actually makes the client render
/// the model (see that function's doc comment).
pub fn import_custom_weapon_model(
    client_path: &str,
    module_name: &str,
    vnum: u32,
    source_abs: &Path,
    texture_sources: &[String],
) -> Result<(PathBuf, String), String> {
    let source_file_name = source_abs
        .file_name()
        .and_then(|f| f.to_str())
        .ok_or("Ungültiger Modell-Dateiname")?;
    let dest_file_name = format!("{vnum}.gr2");
    let safe_module = sanitize_path_segment(module_name);

    let dest_dir = Path::new(client_path)
        .join("pack")
        .join("item")
        .join("ymir work")
        .join("item")
        .join("weapon")
        .join("custom")
        .join(&safe_module);
    std::fs::create_dir_all(&dest_dir).map_err(|e| e.to_string())?;

    let dest = dest_dir.join(&dest_file_name);
    backup_file(&dest)?;
    std::fs::copy(source_abs, &dest).map_err(|e| e.to_string())?;
    // Skip a texture whose name collides with the *source* model's own
    // filename (already copied above under its new name) - relevant only
    // for the rare module that ships a texture literally named e.g.
    // `sword.gr2.dds`-style after its own model, not a real-world case
    // seen so far, but keeps the guard meaningful now that dest/source
    // filenames differ.
    copy_sibling_textures(&dest_dir, texture_sources, source_file_name)?;

    let virtual_path = format!("d:/ymir work/item/weapon/custom/{safe_module}/{dest_file_name}");
    Ok((dest, virtual_path))
}

/// Imports a brand-new female armor body model into the client's
/// `pack/pc_<race>/ymir work/pc2/<race>/` tree, namespaced under
/// `custom/<module_name>/` like the weapon import above. Unlike weapons,
/// wiring this in isn't just a file copy - the returned `model_rel`/
/// `skin_rel` (paths relative to the `.msm`'s own `PathName`, i.e. relative
/// to this same destination folder) must also be written into a new
/// `ShapeData` entry by `msm::add_shape_data`, which is the caller's job.
/// See `msm.rs`'s module doc for why this has no male equivalent.
pub fn import_custom_armor_model(
    client_path: &str,
    module_name: &str,
    race: &str,
    source_abs: &Path,
    texture_sources: &[String],
) -> Result<(PathBuf, String, String), String> {
    let file_name = source_abs
        .file_name()
        .and_then(|f| f.to_str())
        .ok_or("Ungültiger Modell-Dateiname")?;
    let safe_module = sanitize_path_segment(module_name);

    let dest_dir = Path::new(client_path)
        .join("pack")
        .join(format!("pc_{race}"))
        .join("ymir work")
        .join("pc2")
        .join(race)
        .join("custom")
        .join(&safe_module);
    std::fs::create_dir_all(&dest_dir).map_err(|e| e.to_string())?;

    let dest = dest_dir.join(file_name);
    backup_file(&dest)?;
    std::fs::copy(source_abs, &dest).map_err(|e| e.to_string())?;
    let copied_textures = copy_sibling_textures(&dest_dir, texture_sources, file_name)?;

    let model_rel = format!("custom/{safe_module}/{file_name}");
    // SourceSkin/TargetSkin must be identical for a brand-new model (no
    // recolor remap needed) - use the first copied texture, if any; a
    // model with no separate texture file (skin baked into the .gr2, rare
    // but seen in some packs) falls back to the model's own filename so the
    // ShapeData entry is still syntactically complete.
    let skin_name = copied_textures
        .first()
        .and_then(|p| p.file_name())
        .and_then(|f| f.to_str())
        .unwrap_or(file_name);
    let skin_rel = format!("custom/{safe_module}/{skin_name}");

    Ok((dest, model_rel, skin_rel))
}

/// Copies an entire glow-effect bundle (a folder of `.mse`/`.mde`/`.dds`
/// files that reference each other by filename, so they must stay
/// together - see `modulescan.rs`'s `effect_dirs`) into the client's
/// `pack/effect/ymir work/effect/` tree, namespaced the same way as
/// `import_custom_weapon_model`. Unlike weapon models, there is no
/// `item_list.txt`-style DB/text wiring for effects - the engine derives
/// glow effects contextually, so this is a best-effort copy-through only;
/// verify visually in-client after importing.
pub fn import_effect_bundle(
    client_path: &str,
    module_name: &str,
    source_dir: &Path,
) -> Result<Vec<PathBuf>, String> {
    let dir_name = source_dir
        .file_name()
        .and_then(|f| f.to_str())
        .ok_or("Ungültiger Effekt-Ordnername")?;
    let safe_module = sanitize_path_segment(module_name);

    let dest_dir = Path::new(client_path)
        .join("pack")
        .join("effect")
        .join("ymir work")
        .join("effect")
        .join("custom")
        .join(&safe_module)
        .join(dir_name);
    std::fs::create_dir_all(&dest_dir).map_err(|e| e.to_string())?;

    let entries = std::fs::read_dir(source_dir)
        .map_err(|e| format!("Effekt-Ordner {source_dir:?} nicht lesbar: {e}"))?;

    let mut copied = Vec::new();
    for entry in entries.flatten() {
        let path = entry.path();
        if !path.is_file() {
            continue;
        }
        let Some(name) = path.file_name() else { continue };
        let dest = dest_dir.join(name);
        backup_file(&dest)?;
        std::fs::copy(&path, &dest).map_err(|e| e.to_string())?;
        copied.push(dest);
    }
    Ok(copied)
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

/// Deletes the icon `.tga` written by `write_icon_tga` for `vnum`, backing
/// it up first. Used when fully undoing an imported item - see
/// `commands::undo_import_batch`. Returns `None` if there was nothing to
/// delete (e.g. the variant never got an icon in the first place).
pub fn delete_item_icon(client_path: &str, vnum: u32) -> Result<Option<PathBuf>, String> {
    let path = Path::new(client_path)
        .join("pack")
        .join("icon")
        .join("icon")
        .join("item")
        .join(format!("{vnum:05}.tga"));
    if !path.exists() {
        return Ok(None);
    }
    backup_file(&path)?;
    std::fs::remove_file(&path).map_err(|e| e.to_string())?;
    Ok(Some(path))
}

/// Removes `vnum`'s row from every `locale/<lang>/item_list.txt` - the
/// reverse of `upsert_item_list_entries`. Used when fully undoing an
/// imported item: without this the client would keep trying to resolve an
/// icon/model for a vnum that no longer has an `item_proto` row at all.
/// A missing row (nothing to remove) is not an error - the item may never
/// have gotten a row in the first place (e.g. an armor piece with no icon).
pub fn remove_item_list_entries(client_path: &str, vnum: u32) -> Result<Vec<String>, String> {
    let locale_dir = Path::new(client_path).join("locale");
    let entries = std::fs::read_dir(&locale_dir)
        .map_err(|e| format!("Locale-Ordner {locale_dir:?} nicht lesbar: {e}"))?;

    let mut updated = Vec::new();
    for entry in entries.flatten() {
        let path = entry.path();
        if !path.is_dir() {
            continue;
        }
        let target = path.join("item_list.txt");
        if !target.exists() {
            continue;
        }

        let bytes = std::fs::read(&target).map_err(|e| e.to_string())?;
        let (text, _, _) = encoding_rs::WINDOWS_1252.decode(&bytes);

        let mut found = false;
        let lines: Vec<&str> = text
            .lines()
            .filter(|line| {
                let is_target_vnum = line
                    .split('\t')
                    .next()
                    .and_then(|v| v.trim().parse::<u32>().ok())
                    == Some(vnum);
                if is_target_vnum {
                    found = true;
                }
                !is_target_vnum
            })
            .collect();
        if !found {
            continue;
        }

        backup_file(&target)?;
        let mut out = lines.join("\r\n");
        if !lines.is_empty() {
            out.push_str("\r\n");
        }
        let (encoded, _, _) = encoding_rs::WINDOWS_1252.encode(&out);
        std::fs::write(&target, encoded).map_err(|e| e.to_string())?;
        updated.push(target.display().to_string());
    }
    Ok(updated)
}

fn item_type_label(item_type: u32) -> &'static str {
    match item_type {
        1 => "WEAPON",
        2 => "ARMOR",
        _ => "ETC",
    }
}

/// `locale/<lang>/item_list.txt` (tab-separated: vnum, TYPE, icon path,
/// optional model path) is the client's *authoritative* vnum -> icon/model
/// mapping - confirmed live: unlike this tool's own preview reader
/// (icons.rs), the actual game client does not fall back to guessing the
/// icon/model filename from the vnum, so a brand-new custom vnum's icon
/// and/or weapon model silently fail to render until a row exists here.
/// Inserts a new row for `vnum`, or replaces its existing row if one is
/// already present (e.g. re-running the pipeline after picking a
/// different icon), in every `locale/<lang>/item_list.txt`.
pub fn upsert_item_list_entries(
    client_path: &str,
    vnum: u32,
    item_type: u32,
    icon_rel_path: &str,
    model_rel_path: Option<&str>,
) -> Result<Vec<String>, String> {
    let locale_dir = Path::new(client_path).join("locale");
    let entries = std::fs::read_dir(&locale_dir)
        .map_err(|e| format!("Locale-Ordner {locale_dir:?} nicht lesbar: {e}"))?;

    let mut new_line = format!("{vnum}\t{}\t{icon_rel_path}", item_type_label(item_type));
    if let Some(model) = model_rel_path {
        new_line.push('\t');
        new_line.push_str(model);
    }

    let mut updated = Vec::new();
    for entry in entries.flatten() {
        let path = entry.path();
        if !path.is_dir() {
            continue;
        }
        let target = path.join("item_list.txt");
        if !target.exists() {
            continue;
        }
        backup_file(&target)?;

        let bytes = std::fs::read(&target).map_err(|e| e.to_string())?;
        let (text, _, _) = encoding_rs::WINDOWS_1252.decode(&bytes);

        let mut found = false;
        let mut lines: Vec<String> = text
            .lines()
            .map(|line| {
                let is_target_vnum = line
                    .split('\t')
                    .next()
                    .and_then(|v| v.trim().parse::<u32>().ok())
                    == Some(vnum);
                if is_target_vnum {
                    found = true;
                    new_line.clone()
                } else {
                    line.to_string()
                }
            })
            .collect();
        if !found {
            lines.push(new_line.clone());
        }

        let mut out = lines.join("\r\n");
        out.push_str("\r\n");
        let (encoded, _, _) = encoding_rs::WINDOWS_1252.encode(&out);
        std::fs::write(&target, encoded).map_err(|e| e.to_string())?;
        updated.push(target.display().to_string());
    }

    if updated.is_empty() {
        return Err(format!(
            "Keine item_list.txt-Datei unter {locale_dir:?}/<lang>/ gefunden."
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

    #[test]
    fn copies_weapon_model_with_decade_fallback() {
        let scratch = std::env::temp_dir().join(format!(
            "m2manager_packtools_model_test_{}",
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_nanos()
        ));
        let weapon_dir = scratch
            .join("pack")
            .join("item")
            .join("ymir work")
            .join("item")
            .join("weapon");
        std::fs::create_dir_all(&weapon_dir).expect("create scratch weapon dir");
        let client_path = scratch.to_str().unwrap();

        // Stand-in weapon model: content doesn't matter, only that the file
        // exists at the vnum-named convention copy_weapon_model expects.
        std::fs::write(weapon_dir.join("00010.gr2"), b"fake gr2 data").expect("seed model file");

        // Exact-vnum match: vnum 10 has an exact 00010.gr2 on disk.
        let dest = copy_weapon_model(client_path, 500001, 10).expect("exact-match copy failed");
        assert!(dest.exists());
        assert_eq!(dest.file_name().unwrap(), "500001.gr2");

        // Decade fallback: vnum 15 (a refine level) has no 00015.gr2, only
        // the decade base 00010.gr2 - mirrors the icon fallback in icons.rs.
        let dest2 = copy_weapon_model(client_path, 500002, 15).expect("decade-fallback copy failed");
        assert!(dest2.exists());

        // Missing vnum (no exact or decade file) must fail loudly, not
        // silently skip the model.
        let missing = copy_weapon_model(client_path, 500003, 9999999);
        assert!(missing.is_err());

        std::fs::remove_dir_all(&scratch).ok();
    }

    #[test]
    fn upserts_item_list_entry_insert_and_update() {
        let scratch = std::env::temp_dir().join(format!(
            "m2manager_packtools_itemlist_test_{}",
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_nanos()
        ));
        let locale_dir = scratch.join("locale").join("de");
        std::fs::create_dir_all(&locale_dir).expect("create scratch locale dir");
        let client_path = scratch.to_str().unwrap();

        // Seed with the real file's exact format: tab-separated, CRLF,
        // Windows-1252 encoded.
        let seed = "1\tETC\ticon/item/money.tga\td:/ymir work/item/etc/money.gr2\r\n\
                    10\tWEAPON\ticon/item/00010.tga\td:/ymir work/item/weapon/00010.gr2\r\n";
        let (encoded, _, _) = encoding_rs::WINDOWS_1252.encode(seed);
        std::fs::write(locale_dir.join("item_list.txt"), encoded).expect("seed item_list.txt");

        // Insert: brand-new vnum gets appended.
        let updated = upsert_item_list_entries(
            client_path,
            500001,
            1,
            "icon/item/500001.tga",
            Some("d:/ymir work/item/weapon/500001.gr2"),
        )
        .expect("insert failed");
        assert_eq!(updated.len(), 1);
        let bytes = std::fs::read(&updated[0]).unwrap();
        let (text, _, _) = encoding_rs::WINDOWS_1252.decode(&bytes);
        assert!(text.contains("500001\tWEAPON\ticon/item/500001.tga\td:/ymir work/item/weapon/500001.gr2"));
        assert!(text.contains("10\tWEAPON\ticon/item/00010.tga")); // untouched existing row

        // Update: re-running for the same vnum replaces its row, not
        // duplicates it (e.g. picking a different icon and re-running).
        upsert_item_list_entries(client_path, 500001, 1, "icon/item/other.tga", None)
            .expect("update failed");
        let bytes = std::fs::read(&updated[0]).unwrap();
        let (text, _, _) = encoding_rs::WINDOWS_1252.decode(&bytes);
        assert_eq!(text.matches("500001\t").count(), 1);
        assert!(text.contains("500001\tWEAPON\ticon/item/other.tga"));
        assert!(!text.contains("500001.gr2"));

        std::fs::remove_dir_all(&scratch).ok();
    }

    #[test]
    fn imports_custom_weapon_model_renamed_by_vnum() {
        let scratch = std::env::temp_dir().join(format!(
            "m2manager_packtools_custommodel_test_{}",
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_nanos()
        ));
        std::fs::create_dir_all(&scratch).expect("create scratch dir");
        let client_path = scratch.to_str().unwrap();

        let source_dir = scratch.join("source_module");
        std::fs::create_dir_all(&source_dir).expect("create source dir");
        let source_file = source_dir.join("fe_sword.gr2");
        std::fs::write(&source_file, b"fake gr2 data").expect("seed source model");
        let texture_file = source_dir.join("fe_sword.dds");
        std::fs::write(&texture_file, b"fake dds data").expect("seed source texture");
        let textures = vec![texture_file.display().to_string()];

        let (dest, virtual_path) =
            import_custom_weapon_model(client_path, "FrostEdge", 700000, &source_file, &textures)
                .expect("import failed");
        assert!(dest.exists());
        // Renamed to {vnum}.gr2 - not the module author's own filename - so
        // browsing the folder shows what item it belongs to at a glance,
        // consistent with icons already being named {vnum}.tga.
        assert_eq!(
            virtual_path,
            "d:/ymir work/item/weapon/custom/FrostEdge/700000.gr2"
        );
        assert!(dest.to_string_lossy().replace('\\', "/").ends_with(
            "pack/item/ymir work/item/weapon/custom/FrostEdge/700000.gr2"
        ));
        // The texture must still be copied under its OWN original filename
        // (fe_sword.dds, not 700000.dds) - textures.rs resolves it by exact
        // filename in the model's folder, and the .gr2's internal material
        // reference still points at that original name regardless of what
        // the .gr2 file itself was renamed to.
        let copied_texture = dest.parent().unwrap().join("fe_sword.dds");
        assert!(copied_texture.exists(), "sibling texture must keep its original filename");

        // Re-importing (e.g. re-running the pipeline) must back up, not fail.
        let (dest2, _) =
            import_custom_weapon_model(client_path, "FrostEdge", 700000, &source_file, &textures)
                .expect("re-import failed");
        assert_eq!(dest, dest2);

        std::fs::remove_dir_all(&scratch).ok();
    }

    #[test]
    fn imports_custom_armor_model_and_returns_msm_relative_paths() {
        let scratch = std::env::temp_dir().join(format!(
            "m2manager_packtools_armormodel_test_{}",
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_nanos()
        ));
        std::fs::create_dir_all(&scratch).expect("create scratch dir");
        let client_path = scratch.to_str().unwrap();

        let source_dir = scratch.join("source_module");
        std::fs::create_dir_all(&source_dir).expect("create source dir");
        let source_file = source_dir.join("warrior_body.gr2");
        std::fs::write(&source_file, b"fake gr2 data").expect("seed source model");
        let texture_file = source_dir.join("warrior_body.dds");
        std::fs::write(&texture_file, b"fake dds data").expect("seed source texture");
        let textures = vec![texture_file.display().to_string()];

        let (dest, model_rel, skin_rel) =
            import_custom_armor_model(client_path, "Jotunthrym", "warrior", &source_file, &textures)
                .expect("import failed");
        assert!(dest.exists());
        assert!(dest.to_string_lossy().replace('\\', "/").ends_with(
            "pack/pc_warrior/ymir work/pc2/warrior/custom/Jotunthrym/warrior_body.gr2"
        ));
        assert_eq!(model_rel, "custom/Jotunthrym/warrior_body.gr2");
        assert_eq!(skin_rel, "custom/Jotunthrym/warrior_body.dds");
        assert!(dest.parent().unwrap().join("warrior_body.dds").exists());

        std::fs::remove_dir_all(&scratch).ok();
    }

    #[test]
    fn imports_effect_bundle_preserving_folder_contents() {
        let scratch = std::env::temp_dir().join(format!(
            "m2manager_packtools_effectbundle_test_{}",
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_nanos()
        ));
        std::fs::create_dir_all(&scratch).expect("create scratch dir");
        let client_path = scratch.to_str().unwrap();

        let source_dir = scratch.join("frost_edge");
        std::fs::create_dir_all(&source_dir).expect("create effect source dir");
        std::fs::write(source_dir.join("fe_sword_glow.mse"), b"mse").expect("seed mse");
        std::fs::write(source_dir.join("fe_sword_glow.mde"), b"mde").expect("seed mde");
        std::fs::write(source_dir.join("fe_sword.dds"), b"dds").expect("seed dds");

        let copied =
            import_effect_bundle(client_path, "FrostEdge", &source_dir).expect("import_effect_bundle failed");
        assert_eq!(copied.len(), 3);
        for f in &copied {
            assert!(f.exists());
        }
        let dest_dir = Path::new(client_path)
            .join("pack")
            .join("effect")
            .join("ymir work")
            .join("effect")
            .join("custom")
            .join("FrostEdge")
            .join("frost_edge");
        assert!(dest_dir.join("fe_sword_glow.mse").exists());
        assert!(dest_dir.join("fe_sword.dds").exists());

        std::fs::remove_dir_all(&scratch).ok();
    }

    #[test]
    fn deletes_item_icon_and_backs_up_first() {
        let scratch = std::env::temp_dir().join(format!(
            "m2manager_packtools_deleteicon_test_{}",
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_nanos()
        ));
        std::fs::create_dir_all(&scratch).expect("create scratch dir");
        let client_path = scratch.to_str().unwrap();

        let source = r"C:\Users\DevSteven\Desktop\Client\pack\icon\icon\item\money.tga";
        write_icon_tga(client_path, 500001, source).expect("seed icon");

        let deleted = delete_item_icon(client_path, 500001).expect("delete failed");
        assert!(deleted.is_some());
        assert!(!deleted.unwrap().exists());

        // Nothing to delete for a vnum that never got an icon - not an error.
        let missing = delete_item_icon(client_path, 999999).expect("delete of missing icon failed");
        assert!(missing.is_none());

        std::fs::remove_dir_all(&scratch).ok();
    }

    #[test]
    fn removes_item_list_entry_leaving_others_untouched() {
        let scratch = std::env::temp_dir().join(format!(
            "m2manager_packtools_removeitemlist_test_{}",
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_nanos()
        ));
        let locale_dir = scratch.join("locale").join("de");
        std::fs::create_dir_all(&locale_dir).expect("create scratch locale dir");
        let client_path = scratch.to_str().unwrap();

        let seed = "1\tETC\ticon/item/money.tga\r\n\
                    500001\tWEAPON\ticon/item/500001.tga\td:/ymir work/item/weapon/custom/FrostEdge/fe_sword.gr2\r\n\
                    500002\tWEAPON\ticon/item/500002.tga\r\n";
        let (encoded, _, _) = encoding_rs::WINDOWS_1252.encode(seed);
        std::fs::write(locale_dir.join("item_list.txt"), encoded).expect("seed item_list.txt");

        let updated = remove_item_list_entries(client_path, 500001).expect("remove failed");
        assert_eq!(updated.len(), 1);

        let bytes = std::fs::read(&updated[0]).unwrap();
        let (text, _, _) = encoding_rs::WINDOWS_1252.decode(&bytes);
        assert!(!text.contains("500001\t"));
        assert!(text.contains("1\tETC\ticon/item/money.tga"));
        assert!(text.contains("500002\tWEAPON\ticon/item/500002.tga"));

        // Removing a vnum with no row is a no-op, not an error.
        let noop = remove_item_list_entries(client_path, 999999).expect("no-op remove failed");
        assert!(noop.is_empty());

        std::fs::remove_dir_all(&scratch).ok();
    }
}
