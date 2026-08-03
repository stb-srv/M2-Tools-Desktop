use base64::Engine;
use std::path::{Path, PathBuf};

/// Granny stores the texture path as it was at export time (often an absolute
/// artist path like "d:\\ymir work\\npc\\arms\\arms.dds"), so only the file name
/// is usable - resolve it against the folder the .gr2 actually lives in.
fn resolve_texture_path(model_path: &str, texture_name: Option<&str>) -> Option<PathBuf> {
    let model = Path::new(model_path);
    let dir = model.parent()?;

    if let Some(name) = texture_name {
        let file = name.rsplit(['\\', '/']).next().unwrap_or(name);
        let candidate = dir.join(file);
        if candidate.exists() {
            return Some(candidate);
        }
    }

    // Fall back to a .dds sharing the model's base name, then to any single
    // .dds in the folder - Metin2 NPC folders usually hold exactly one.
    if let Some(stem) = model.file_stem() {
        let candidate = dir.join(format!("{}.dds", stem.to_string_lossy()));
        if candidate.exists() {
            return Some(candidate);
        }
    }

    let mut found = None;
    for entry in std::fs::read_dir(dir).ok()?.flatten() {
        let path = entry.path();
        if path.extension().and_then(|e| e.to_str()).map(|e| e.eq_ignore_ascii_case("dds"))
            == Some(true)
        {
            if found.is_some() {
                return None; // ambiguous, don't guess
            }
            found = Some(path);
        }
    }
    found
}

/// Decodes a `.dds` file (any format `image_dds` supports, e.g. the DXT1
/// minimap tiles under a map's grid folders) into an RGBA image. Shared by
/// NPC/model texture loading here and by `mapdata`'s minimap compositing.
pub fn decode_dds_file(path: &Path) -> Result<image::RgbaImage, String> {
    let bytes = std::fs::read(path).map_err(|e| format!("DDS nicht lesbar: {e}"))?;
    let dds = ddsfile::Dds::read(&mut std::io::Cursor::new(bytes))
        .map_err(|e| format!("DDS konnte nicht gelesen werden: {e}"))?;
    image_dds::image_from_dds(&dds, 0).map_err(|e| format!("DDS konnte nicht dekodiert werden: {e}"))
}

pub fn load_texture_data_url(
    model_path: &str,
    texture_name: Option<&str>,
) -> Result<Option<String>, String> {
    let Some(path) = resolve_texture_path(model_path, texture_name) else {
        return Ok(None);
    };

    let rgba = decode_dds_file(&path)?;

    let mut png_bytes = Vec::new();
    rgba.write_to(
        &mut std::io::Cursor::new(&mut png_bytes),
        image::ImageFormat::Png,
    )
    .map_err(|e| format!("Textur konnte nicht konvertiert werden: {e}"))?;

    let encoded = base64::engine::general_purpose::STANDARD.encode(png_bytes);
    Ok(Some(format!("data:image/png;base64,{encoded}")))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn decodes_npc_dds_texture() {
        let model = r"C:\Users\DevSteven\Desktop\Client\pack\npc\ymir work\npc\arms\arms.gr2";
        let url = load_texture_data_url(model, None)
            .expect("texture lookup failed")
            .expect("expected a texture next to arms.gr2");
        assert!(url.starts_with("data:image/png;base64,"));
        println!("arms texture data url: {} bytes", url.len());
    }
}
