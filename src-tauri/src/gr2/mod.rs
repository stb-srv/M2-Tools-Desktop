// Metin2's .gr2 assets use an old Granny2 format/compression variant that
// modern 64-bit Granny builds don't recognize, and the client's own granny2.dll
// (which does) is a 32-bit DLL that can't be loaded into this 64-bit process.
// Bridged via a 32-bit sidecar binary (gr2-sidecar/) that loads the user's own
// client DLL and reports mesh data back as JSON. See m2manager-gr2-reference.

use serde::{Deserialize, Serialize};
use std::path::PathBuf;
use std::process::Command;

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct Mesh {
    pub name: String,
    pub is_rigid: bool,
    pub vertices: Vec<f32>,
    pub normals: Vec<f32>,
    pub uvs: Vec<f32>,
    pub indices: Vec<u32>,
    /// Texture path as recorded by the exporter (usually an absolute artist
    /// path), only meaningful for resolving the file name.
    #[serde(default)]
    pub texture_name: Option<String>,
    /// Decoded texture as a PNG data URL, filled in by `parse`.
    #[serde(default)]
    pub texture: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct ModelInfo {
    pub name: String,
    pub bone_count: i32,
    pub meshes: Vec<Mesh>,
    pub skipped_meshes: usize,
}

#[derive(Deserialize)]
struct SidecarOutput {
    ok: bool,
    data: Option<ModelInfo>,
    error: Option<String>,
}

fn sidecar_path() -> PathBuf {
    // Dev-time resolution only - production builds must bundle this via
    // Tauri's externalBin sidecar mechanism instead (not yet wired up).
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("gr2-sidecar")
        .join("target")
        .join("i686-pc-windows-msvc")
        .join("release")
        .join("gr2-sidecar.exe")
}

pub fn parse(granny_dll_path: &str, gr2_path: &str) -> Result<ModelInfo, String> {
    let sidecar = sidecar_path();
    if !sidecar.exists() {
        return Err(format!(
            "GR2-Sidecar nicht gefunden unter {}. Mit 'cargo build --release --target i686-pc-windows-msvc' im gr2-sidecar-Ordner bauen.",
            sidecar.display()
        ));
    }

    let output = Command::new(sidecar)
        .arg(granny_dll_path)
        .arg(gr2_path)
        .output()
        .map_err(|e| format!("GR2-Sidecar konnte nicht gestartet werden: {e}"))?;

    let stdout = String::from_utf8_lossy(&output.stdout);
    let parsed: SidecarOutput = serde_json::from_str(stdout.trim())
        .map_err(|e| format!("Ungültige Antwort vom GR2-Sidecar: {e} (stdout: {stdout})"))?;

    if !parsed.ok {
        return Err(parsed
            .error
            .unwrap_or_else(|| "Unbekannter Fehler im GR2-Sidecar".to_string()));
    }

    let mut model = parsed
        .data
        .ok_or_else(|| "GR2-Sidecar meldete Erfolg ohne Daten".to_string())?;

    // Textures live next to the .gr2 as .dds; a model without one still renders
    // untextured, so a failed lookup must not fail the whole load.
    for mesh in &mut model.meshes {
        mesh.texture =
            crate::textures::load_texture_data_url(gr2_path, mesh.texture_name.as_deref())
                .unwrap_or(None);
    }

    Ok(model)
}

/// Locates the client's NPC list. Not every client keeps it at the usual
/// `root/npclist.txt`, so fall back to scanning, and let the user point at it
/// explicitly if even that fails (some clients rename or relocate it).
pub fn find_npclist(client_path: &str, override_path: Option<&str>) -> Option<PathBuf> {
    if let Some(p) = override_path.filter(|p| !p.is_empty()) {
        let path = PathBuf::from(p);
        if path.is_file() {
            return Some(path);
        }
    }

    let default = std::path::Path::new(client_path)
        .join("root")
        .join("npclist.txt");
    if default.is_file() {
        return Some(default);
    }

    find_file_recursive(std::path::Path::new(client_path), "npclist.txt", 10)
}

fn find_file_recursive(dir: &std::path::Path, filename: &str, max_depth: u32) -> Option<PathBuf> {
    if max_depth == 0 {
        return None;
    }
    let entries = std::fs::read_dir(dir).ok()?;
    let mut subdirs = Vec::new();
    for entry in entries.flatten() {
        let path = entry.path();
        if path.is_dir() {
            subdirs.push(path);
        } else if path.file_name().and_then(|n| n.to_str()) == Some(filename) {
            return Some(path);
        }
    }
    subdirs
        .into_iter()
        .find_map(|d| find_file_recursive(&d, filename, max_depth - 1))
}

/// Shop NPCs generally have an empty `mob_proto.folder`; the client resolves
/// their models through npclist.txt instead, which maps NPC vnum to a model
/// folder name. Lines are tab-separated and either `vnum<TAB>folder` or
/// `vnum<TAB>name<TAB>folder`, so the folder is always the last field.
pub fn lookup_npc_folder(list_path: &std::path::Path, npc_vnum: i32) -> Option<String> {
    let contents = std::fs::read_to_string(list_path).ok()?;

    for line in contents.lines() {
        let fields: Vec<&str> = line.split('\t').map(|f| f.trim()).collect();
        if fields.len() < 2 {
            continue;
        }
        if fields[0].parse::<i32>().ok() == Some(npc_vnum) {
            let folder = fields[fields.len() - 1];
            if !folder.is_empty() {
                return Some(folder.to_string());
            }
        }
    }
    None
}

pub fn find_granny_dll(client_path: &str) -> Option<String> {
    let candidate = std::path::Path::new(client_path).join("granny2.dll");
    candidate.exists().then(|| candidate.to_string_lossy().into_owned())
}

/// Metin2 clients store each model under a folder named after the NPC/mob
/// (mob_proto.folder), but the parent tree ("pack/npc/ymir work/npc/...",
/// "pack/monster/ymir work/monster/...") isn't consistent across cores, so
/// search by folder name instead of assuming a fixed path shape.
pub fn find_npc_model(client_path: &str, folder: &str) -> Option<String> {
    find_recursive(std::path::Path::new(client_path), folder, 10)
}

fn find_recursive(dir: &std::path::Path, folder: &str, max_depth: u32) -> Option<String> {
    if max_depth == 0 {
        return None;
    }
    let entries = std::fs::read_dir(dir).ok()?;
    for entry in entries.flatten() {
        let path = entry.path();
        if !path.is_dir() {
            continue;
        }
        if path.file_name().and_then(|n| n.to_str()) == Some(folder) {
            let gr2 = path.join(format!("{folder}.gr2"));
            if gr2.exists() {
                return Some(gr2.to_string_lossy().into_owned());
            }
        }
        if let Some(found) = find_recursive(&path, folder, max_depth - 1) {
            return Some(found);
        }
    }
    None
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_real_alchemist_model() {
        let dll = r"C:\Users\DevSteven\Desktop\Client\granny2.dll";
        let gr2 = r"C:\Users\DevSteven\Desktop\Client\pack\npc\ymir work\npc\alchemist\alchemist.gr2";
        let result = parse(dll, gr2).expect("failed to parse alchemist.gr2 via sidecar");
        println!("model name: {:?}", result.name);
        println!("bone_count: {}", result.bone_count);
        println!("mesh count: {}", result.meshes.len());
        println!("skipped meshes: {}", result.skipped_meshes);
        assert!(result.meshes.len() + result.skipped_meshes > 0);
    }

    #[test]
    fn resolves_shop_npc_model_via_npclist() {
        let client = r"C:\Users\DevSteven\Desktop\Client";
        // 9001 = Waffenhaendler's NPC; its mob_proto.folder is empty in the DB,
        // so this must come from the client's own npclist.txt.
        let list = find_npclist(client, None).expect("npclist.txt not found");
        println!("npclist: {}", list.display());
        let folder = lookup_npc_folder(&list, 9001).expect("npclist lookup failed for 9001");
        println!("npc 9001 -> folder {folder:?}");
        assert_eq!(folder, "arms");

        let model = find_npc_model(client, &folder).expect("model file not found");
        println!("model path: {model}");

        let dll = find_granny_dll(client).expect("granny2.dll not found");
        let parsed = parse(&dll, &model).expect("failed to parse shop NPC model");
        println!(
            "meshes: {}, skipped: {}, bones: {}",
            parsed.meshes.len(),
            parsed.skipped_meshes,
            parsed.bone_count
        );
        assert!(!parsed.meshes.is_empty(), "shop NPC should yield geometry");
        assert!(parsed.meshes.iter().all(|m| !m.vertices.is_empty()));
    }
}
