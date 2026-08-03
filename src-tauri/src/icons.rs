use base64::Engine;
use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::sync::{Mutex, OnceLock};

// Metin2 item icons ship as loose .tga files inside the client's icon.epk
// pack archive. Most are named by zero-padded vnum (e.g. "00010.tga" for
// vnum 10), but plenty are not (vnum 1 -> "money.tga", quest books ->
// "book_01.tga", boss-drop boxes -> "boss_box.tga", hair items ->
// "hairM_21_1.tga", ...), so guessing the filename from the vnum alone
// silently fails for those. The client ships the authoritative mapping in
// `locale/<lang>/item_list.txt` (tab-separated: vnum, type, icon path,
// optional model path) - parse and use that first, falling back to the old
// vnum-guessing heuristic only for vnums missing from the list.
fn find_item_list(client_path: &str) -> Option<PathBuf> {
    find_recursive(Path::new(client_path), "item_list.txt", 6)
}

fn parse_item_icon_map(path: &Path) -> HashMap<u32, String> {
    let bytes = std::fs::read(path).unwrap_or_default();
    let (text, _, _) = encoding_rs::WINDOWS_1252.decode(&bytes);
    let mut map = HashMap::new();
    for line in text.lines() {
        let mut fields = line.split('\t');
        let Some(vnum_str) = fields.next() else { continue };
        let Some(_item_type) = fields.next() else { continue };
        let Some(icon_path) = fields.next() else { continue };
        if let Ok(vnum) = vnum_str.trim().parse::<u32>() {
            let icon_path = icon_path.trim();
            if !icon_path.is_empty() {
                map.insert(vnum, icon_path.to_string());
            }
        }
    }
    map
}

static ICON_MAP_CACHE: OnceLock<Mutex<Option<(String, HashMap<u32, String>)>>> = OnceLock::new();

/// Returns the icon path (relative to `pack/icon/`, e.g. "icon/item/money.tga")
/// for a vnum, per `item_list.txt`. Parses the file once per client path and
/// caches the result; re-parses if the configured client path changes.
fn icon_path_for_vnum(client_path: &str, vnum: u32) -> Option<String> {
    let cache = ICON_MAP_CACHE.get_or_init(|| Mutex::new(None));
    let mut guard = cache.lock().ok()?;
    let needs_reload = guard.as_ref().map(|(p, _)| p != client_path).unwrap_or(true);
    if needs_reload {
        let map = find_item_list(client_path)
            .map(|p| parse_item_icon_map(&p))
            .unwrap_or_default();
        *guard = Some((client_path.to_string(), map));
    }
    guard.as_ref().and_then(|(_, map)| map.get(&vnum).cloned())
}

fn find_icon_file(client_path: &str, vnum: u32) -> Option<PathBuf> {
    if let Some(rel) = icon_path_for_vnum(client_path, vnum) {
        let mapped = Path::new(client_path).join("pack").join("icon").join(&rel);
        if mapped.exists() {
            return Some(mapped);
        }
        if let Some(filename) = Path::new(&rel).file_name().and_then(|f| f.to_str()) {
            if let Some(found) = find_recursive(Path::new(client_path), filename, 10) {
                return Some(found);
            }
        }
    }

    // Not listed in item_list.txt (or the list wasn't found) - fall back to
    // the vnum-guessing heuristic. Refine chains (Schwert+0..+9, vnum 10..19)
    // share a single icon file named after the decade's base vnum.
    try_icon_path(client_path, vnum).or_else(|| try_icon_path(client_path, (vnum / 10) * 10))
}

fn try_icon_path(client_path: &str, vnum: u32) -> Option<PathBuf> {
    let filename = format!("{vnum:05}.tga");
    let direct = Path::new(client_path)
        .join("pack")
        .join("icon")
        .join("icon")
        .join("item")
        .join(&filename);
    if direct.exists() {
        return Some(direct);
    }
    find_recursive(Path::new(client_path), &filename, 10)
}

fn find_recursive(dir: &Path, filename: &str, max_depth: u32) -> Option<PathBuf> {
    if max_depth == 0 {
        return None;
    }
    let entries = std::fs::read_dir(dir).ok()?;
    for entry in entries.flatten() {
        let path = entry.path();
        if path.is_dir() {
            if let Some(found) = find_recursive(&path, filename, max_depth - 1) {
                return Some(found);
            }
        } else if path.file_name().and_then(|n| n.to_str()) == Some(filename) {
            return Some(path);
        }
    }
    None
}

fn encode_icon_data_url(path: &Path) -> Result<String, String> {
    let image = image::open(path).map_err(|e| format!("Icon konnte nicht gelesen werden: {e}"))?;

    let mut png_bytes = Vec::new();
    image
        .write_to(
            &mut std::io::Cursor::new(&mut png_bytes),
            image::ImageFormat::Png,
        )
        .map_err(|e| format!("Icon konnte nicht konvertiert werden: {e}"))?;

    let encoded = base64::engine::general_purpose::STANDARD.encode(png_bytes);
    Ok(format!("data:image/png;base64,{encoded}"))
}

pub fn load_item_icon_data_url(client_path: &str, vnum: u32) -> Result<Option<String>, String> {
    let Some(path) = find_icon_file(client_path, vnum) else {
        return Ok(None);
    };
    encode_icon_data_url(&path).map(Some)
}

#[derive(serde::Serialize)]
pub struct IconFile {
    pub absolute_path: String,
    pub file_name: String,
}

fn collect_tga_files(dir: &Path, out: &mut Vec<IconFile>) {
    let Ok(entries) = std::fs::read_dir(dir) else {
        return;
    };
    for entry in entries.flatten() {
        let path = entry.path();
        if path.is_dir() {
            collect_tga_files(&path, out);
        } else if path
            .extension()
            .and_then(|e| e.to_str())
            .is_some_and(|e| e.eq_ignore_ascii_case("tga"))
        {
            let Some(file_name) = path.file_name().and_then(|n| n.to_str()) else {
                continue;
            };
            out.push(IconFile {
                absolute_path: path.to_string_lossy().into_owned(),
                file_name: file_name.to_string(),
            });
        }
    }
}

/// Lists every `.tga` file under the client's item-icon folder (recursively)
/// - used by the Icon Browser to let the user pick an icon by appearance
/// when creating a brand-new item, which has no vnum→icon mapping yet for
/// `item_list.txt`/vnum-guessing to resolve.
pub fn list_item_icon_files(client_path: &str) -> Vec<IconFile> {
    let base = Path::new(client_path)
        .join("pack")
        .join("icon")
        .join("icon")
        .join("item");
    let mut out = Vec::new();
    collect_tga_files(&base, &mut out);
    out.sort_by(|a, b| a.file_name.cmp(&b.file_name));
    out
}

pub fn load_icon_file_data_url(absolute_path: &str) -> Result<String, String> {
    encode_icon_data_url(Path::new(absolute_path))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn loads_exact_and_fallback_icons() {
        let client = r"C:\Users\DevSteven\Desktop\Client";

        let exact = load_item_icon_data_url(client, 10).expect("vnum 10 lookup failed");
        assert!(exact.is_some(), "expected direct icon for vnum 10");
        println!("vnum 10 (exact): {} bytes", exact.unwrap().len());

        let fallback = load_item_icon_data_url(client, 15).expect("vnum 15 lookup failed");
        assert!(fallback.is_some(), "expected decade-fallback icon for vnum 15");
        println!("vnum 15 (fallback to 10): {} bytes", fallback.unwrap().len());

        let missing = load_item_icon_data_url(client, 999999).expect("lookup should not error");
        assert!(missing.is_none());
    }

    #[test]
    fn loads_icons_with_non_vnum_filenames_via_item_list() {
        let client = r"C:\Users\DevSteven\Desktop\Client";

        // vnum 1 ("money") has no 00001.tga/00000.tga on disk - only resolvable
        // via item_list.txt's explicit "icon/item/money.tga" mapping.
        let money = load_item_icon_data_url(client, 1).expect("vnum 1 lookup failed");
        assert!(money.is_some(), "expected money.tga icon for vnum 1 via item_list.txt");

        // vnum 50070 ("boss_box") - same story, named icon not derivable from vnum.
        let boss_box = load_item_icon_data_url(client, 50070).expect("vnum 50070 lookup failed");
        assert!(
            boss_box.is_some(),
            "expected boss_box.tga icon for vnum 50070 via item_list.txt"
        );
    }
}
