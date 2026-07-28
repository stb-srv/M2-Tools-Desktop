use base64::Engine;
use std::path::{Path, PathBuf};

// Metin2 item icons ship as loose .tga files named by zero-padded vnum
// (e.g. "00010.tga" for vnum 10), extracted from the client's icon.epk pack
// archive. The exact parent path varies by how the client was unpacked, so
// search for it rather than assuming a fixed structure.
//
// Refine chains (Schwert+0..+9, vnum 10..19) share a single icon file named
// after the decade's base vnum (10) - there's no 00011.tga..00019.tga, only
// 00010.tga. Try the exact vnum first, then fall back to the decade base.
fn find_icon_file(client_path: &str, vnum: u32) -> Option<PathBuf> {
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

pub fn load_item_icon_data_url(client_path: &str, vnum: u32) -> Result<Option<String>, String> {
    let Some(path) = find_icon_file(client_path, vnum) else {
        return Ok(None);
    };

    let image = image::open(&path).map_err(|e| format!("Icon konnte nicht gelesen werden: {e}"))?;

    let mut png_bytes = Vec::new();
    image
        .write_to(
            &mut std::io::Cursor::new(&mut png_bytes),
            image::ImageFormat::Png,
        )
        .map_err(|e| format!("Icon konnte nicht konvertiert werden: {e}"))?;

    let encoded = base64::engine::general_purpose::STANDARD.encode(png_bytes);
    Ok(Some(format!("data:image/png;base64,{encoded}")))
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
}
