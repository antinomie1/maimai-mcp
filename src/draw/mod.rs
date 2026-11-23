pub mod assets;
pub mod b50;
pub mod canvas;
pub mod chart_info;
pub mod level_progress;
pub mod plate_table;
pub mod play_score;
pub mod rating_table;

use crate::config::CONFIG;
use crate::error::AppError;
use base64::{engine::general_purpose::STANDARD as BASE64, Engine};
use image::{codecs::png::PngEncoder, ExtendedColorType, ImageEncoder, RgbaImage};
use std::fs;
use std::path::PathBuf;

/// Encodes an RGBA image to PNG bytes and Base64 string, and optionally writes to output dir.
pub fn process_and_save_image(
    img: &RgbaImage,
    file_prefix: &str,
) -> Result<(String, PathBuf), AppError> {
    let mut png_bytes = Vec::new();
    let encoder = PngEncoder::new(&mut png_bytes);
    encoder
        .write_image(
            img.as_raw(),
            img.width(),
            img.height(),
            ExtendedColorType::Rgba8,
        )
        .map_err(|e| AppError::Other(format!("PNG encode error: {e}")))?;

    let base64_str = BASE64.encode(&png_bytes);

    let filename = format!("{file_prefix}_{}.png", std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap_or_default()
        .as_millis());
    let file_path = CONFIG.output_dir.join(filename);

    let _ = fs::write(&file_path, &png_bytes);

    Ok((base64_str, file_path))
}
