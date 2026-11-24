#![allow(clippy::too_many_arguments)]
//! Text rendering that mirrors Pillow + libraqm: HarfBuzz shaping (rustybuzz),
//! 26.6 fixed-point pen positions, FreeType metric rounding, Pillow anchors,
//! filled strokes and `multiline_text` spacing.

use crate::config::CONFIG;
use ab_glyph::{Font as _, FontRef, GlyphId, PxScale, point};
use image::RgbaImage;
use std::sync::LazyLock;

pub type Color = [u8; 4];
pub const WHITE: Color = [255, 255, 255, 255];
pub const BLACK: Color = [0, 0, 0, 255];

pub struct Font {
    face: rustybuzz::Face<'static>,
    ab: FontRef<'static>,
    upem: f32,
    asc: f32,
    desc: f32,
}

/// Upstream font handles: SIYUAN, TBFONT, FOTNEWRODIN, SHANGGUMONO.
#[derive(Clone, Copy)]
pub enum F {
    Sy,
    Tb,
    Fot,
    Sg,
}

struct Fonts {
    sy: Option<Font>,
    tb: Option<Font>,
    fot: Option<Font>,
    sg: Option<Font>,
}

static FONTS: LazyLock<Fonts> = LazyLock::new(|| {
    let load = |name: &str| {
        let font = Font::load(&CONFIG.static_dir.join("font").join(name));
        if font.is_none() {
            log::warn!("字体加载失败: {name}");
        }
        font
    };
    Fonts {
        sy: load("ResourceHanRoundedCN-Bold.ttf"),
        tb: load("Torus SemiBold.otf"),
        fot: load("FOT-NewRodin Pro EB.otf"),
        sg: load("ShangguMonoSC-Regular.otf"),
    }
});

pub fn font(f: F) -> Option<&'static Font> {
    match f {
        F::Sy => FONTS.sy.as_ref(),
        F::Tb => FONTS.tb.as_ref(),
        F::Fot => FONTS.fot.as_ref(),
        F::Sg => FONTS.sg.as_ref(),
    }
}

/// FreeType `PIXEL()`: 26.6 -> nearest pixel.
fn pixel(v: i32) -> i32 {
    ((v + 32) & -64) >> 6
}

struct Shaped {
    /// (glyph, pen x in px, y offset in px)
    glyphs: Vec<(GlyphId, i32, i32)>,
    width26: i32,
}

impl Font {
    fn load(path: &std::path::Path) -> Option<Self> {
        let bytes: &'static [u8] = Box::leak(std::fs::read(path).ok()?.into_boxed_slice());
        let face = rustybuzz::Face::from_slice(bytes, 0)?;
        let ab = FontRef::try_from_slice(bytes).ok()?;
        let hhea = face.tables().hhea;
        let upem = f32::from(face.units_per_em() as u16);
        Some(Self { asc: f32::from(hhea.ascender), desc: f32::from(hhea.descender), face, ab, upem })
    }

    fn to26(&self, units: f32, size: f32) -> i32 {
        (units * size * 64.0 / self.upem).round() as i32
    }

    /// Grid-fitted ascender / descender in 26.6 (FreeType rounds outwards).
    fn metrics26(&self, size: f32) -> (i32, i32) {
        let a = self.to26(self.asc, size);
        let d = self.to26(self.desc, size);
        ((a + 63) & -64, d & -64)
    }

    fn scale(&self, size: f32) -> PxScale {
        PxScale::from(size * self.ab.height_unscaled() / self.upem)
    }

    fn shape(&self, text: &str, size: f32) -> Shaped {
        let mut buf = rustybuzz::UnicodeBuffer::new();
        buf.push_str(text);
        buf.guess_segment_properties();
        let out = rustybuzz::shape(&self.face, &[], buf);
        let mut pos = 0;
        let glyphs = out
            .glyph_infos()
            .iter()
            .zip(out.glyph_positions())
            .map(|(info, p)| {
                let px = pixel(pos + self.to26(p.x_offset as f32, size));
                let py = pixel(self.to26(p.y_offset as f32, size));
                pos += self.to26(p.x_advance as f32, size);
                (GlyphId(info.glyph_id as u16), px, py)
            })
            .collect();
        Shaped { glyphs, width26: pos }
    }

    /// `font.getlength(text)`.
    pub fn length(&self, text: &str, size: f32) -> f32 {
        self.shape(text, size).width26 as f32 / 64.0
    }

    /// Vertical extent (y_max, y_min) of the inked glyphs, baseline-relative, up positive.
    fn ink_y(&self, sh: &Shaped, size: f32) -> (i32, i32) {
        let scale = self.scale(size);
        let (mut top, mut bottom) = (0i32, 0i32);
        for &(id, px, py) in &sh.glyphs {
            if let Some(g) = self.ab.outline_glyph(id.with_scale_and_position(scale, point(px as f32, -py as f32))) {
                let b = g.px_bounds();
                top = top.max(-b.min.y.floor() as i32);
                bottom = bottom.min(-b.max.y.ceil() as i32);
            }
        }
        (top, bottom)
    }

    /// Bottom of `getbbox(text)` with the default `la` anchor.
    pub fn bbox_bottom(&self, text: &str, size: f32) -> i32 {
        let sh = self.shape(text, size);
        pixel(self.metrics26(size).0) - self.ink_y(&sh, size).1
    }

    /// Right of `getbbox(text)`.
    pub fn bbox_right(&self, text: &str, size: f32) -> i32 {
        let sh = self.shape(text, size);
        let scale = self.scale(size);
        let mut right = pixel(sh.width26);
        for &(id, px, py) in &sh.glyphs {
            if let Some(g) = self.ab.outline_glyph(id.with_scale_and_position(scale, point(px as f32, -py as f32))) {
                right = right.max(g.px_bounds().max.x.ceil() as i32);
            }
        }
        right
    }

    /// Draws one line; `(x, y)` is the anchor point.
    #[allow(clippy::too_many_arguments)]
    fn draw_line(
        &self,
        img: &mut RgbaImage,
        x: f32,
        y: f32,
        size: f32,
        text: &str,
        color: Color,
        anchor: &[u8; 2],
        stroke: i32,
        stroke_color: Color,
    ) {
        let sh = self.shape(text, size);
        let (asc, desc) = self.metrics26(size);
        let x_anchor = match anchor[0] {
            b'm' => pixel(sh.width26 / 2),
            b'r' => pixel(sh.width26),
            _ => 0,
        };
        let y_anchor = match anchor[1] {
            b'a' => pixel(asc),
            b'm' => pixel((asc + desc) / 2),
            b'd' => pixel(desc),
            b't' => self.ink_y(&sh, size).0,
            b'b' => self.ink_y(&sh, size).1,
            _ => 0,
        };
        let ox = x.floor() as i32 - x_anchor;
        let base = y.floor() as i32 + y_anchor;
        let scale = self.scale(size);

        // Rasterise into a coverage buffer padded by the stroke width.
        let outlines: Vec<_> = sh
            .glyphs
            .iter()
            .filter_map(|&(id, px, py)| {
                self.ab.outline_glyph(id.with_scale_and_position(scale, point((ox + px) as f32, (base - py) as f32)))
            })
            .collect();
        if outlines.is_empty() {
            return;
        }
        let (mut x0, mut y0, mut x1, mut y1) = (i32::MAX, i32::MAX, i32::MIN, i32::MIN);
        for g in &outlines {
            let b = g.px_bounds();
            x0 = x0.min(b.min.x as i32);
            y0 = y0.min(b.min.y as i32);
            x1 = x1.max(b.max.x as i32);
            y1 = y1.max(b.max.y as i32);
        }
        let pad = stroke.max(0);
        let (bx, by) = (x0 - pad, y0 - pad);
        let (w, h) = ((x1 - x0 + 2 * pad) as usize, (y1 - y0 + 2 * pad) as usize);
        let mut cov = vec![0f32; w * h];
        for g in &outlines {
            let b = g.px_bounds();
            let (gx, gy) = (b.min.x as i32 - bx, b.min.y as i32 - by);
            g.draw(|px, py, c| {
                let (cx, cy) = (gx + px as i32, gy + py as i32);
                if cx >= 0 && cy >= 0 && (cx as usize) < w && (cy as usize) < h {
                    let v = &mut cov[cy as usize * w + cx as usize];
                    *v = v.max(c.min(1.0));
                }
            });
        }
        if stroke > 0 {
            blend(img, &dilate(&cov, w, h, stroke), w, h, bx, by, stroke_color);
        }
        blend(img, &cov, w, h, bx, by, color);
    }
}

/// Filled stroke: coverage dilated by a disk of radius `r` (antialiased edge).
fn dilate(cov: &[f32], w: usize, h: usize, r: i32) -> Vec<f32> {
    let kernel: Vec<(i32, i32, f32)> = (-r..=r)
        .flat_map(|dy| (-r..=r).map(move |dx| (dx, dy)))
        .filter_map(|(dx, dy)| {
            let weight = (r as f32 + 0.5 - ((dx * dx + dy * dy) as f32).sqrt()).clamp(0.0, 1.0);
            (weight > 0.0).then_some((dx, dy, weight))
        })
        .collect();
    let mut out = vec![0f32; w * h];
    for y in 0..h as i32 {
        for x in 0..w as i32 {
            let c = cov[y as usize * w + x as usize];
            if c <= 0.0 {
                continue;
            }
            for &(dx, dy, k) in &kernel {
                let (nx, ny) = (x + dx, y + dy);
                if nx >= 0 && ny >= 0 && (nx as usize) < w && (ny as usize) < h {
                    let o = &mut out[ny as usize * w + nx as usize];
                    *o = o.max(c * k);
                }
            }
        }
    }
    out
}

/// Pillow `draw_bitmap`: lerp ink into the image by 8-bit coverage.
fn blend(img: &mut RgbaImage, cov: &[f32], w: usize, h: usize, bx: i32, by: i32, ink: Color) {
    let (iw, ih) = (img.width() as i32, img.height() as i32);
    for y in 0..h as i32 {
        let ty = by + y;
        if ty < 0 || ty >= ih {
            continue;
        }
        for x in 0..w as i32 {
            let tx = bx + x;
            let m = (cov[y as usize * w + x as usize] * 255.0).round() as u32;
            if tx < 0 || tx >= iw || m == 0 {
                continue;
            }
            let px = img.get_pixel_mut(tx as u32, ty as u32);
            let (sa, da) = (ink[3] as u32 * m / 255, px[3] as u32);
            let oa = sa + da * (255 - sa) / 255;
            if oa == 0 {
                continue;
            }
            for c in 0..3 {
                let v = (ink[c] as u32 * sa * 255 + px[c] as u32 * da * (255 - sa)) / (oa * 255).max(1);
                px[c] = v.min(255) as u8;
            }
            px[3] = oa.min(255) as u8;
        }
    }
}

/// Upstream `DrawText.draw` (handles `multiline`).
#[allow(clippy::too_many_arguments)]
pub fn draw_full(
    img: &mut RgbaImage,
    f: F,
    x: i32,
    y: i32,
    size: f32,
    text: &str,
    color: Color,
    anchor: &str,
    stroke: i32,
    stroke_color: Color,
) {
    let Some(font) = font(f) else { return };
    let a = anchor.as_bytes();
    let anchor = [a.first().copied().unwrap_or(b'l'), a.get(1).copied().unwrap_or(b'a')];
    let lines: Vec<&str> = text.split('\n').collect();
    if lines.len() == 1 {
        font.draw_line(img, x as f32, y as f32, size, text, color, &anchor, stroke, stroke_color);
        return;
    }
    let spacing = (font.bbox_bottom("A", size) + stroke + stroke + 4) as f32;
    let widths: Vec<f32> = lines.iter().map(|l| font.length(l, size)).collect();
    let max_w = widths.iter().copied().fold(0.0, f32::max);
    let mut top = y as f32;
    match anchor[1] {
        b'm' => top -= (lines.len() - 1) as f32 * spacing / 2.0,
        b'd' => top -= (lines.len() - 1) as f32 * spacing,
        _ => {}
    }
    for (line, w) in lines.iter().zip(widths) {
        let left = x as f32
            - match anchor[0] {
                b'm' => (max_w - w) / 2.0,
                b'r' => max_w - w,
                _ => 0.0,
            };
        font.draw_line(img, left, top, size, line, color, &anchor, stroke, stroke_color);
        top += spacing;
    }
}

/// Text without stroke.
pub fn draw(img: &mut RgbaImage, f: F, x: i32, y: i32, size: f32, text: &str, color: Color, anchor: &str) {
    draw_full(img, f, x, y, size, text, color, anchor, 0, WHITE);
}

/// Text with a white stroke (the common upstream style).
#[allow(clippy::too_many_arguments)]
pub fn draw_stroked(
    img: &mut RgbaImage,
    f: F,
    x: i32,
    y: i32,
    size: f32,
    text: &str,
    color: Color,
    anchor: &str,
    stroke: i32,
) {
    draw_full(img, f, x, y, size, text, color, anchor, stroke, WHITE);
}

/// Upstream `text_to_image`: plain black text on white, ShangguMono 24.
pub fn text_to_image(text: &str) -> RgbaImage {
    let (pad, margin, size) = (10, 4, 24.0);
    let lines: Vec<&str> = text.trim().split('\n').collect();
    let Some(font) = font(F::Sg) else {
        return RgbaImage::from_pixel(1, 1, WHITE.into());
    };
    let max_w = lines.iter().map(|l| font.bbox_right(l, size)).max().unwrap_or(0);
    let b = lines.last().map_or(0, |l| font.bbox_bottom(l, size));
    let n = lines.len() as i32;
    let (w, h) = (max_w + pad * 2, b * n + margin * (n - 1) + pad * 2);
    let mut img = RgbaImage::from_pixel(w.max(1) as u32, h.max(1) as u32, WHITE.into());
    for (i, line) in lines.iter().enumerate() {
        font.draw_line(
            &mut img,
            pad as f32,
            (pad + i as i32 * (margin + b)) as f32,
            size,
            line,
            BLACK,
            b"la",
            0,
            WHITE,
        );
    }
    img
}
