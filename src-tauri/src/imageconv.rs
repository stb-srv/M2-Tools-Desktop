use base64::Engine;
use std::path::Path;

/// Converts any image the `image` crate can decode (PNG/JPEG/BMP/GIF/TGA)
/// into a `.tga` file at `dest_path`. Shared by the Item Editor's icon step
/// (`packtools::write_icon_tga`) and the standalone TGA Converter tool -
/// same conversion, two entry points.
///
/// The TGA is written by hand as uncompressed 32-bit BGRA with bottom-left
/// origin - the exact layout of the client's stock icons. The `image`
/// crate's own TGA encoder must NOT be used here: it RLE-compresses, and
/// its RLE output is rejected by strict decoders (Pillow: "buffer overrun";
/// the game client: icon renders invisible). Verified live 2026-08-05 with
/// the FrostEdge import - 5 of 7 icons produced by the old
/// `save_with_format(Tga)` path were undecodable and showed as empty
/// inventory slots.
pub fn convert_to_tga(source_path: &str, dest_path: &Path) -> Result<(), String> {
    let image = image::open(source_path)
        .map_err(|e| format!("Bild konnte nicht gelesen werden: {e}"))?
        .to_rgba8();
    let (width, height) = image.dimensions();
    if width > u16::MAX as u32 || height > u16::MAX as u32 {
        return Err(format!("Bild ist zu groß für TGA: {width}x{height}"));
    }

    let mut out = Vec::with_capacity(18 + (width * height * 4) as usize);
    out.extend_from_slice(&[
        0, // no image ID
        0, // no color map
        2, // uncompressed truecolor
        0, 0, 0, 0, 0, // color map spec (unused)
        0, 0, // x origin
        0, 0, // y origin
    ]);
    out.extend_from_slice(&(width as u16).to_le_bytes());
    out.extend_from_slice(&(height as u16).to_le_bytes());
    out.push(32); // bits per pixel
    out.push(0x08); // descriptor: 8 alpha bits, bottom-left origin

    for row in image.rows().rev() {
        for pixel in row {
            let [r, g, b, a] = pixel.0;
            out.extend_from_slice(&[b, g, r, a]);
        }
    }

    if let Some(parent) = dest_path.parent() {
        std::fs::create_dir_all(parent).map_err(|e| e.to_string())?;
    }
    std::fs::write(dest_path, out).map_err(|e| format!("Konnte nicht als .tga gespeichert werden: {e}"))
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

        // Must match the stock icons' exact layout (uncompressed truecolor,
        // 32bpp, bottom-left origin) - the client rejects RLE-compressed TGA.
        let header = &std::fs::read(&dest).unwrap()[..18];
        assert_eq!(header[2], 2, "image type must be uncompressed truecolor, not RLE");
        assert_eq!(header[16], 32, "must be 32bpp BGRA");
        assert_eq!(header[17], 0x08, "descriptor must be 8 alpha bits + bottom-left origin");

        let preview = preview_as_data_url(dest.to_str().unwrap()).expect("preview failed");
        assert!(preview.starts_with("data:image/png;base64,"));

        std::fs::remove_file(&dest).ok();
    }
}
