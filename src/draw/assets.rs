use crate::config::CONFIG;
use ab_glyph::FontArc;
use image::{ImageBuffer, Rgba, RgbaImage};
use std::fs;
use std::path::PathBuf;
use std::sync::LazyLock;

pub struct FontPool {
    pub siyuan: Option<FontArc>,
    pub torus: Option<FontArc>,
    pub fot_new_rodin: Option<FontArc>,
    pub shanggu: Option<FontArc>,
}

impl FontPool {
    pub fn load() -> Self {
        let font_dir = CONFIG.static_dir.join("font");

        let load_font = |name: &str| -> Option<FontArc> {
            let p = font_dir.join(name);
            if p.exists() {
                if let Ok(data) = fs::read(&p) {
                    return FontArc::try_from_vec(data).ok();
                }
            }
            None
        };

        Self {
            siyuan: load_font("ResourceHanRoundedCN-Bold.ttf"),
            torus: load_font("Torus SemiBold.otf"),
            fot_new_rodin: load_font("FOT-NewRodin Pro EB.otf"),
            shanggu: load_font("ShangguMonoSC-Regular.otf"),
        }
    }
}

pub static FONTS: LazyLock<FontPool> = LazyLock::new(FontPool::load);

pub fn load_image(path: &PathBuf) -> Option<RgbaImage> {
    if !path.exists() {
        return None;
    }
    image::open(path).ok().map(|img| img.to_rgba8())
}

pub fn get_pic_path(relative: &str) -> PathBuf {
    CONFIG.static_dir.join("mai/pic").join(relative)
}

/// Normalizes song_id by `% 10000` to find the exact cover image from `mai/cover/`.
pub fn get_cover_path(song_id: u32) -> PathBuf {
    let normalized_id = song_id % 10000;
    let p = CONFIG.static_dir.join("mai/cover").join(format!("{normalized_id}.png"));
    if p.exists() {
        p
    } else {
        CONFIG.static_dir.join("mai/cover/0.png")
    }
}

pub fn create_canvas(width: u32, height: u32, color: Rgba<u8>) -> RgbaImage {
    ImageBuffer::from_pixel(width, height, color)
}
