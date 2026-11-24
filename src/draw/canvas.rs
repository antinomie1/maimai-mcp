//! Image primitives matching Pillow semantics: cached assets, premultiplied
//! bicubic resize, `alpha_composite`, gradients and frosted cards.

use crate::config::CONFIG;
use crate::core::models::Theme;
use image::{RgbaImage, imageops};
use rayon::prelude::*;
use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::sync::{Arc, LazyLock, RwLock};

pub fn pic(name: &str) -> PathBuf {
    CONFIG.static_dir.join("mai/pic").join(name)
}

pub fn theme_pic(theme: Theme, name: &str) -> PathBuf {
    CONFIG.static_dir.join("mai/pic").join(theme.dir()).join(name)
}

pub fn mai_dir(sub: &str) -> PathBuf {
    CONFIG.static_dir.join("mai").join(sub)
}

/// Upstream `song_chart`: `cover/<id % 10000>.png`, falling back to `0.png`.
pub fn cover_path(song_id: u32) -> PathBuf {
    let p = mai_dir("cover").join(format!("{}.png", song_id % 10000));
    if p.exists() { p } else { mai_dir("cover").join("0.png") }
}

type Cache = RwLock<HashMap<(PathBuf, u32, u32), Arc<RgbaImage>>>;
static CACHE: LazyLock<Cache> = LazyLock::new(Default::default);

fn decode(path: &Path) -> Option<RgbaImage> {
    match image::open(path) {
        Ok(img) => Some(img.to_rgba8()),
        Err(e) => {
            log::warn!("图片加载失败 {}: {e}", path.display());
            None
        }
    }
}

/// Opens (and caches) a UI asset; covers are decoded on demand and never cached.
pub fn open(path: &Path) -> Option<Arc<RgbaImage>> {
    open_sized(path, 0, 0)
}

/// Opens an image resized to `w x h` (0 = original). Cached unless it is a cover.
pub fn open_sized(path: &Path, w: u32, h: u32) -> Option<Arc<RgbaImage>> {
    let key = (path.to_path_buf(), w, h);
    if let Some(img) = CACHE.read().ok()?.get(&key) {
        return Some(img.clone());
    }
    let raw = decode(path)?;
    let img = Arc::new(if w == 0 || (raw.width() == w && raw.height() == h) { raw } else { resize(&raw, w, h) });
    if !path.starts_with(mai_dir("cover"))
        && let Ok(mut c) = CACHE.write()
    {
        c.insert(key, img.clone());
    }
    Some(img)
}

/// Covers for many songs at once, decoded in parallel.
pub fn covers(ids: &[u32], w: u32, h: u32) -> HashMap<u32, Arc<RgbaImage>> {
    let mut uniq: Vec<u32> = ids.to_vec();
    uniq.sort_unstable();
    uniq.dedup();
    uniq.par_iter().filter_map(|&id| Some((id, open_sized(&cover_path(id), w, h)?))).collect()
}

/// Pillow `Image.resize` (BICUBIC) on premultiplied alpha.
pub fn resize(img: &RgbaImage, w: u32, h: u32) -> RgbaImage {
    resize_with(img, w, h, imageops::FilterType::CatmullRom)
}

pub fn resize_with(img: &RgbaImage, w: u32, h: u32, filter: imageops::FilterType) -> RgbaImage {
    let mut pre = img.clone();
    for p in pre.pixels_mut() {
        let a = u32::from(p[3]);
        for c in 0..3 {
            let t = u32::from(p[c]) * a + 128;
            p[c] = ((t + (t >> 8)) >> 8) as u8;
        }
    }
    let mut out = imageops::resize(&pre, w.max(1), h.max(1), filter);
    for p in out.pixels_mut() {
        let a = u32::from(p[3]);
        if a != 0 && a != 255 {
            let inv = (255 << 16) / a;
            for c in 0..3 {
                p[c] = ((u32::from(p[c]) * inv + (1 << 15)) >> 16).min(255) as u8;
            }
        }
    }
    out
}

/// Pillow `Image.alpha_composite(src, dest=(x, y))`.
pub fn paste(dst: &mut RgbaImage, src: &RgbaImage, x: i32, y: i32) {
    let (dw, dh) = (dst.width() as i32, dst.height() as i32);
    let (sw, sh) = (src.width() as i32, src.height() as i32);
    let (x0, y0) = (x.max(0), y.max(0));
    let (x1, y1) = ((x + sw).min(dw), (y + sh).min(dh));
    if x0 >= x1 || y0 >= y1 {
        return;
    }
    let (d, s) = (dst.as_mut(), src.as_raw());
    for ty in y0..y1 {
        let srow = ((ty - y) * sw) as usize * 4;
        let drow = (ty * dw) as usize * 4;
        for tx in x0..x1 {
            let si = srow + (tx - x) as usize * 4;
            let di = drow + tx as usize * 4;
            let sa = u32::from(s[si + 3]);
            if sa == 0 {
                continue;
            }
            if sa == 255 {
                d[di..di + 4].copy_from_slice(&s[si..si + 4]);
                continue;
            }
            let da = u32::from(d[di + 3]);
            let blend = da * (255 - sa);
            let outa255 = sa * 255 + blend;
            let coef1 = sa * 255 * 255 * 128 / outa255;
            let coef2 = 255 * 128 - coef1;
            for c in 0..3 {
                let t = u32::from(s[si + c]) * coef1 + u32::from(d[di + c]) * coef2 + (0x80 << 7);
                d[di + c] = (((t >> 8) + t) >> 8 >> 7).min(255) as u8;
            }
            let t = outa255 + 0x80;
            d[di + 3] = (((t >> 8) + t) >> 8) as u8;
        }
    }
}

/// Opens, optionally resizes (0 = keep) and composites an asset; missing files are skipped.
pub fn paste_file(dst: &mut RgbaImage, path: &Path, w: u32, h: u32, x: i32, y: i32) {
    if let Some(img) = open_sized(path, w, h) {
        paste(dst, &img, x, y);
    }
}

pub fn crop(img: &RgbaImage, x: u32, y: u32, w: u32, h: u32) -> RgbaImage {
    imageops::crop_imm(img, x, y, w.min(img.width().saturating_sub(x)), h.min(img.height().saturating_sub(y)))
        .to_image()
}

/// Upstream `tricolor_gradient_prism_plus`.
pub fn gradient(width: u32, height: u32) -> RgbaImage {
    const STOPS: [(f64, [f64; 3]); 9] = [
        (0.0, [255.0, 255.0, 255.0]),
        (0.14, [255.0, 255.0, 255.0]),
        (0.24, [255.0, 213.0, 207.0]),
        (0.46, [255.0, 213.0, 207.0]),
        (0.56, [255.0, 197.0, 213.0]),
        (0.67, [234.0, 171.0, 255.0]),
        (0.85, [114.0, 188.0, 254.0]),
        (0.95, [101.0, 242.0, 223.0]),
        (1.0, [101.0, 242.0, 223.0]),
    ];
    let mut img = RgbaImage::new(width, height);
    for y in 0..height {
        let t = if height > 1 { 1.0 - f64::from(y) / f64::from(height - 1) } else { 0.0 };
        let mut px = [0u8, 0, 0, 0];
        for pair in STOPS.windows(2) {
            let ((p1, c1), (p2, c2)) = (pair[0], pair[1]);
            if p1 <= t && t <= p2 {
                let r = (t - p1) / (p2 - p1);
                for j in 0..3 {
                    px[j] = (c1[j] + (c2[j] - c1[j]) * r) as u8;
                }
                px[3] = 255;
                break;
            }
        }
        for x in 0..width {
            img.put_pixel(x, y, image::Rgba(px));
        }
    }
    img
}

/// Pillow `rounded_rectangle((x0, y0, x1, y1), radius)` coverage test (inclusive box).
fn in_rounded(px: i32, py: i32, x0: i32, y0: i32, x1: i32, y1: i32, r: i32) -> bool {
    if px < x0 || py < y0 || px > x1 || py > y1 {
        return false;
    }
    let cx = if px < x0 + r {
        x0 + r
    } else if px > x1 - r {
        x1 - r
    } else {
        return true;
    };
    let cy = if py < y0 + r {
        y0 + r
    } else if py > y1 - r {
        y1 - r
    } else {
        return true;
    };
    let (dx, dy) = ((px - cx) as f32, (py - cy) as f32);
    dx * dx + dy * dy <= (r as f32 + 0.5).powi(2)
}

/// Upstream `generate_frosted_card`: blurred translucent rounded card with drop shadow.
pub fn frosted_card(im: &RgbaImage, b: (u32, u32, u32, u32), alpha: f32) -> RgbaImage {
    let (w, h) = (b.2 - b.0, b.3 - b.1);
    let mut card = imageops::fast_blur(&crop(im, b.0, b.1, w, h), 4.0);
    let white = RgbaImage::from_pixel(w, h, image::Rgba([255, 255, 255, (255.0 * alpha) as u8]));
    paste(&mut card, &white, 0, 0);

    let (sw, sh) = (w + 20, h + 20);
    let mut shadow = RgbaImage::new(sw, sh);
    for (x, y, p) in shadow.enumerate_pixels_mut() {
        if in_rounded(x as i32, y as i32, 15, 15, 15 + w as i32, 15 + h as i32, 25) {
            *p = image::Rgba([0, 0, 0, 50]);
        }
    }
    let shadow = imageops::fast_blur(&shadow, 3.0);

    let mut layer = RgbaImage::new(im.width(), im.height());
    let (sx, sy) = (b.0 as i64 + 10 - 15, b.1 as i64 + 10 - 15);
    imageops::replace(&mut layer, &shadow, sx, sy);
    for y in 0..h {
        for x in 0..w {
            if in_rounded(x as i32, y as i32, 0, 0, w as i32, h as i32, 25) {
                let (tx, ty) = (b.0 + x, b.1 + y);
                if tx < layer.width() && ty < layer.height() {
                    layer.put_pixel(tx, ty, *card.get_pixel(x, y));
                }
            }
        }
    }
    let mut out = im.clone();
    paste(&mut out, &layer, 0, 0);
    out
}

/// Standard table background (`_generate_bg` / `DrawScore.__init__`).
pub fn table_bg(height: u32, separator: Option<i32>) -> RgbaImage {
    let mut im = gradient(1400, height);
    let h = height as i32;
    paste_file(&mut im, &pic("aurora.png"), 0, 0, 0, 0);
    paste_file(&mut im, &pic("bg_shines.png"), 0, 0, 11, 6);
    paste_file(&mut im, &pic("rainbow.png"), 0, 0, 318, h - 545);
    paste_file(&mut im, &pic("rainbow_bottom.png"), 0, 0, 122, h - 305);
    if let Some(pattern) = open(&pic("pattern.png")) {
        for i in 0..=(h / 358) {
            paste(&mut im, &pattern, 0, (358 + 7) * i);
        }
    }
    if let Some(y) = separator {
        paste_file(&mut im, &pic("separator.png"), 0, 0, 100, y);
    }
    im
}
