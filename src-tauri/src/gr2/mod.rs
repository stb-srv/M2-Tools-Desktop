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
    pub vertices: Vec<f32>,
    pub normals: Vec<f32>,
    pub uvs: Vec<f32>,
    pub indices: Vec<u32>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct ModelInfo {
    pub name: String,
    pub bone_count: i32,
    pub meshes: Vec<Mesh>,
    pub skipped_skinned_meshes: usize,
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

    if parsed.ok {
        parsed
            .data
            .ok_or_else(|| "GR2-Sidecar meldete Erfolg ohne Daten".to_string())
    } else {
        Err(parsed
            .error
            .unwrap_or_else(|| "Unbekannter Fehler im GR2-Sidecar".to_string()))
    }
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
        println!("skipped skinned meshes: {}", result.skipped_skinned_meshes);
        assert!(result.meshes.len() + result.skipped_skinned_meshes > 0);
    }
}
