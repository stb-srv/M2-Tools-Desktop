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

/// Everything this module caches for one client path: the parsed
/// `item_list.txt` mapping, an index of every `.tga` under the client
/// (lower-cased file name -> first path found) and the already encoded
/// icons per vnum (`None` = no icon exists).
///
/// The index replaces the old per-icon recursive scan of the whole client
/// folder: on a fresh, still packed client *every* lookup missed and walked
/// tens of thousands of files, so icon lists took seconds and still stayed
/// empty. Now the folder is walked once per client path.
#[derive(Default)]
struct IconCaches {
    client_path: String,
    item_list: Option<HashMap<u32, String>>,
    tga_index: Option<HashMap<String, PathBuf>>,
    data_urls: HashMap<u32, Option<String>>,
}

/// Upper bound for cached encoded icons (a few KB each) so browsing every
/// item doesn't grow memory without limit.
const MAX_CACHED_ICONS: usize = 5000;

static CACHES: OnceLock<Mutex<IconCaches>> = OnceLock::new();

fn with_caches<T>(client_path: &str, f: impl FnOnce(&mut IconCaches) -> T) -> Option<T> {
    let cache = CACHES.get_or_init(|| Mutex::new(IconCaches::default()));
    let mut guard = cache.lock().ok()?;
    if guard.client_path != client_path {
        *guard = IconCaches {
            client_path: client_path.to_string(),
            ..Default::default()
        };
    }
    Some(f(&mut guard))
}

/// Drops every cached mapping, index and encoded icon. Called whenever icon
/// files or `item_list.txt` change (new/deleted icon, client unpacked).
pub fn invalidate_caches() {
    if let Some(cache) = CACHES.get() {
        if let Ok(mut guard) = cache.lock() {
            *guard = IconCaches::default();
        }
    }
}

/// Returns the icon path (relative to `pack/icon/`, e.g. "icon/item/money.tga")
/// for a vnum, per `item_list.txt`. Parses the file once per client path.
fn icon_path_for_vnum(client_path: &str, vnum: u32) -> Option<String> {
    with_caches(client_path, |c| {
        c.item_list
            .get_or_insert_with(|| {
                find_item_list(client_path)
                    .map(|p| parse_item_icon_map(&p))
                    .unwrap_or_default()
            })
            .get(&vnum)
            .cloned()
    })
    .flatten()
}

fn build_tga_index(dir: &Path, max_depth: u32, out: &mut HashMap<String, PathBuf>) {
    if max_depth == 0 {
        return;
    }
    let Ok(entries) = std::fs::read_dir(dir) else { return };
    let mut entries: Vec<_> = entries.flatten().map(|e| e.path()).collect();
    entries.sort();
    for path in entries {
        if path.is_dir() {
            build_tga_index(&path, max_depth - 1, out);
        } else if let Some(name) = path.file_name().and_then(|n| n.to_str()) {
            let lower = name.to_lowercase();
            if lower.ends_with(".tga") {
                out.entry(lower).or_insert(path);
            }
        }
    }
}

/// Looks up a `.tga` by file name anywhere in the client (case-insensitive,
/// like Windows itself), using the cached index.
fn find_tga_by_name(client_path: &str, filename: &str) -> Option<PathBuf> {
    with_caches(client_path, |c| {
        c.tga_index
            .get_or_insert_with(|| {
                let mut index = HashMap::new();
                build_tga_index(Path::new(client_path), 10, &mut index);
                index
            })
            .get(&filename.to_lowercase())
            .cloned()
    })
    .flatten()
}

fn find_icon_file(client_path: &str, vnum: u32) -> Option<PathBuf> {
    if let Some(rel) = icon_path_for_vnum(client_path, vnum) {
        let mapped = Path::new(client_path).join("pack").join("icon").join(&rel);
        if mapped.exists() {
            return Some(mapped);
        }
        if let Some(filename) = Path::new(&rel).file_name().and_then(|f| f.to_str()) {
            if let Some(found) = find_tga_by_name(client_path, filename) {
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
    find_tga_by_name(client_path, &filename)
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
    if let Some(Some(cached)) = with_caches(client_path, |c| c.data_urls.get(&vnum).cloned()) {
        return Ok(cached);
    }
    let result = match find_icon_file(client_path, vnum) {
        Some(path) => Some(encode_icon_data_url(&path)?),
        None => None,
    };
    with_caches(client_path, |c| {
        if c.data_urls.len() >= MAX_CACHED_ICONS {
            c.data_urls.clear();
        }
        c.data_urls.insert(vnum, result.clone());
    });
    Ok(result)
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
    fn icon_lookup_uses_case_insensitive_index_and_sees_new_files_after_invalidate() {
        let client = std::env::temp_dir().join(format!("m2m_icon_index_{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&client);
        let deep = client.join("pack").join("icon_extra").join("item");
        std::fs::create_dir_all(&deep).unwrap();
        let tga = |path: &Path| image::RgbaImage::new(2, 2).save_with_format(path, image::ImageFormat::Tga).unwrap();
        tga(&deep.join("00020.TGA"));
        let client_str = client.to_string_lossy().into_owned();

        // vnum 25 -> decade fallback 00020.tga, found via index despite upper-case name.
        assert!(load_item_icon_data_url(&client_str, 25).unwrap().is_some());
        assert!(load_item_icon_data_url(&client_str, 30).unwrap().is_none());

        // A file added later is only seen after the caches are dropped.
        tga(&deep.join("00030.tga"));
        assert!(load_item_icon_data_url(&client_str, 30).unwrap().is_none());
        invalidate_caches();
        assert!(load_item_icon_data_url(&client_str, 30).unwrap().is_some());
        let _ = std::fs::remove_dir_all(&client);
    }

    #[test]
    #[ignore = "braucht echte Client-Dateien (M2_TEST_CLIENT_DIR oder Client-Pfad aus M2Manager) - mit `cargo test -- --ignored` ausführen"]
    fn loads_exact_and_fallback_icons() {
        let client = &crate::test_support::client_dir().to_string_lossy().into_owned();

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
    #[ignore = "braucht echte Client-Dateien (M2_TEST_CLIENT_DIR oder Client-Pfad aus M2Manager) - mit `cargo test -- --ignored` ausführen"]
    fn loads_icons_with_non_vnum_filenames_via_item_list() {
        let client = &crate::test_support::client_dir().to_string_lossy().into_owned();

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
