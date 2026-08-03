use crate::textures;
use base64::Engine;
use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};

// Client map folder format verified against a real client install
// (pack\map_empire\metin2_map_a1\...): each map folder has a `setting.txt`
// (CellScale/ViewRadius/MapSize) and a grid of subfolders named
// "<x:03><y:03>" (e.g. "003002"), each holding a 256x256 DXT1 `minimap.dds`.
//
// Coordinate scale for regen spawns was verified (twice - the first attempt
// was wrong, see below) against a real 939-spawn file
// (`map/Map1/Blau/regen.txt`, paired with `map_empire/metin2_map_b1`):
//
// - First attempt used `CellScale(200) * ViewRadius(128) / 256px = 100`
//   world units per pixel, by analogy with the server's quest scripts'
//   `pc.warp(x*100, y*100, ...)` convention. This was wrong: it crammed all
//   939 (all-unique, clearly hand/tool-placed, not duplicated) spawn
//   positions into an ~8x12 pixel corner of the 1024x1280 composite -
//   nonsensical for a capital city's monster population.
// - Re-checked by bucketing the real coordinates: the 939 unique positions
//   spread smoothly across raw x in [75, 800ish] and y in [301, 1200ish] -
//   which lines up almost exactly with the 1024x1280 composite with NO
//   division at all. Conclusion: regen.txt coordinates are already in the
//   same pixel-equivalent space as the composited minimap (presumably
//   because map-authoring tools place spawns by reading position directly
//   off that same minimap image), unlike hand-typed quest-script warp
//   targets which need the engine's own *100 conversion. So
//   `units_per_pixel` is 1.0 - CellScale/ViewRadius turned out to answer a
//   different question (DDS tile pixel size) and don't apply here.
//
// `setting.txt` also has a `BasePosition` field; a separate live check
// showed subtracting it pushes coordinates negative (off-canvas), so it's
// deliberately not parsed - see git history for that diagnostic.

#[derive(Debug, Clone, PartialEq)]
pub struct MapSetting {
    pub map_size: (u32, u32),
}

impl MapSetting {
    pub fn units_per_pixel(&self) -> f64 {
        1.0
    }
}

pub fn parse_setting(content: &str) -> Result<MapSetting, String> {
    let mut map_size = None;

    for line in content.lines() {
        let tokens: Vec<&str> = line.split_whitespace().collect();
        let Some((key, values)) = tokens.split_first() else {
            continue;
        };
        match *key {
            "MapSize" if values.len() >= 2 => {
                map_size = values[0].parse().ok().zip(values[1].parse().ok())
            }
            _ => {}
        }
    }

    Ok(MapSetting {
        map_size: map_size.ok_or_else(|| "setting.txt: MapSize fehlt".to_string())?,
    })
}

/// Stitches every cell's `minimap.dds` into one composite image. Missing
/// cells (water/edge of the map) are left transparent rather than treated as
/// an error - not every grid slot in `MapSize` necessarily has a folder.
pub fn build_composite(map_dir: &Path, setting: &MapSetting) -> Result<image::RgbaImage, String> {
    let (cols, rows) = setting.map_size;
    let mut tile_size: Option<(u32, u32)> = None;
    let mut tiles: Vec<(u32, u32, image::RgbaImage)> = Vec::new();

    for y in 0..rows {
        for x in 0..cols {
            let minimap_path = map_dir.join(format!("{x:03}{y:03}")).join("minimap.dds");
            if !minimap_path.exists() {
                continue;
            }
            let tile = textures::decode_dds_file(&minimap_path)?;
            if tile_size.is_none() {
                tile_size = Some((tile.width(), tile.height()));
            }
            tiles.push((x, y, tile));
        }
    }

    let (tile_w, tile_h) = tile_size.ok_or_else(|| {
        format!(
            "Keine minimap.dds-Kacheln in {} gefunden.",
            map_dir.display()
        )
    })?;

    let mut composite = image::RgbaImage::new(cols * tile_w, rows * tile_h);
    for (x, y, tile) in tiles {
        image::imageops::overlay(&mut composite, &tile, (x * tile_w) as i64, (y * tile_h) as i64);
    }

    Ok(composite)
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MapFolderInfo {
    pub category: String,
    pub folder_name: String,
}

/// Scans `<client_path>\pack\map_*` for already-unpacked map folders (a
/// subfolder counts as a map if it has a `setting.txt`). Packed-only
/// categories (e.g. `map_dungeon` on a client where it hasn't been unpacked
/// yet) are silently skipped rather than erroring - unpacking is out of
/// scope for this feature.
pub fn list_map_folders(client_path: &str) -> Result<Vec<MapFolderInfo>, String> {
    let pack_dir = Path::new(client_path).join("pack");
    let entries = std::fs::read_dir(&pack_dir)
        .map_err(|e| format!("pack-Ordner nicht lesbar unter {}: {e}", pack_dir.display()))?;

    let mut result = Vec::new();
    for entry in entries.flatten() {
        let path = entry.path();
        let Some(category) = path.file_name().and_then(|n| n.to_str()).map(str::to_string) else {
            continue;
        };
        if !path.is_dir() || !category.starts_with("map_") {
            continue;
        }

        let Ok(sub_entries) = std::fs::read_dir(&path) else {
            continue;
        };
        for sub in sub_entries.flatten() {
            let sub_path = sub.path();
            if sub_path.is_dir() && sub_path.join("setting.txt").exists() {
                if let Some(folder_name) = sub_path.file_name().and_then(|n| n.to_str()) {
                    result.push(MapFolderInfo {
                        category: category.clone(),
                        folder_name: folder_name.to_string(),
                    });
                }
            }
        }
    }

    result.sort_by(|a, b| (&a.category, &a.folder_name).cmp(&(&b.category, &b.folder_name)));
    Ok(result)
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RegenMapImage {
    pub image_data_url: String,
    pub units_per_pixel: f64,
    pub width_px: u32,
    pub height_px: u32,
}

fn cache_path(app_data_dir: &Path, category: &str, folder_name: &str) -> PathBuf {
    app_data_dir
        .join("map_cache")
        .join(format!("{category}__{folder_name}.png"))
}

/// Builds (or loads from disk cache) the composite minimap for one map
/// folder and returns it as a PNG data URL, same shape as
/// `imageconv::preview_as_data_url` / `textures::load_texture_data_url` so
/// the frontend can drop it straight into an `<img>`/`<image>` element.
pub fn render_map(
    app_data_dir: &Path,
    client_path: &str,
    category: &str,
    folder_name: &str,
    force_rebuild: bool,
) -> Result<RegenMapImage, String> {
    let map_dir = Path::new(client_path)
        .join("pack")
        .join(category)
        .join(folder_name);
    let setting_path = map_dir.join("setting.txt");
    let setting_content = std::fs::read_to_string(&setting_path).map_err(|e| {
        format!(
            "setting.txt nicht lesbar unter {}: {e}",
            setting_path.display()
        )
    })?;
    let setting = parse_setting(&setting_content)?;

    let cache = cache_path(app_data_dir, category, folder_name);
    let composite = if !force_rebuild && cache.exists() {
        image::open(&cache)
            .map_err(|e| format!("Kartencache nicht lesbar: {e}"))?
            .to_rgba8()
    } else {
        let built = build_composite(&map_dir, &setting)?;
        if let Some(parent) = cache.parent() {
            std::fs::create_dir_all(parent).map_err(|e| e.to_string())?;
        }
        built
            .save_with_format(&cache, image::ImageFormat::Png)
            .map_err(|e| format!("Kartencache konnte nicht geschrieben werden: {e}"))?;
        built
    };

    let mut png_bytes = Vec::new();
    composite
        .write_to(&mut std::io::Cursor::new(&mut png_bytes), image::ImageFormat::Png)
        .map_err(|e| format!("Karte konnte nicht kodiert werden: {e}"))?;
    let encoded = base64::engine::general_purpose::STANDARD.encode(png_bytes);

    Ok(RegenMapImage {
        image_data_url: format!("data:image/png;base64,{encoded}"),
        units_per_pixel: setting.units_per_pixel(),
        width_px: composite.width(),
        height_px: composite.height(),
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    const REAL_SETTING: &str = "ScriptType\tMapSetting\n\nCellScale\t200\nHeightScale\t0.500000\n\nViewRadius\t128\n\nMapSize\t4\t5\nBasePosition\t409600\t896000\nTextureSet\ttextureset\\metin2_A1.txt\nEnvironment\tA1.msenv\n\n";

    #[test]
    fn parses_real_setting_file() {
        let setting = parse_setting(REAL_SETTING).expect("should parse real setting.txt");
        assert_eq!(setting.map_size, (4, 5));
        assert_eq!(setting.units_per_pixel(), 1.0);
    }

    #[test]
    fn rejects_missing_fields() {
        assert!(parse_setting("ScriptType\tMapSetting\n").is_err());
    }

    #[test]
    fn builds_composite_from_real_client_map() {
        let map_dir = Path::new(r"C:\Users\DevSteven\Desktop\Client\pack\map_empire\metin2_map_a1");
        let setting_content = std::fs::read_to_string(map_dir.join("setting.txt"))
            .expect("real client setting.txt should be readable");
        let setting = parse_setting(&setting_content).unwrap();
        let composite = build_composite(map_dir, &setting).expect("composite should build");
        assert_eq!(composite.width(), setting.map_size.0 * 256);
        assert_eq!(composite.height(), setting.map_size.1 * 256);
    }

    #[test]
    fn lists_real_client_map_folders() {
        let folders = list_map_folders(r"C:\Users\DevSteven\Desktop\Client")
            .expect("should list map folders");
        assert!(folders
            .iter()
            .any(|f| f.category == "map_empire" && f.folder_name == "metin2_map_a1"));
    }

    #[test]
    fn render_map_produces_cached_png_data_url() {
        let app_data_dir = std::env::temp_dir().join(format!(
            "m2manager_mapdata_test_{}",
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_nanos()
        ));
        let result = render_map(
            &app_data_dir,
            r"C:\Users\DevSteven\Desktop\Client",
            "map_empire",
            "metin2_map_a1",
            false,
        )
        .expect("render_map should succeed against the real client");
        assert!(result.image_data_url.starts_with("data:image/png;base64,"));
        assert_eq!(result.units_per_pixel, 1.0);
        assert!(cache_path(&app_data_dir, "map_empire", "metin2_map_a1").exists());

        // Second call should hit the cache without recomputing.
        let cached = render_map(
            &app_data_dir,
            r"C:\Users\DevSteven\Desktop\Client",
            "map_empire",
            "metin2_map_a1",
            false,
        )
        .expect("cached render_map should succeed");
        assert_eq!(cached.width_px, result.width_px);

        std::fs::remove_dir_all(&app_data_dir).ok();
    }
}
