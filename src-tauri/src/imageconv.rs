use base64::Engine;
use std::path::Path;

/// Converts any image the `image` crate can decode (PNG/JPEG/BMP/GIF/TGA)
/// into a `.tga` file at `dest_path`. Shared by the Item Editor's icon step
/// (`packtools::write_icon_tga`) and the standalone TGA Converter tool -
/// same conversion, two entry points.
pub fn convert_to_tga(source_path: &str, dest_path: &Path) -> Result<(), String> {
    let image = image::open(source_path)
        .map_err(|e| format!("Bild konnte nicht gelesen werden: {e}"))?
        .to_rgba8();

    if let Some(parent) = dest_path.parent() {
        std::fs::create_dir_all(parent).map_err(|e| e.to_string())?;
    }

    image
        .save_with_format(dest_path, image::ImageFormat::Tga)
        .map_err(|e| format!("Konnte nicht als .tga gespeichert werden: {e}"))
}

/// Decodes any supported image (including `.tga`, which browsers can't
/// render natively) and re-encodes it as a PNG data URL for `<img>` preview.
pub fn preview_as_data_url(path: &str) -> Result<String, String> {
    let image = image::open(path).map_err(|e| format!("Bild konnte nicht gelesen werden: {e}"))?;

    let mut png_bytes = Vec::new();
    image
        .write_to(&mut std::io::Cursor::new(&mut png_bytes), image::ImageFormat::Png)
        .map_err(|e| format!("Vorschau konnte nicht erzeugt werden: {e}"))?;

    let encoded = base64::engine::general_purpose::STANDARD.encode(png_bytes);
    Ok(format!("data:image/png;base64,{encoded}"))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn converts_real_client_icon_png_round_trip() {
        // Reuses a real file already on this machine (verified elsewhere in
        // icons.rs tests) rather than a synthetic fixture.
        let source = r"C:\Users\DevSteven\Desktop\Client\pack\icon\icon\item\00010.tga";
        let dest = std::env::temp_dir().join(format!(
            "m2manager_imageconv_test_{}.tga",
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_nanos()
        ));

        convert_to_tga(source, &dest).expect("convert_to_tga failed");
        assert!(dest.exists());
        image::open(&dest).expect("converted tga should be readable");

        let preview = preview_as_data_url(dest.to_str().unwrap()).expect("preview failed");
        assert!(preview.starts_with("data:image/png;base64,"));

        std::fs::remove_file(&dest).ok();
    }
}
