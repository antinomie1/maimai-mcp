use ab_glyph::{Font, PxScale, ScaleFont};
use image::{imageops, Rgba, RgbaImage};
use imageproc::drawing::draw_text_mut;

pub struct CanvasHelper;

impl CanvasHelper {
    /// Overlay `source` on `target` at (x, y) with alpha compositing
    pub fn overlay(target: &mut RgbaImage, source: &RgbaImage, x: i64, y: i64) {
        imageops::overlay(target, source, x, y);
    }

    /// Resize image using Lanczos3
    pub fn resize(img: &RgbaImage, nwidth: u32, nheight: u32) -> RgbaImage {
        imageops::resize(img, nwidth, nheight, imageops::FilterType::Lanczos3)
    }

    /// Measure text pixel width using font and scale
    pub fn measure_text_width<F: Font>(text: &str, font: &F, scale_size: f32) -> f32 {
        let scale = PxScale::from(scale_size);
        let scaled_font = font.as_scaled(scale);
        let mut width = 0.0;
        for ch in text.chars() {
            width += scaled_font.h_advance(scaled_font.glyph_id(ch));
        }
        width
    }

    /// Truncate song title by pixel width (max width default 160.0 px at 14pt)
    pub fn truncate_title_by_font<F: Font>(title: &str, font: &F, scale_size: f32, max_width: f32) -> String {
        let full_w = Self::measure_text_width(title, font, scale_size);
        if full_w <= max_width {
            return title.to_string();
        }
        let dots_w = Self::measure_text_width("...", font, scale_size);
        let avail_w = max_width - dots_w;
        let mut cur = String::new();
        for ch in title.chars() {
            let next = format!("{}{}", cur, ch);
            if Self::measure_text_width(&next, font, scale_size) > avail_w {
                break;
            }
            cur.push(ch);
        }
        format!("{}...", cur)
    }

    /// Draw text on canvas with anchor handling (Left-Middle, Center-Middle, Left-Top, Right-Middle)
    /// and optional stroke/outline support.
    pub fn draw_text_with_stroke<F: Font>(
        target: &mut RgbaImage,
        color: Rgba<u8>,
        x: i32,
        y: i32,
        scale_size: f32,
        font: &F,
        text: &str,
        anchor: &str,
        stroke_width: i32,
        stroke_color: Rgba<u8>,
    ) {
        let scale = PxScale::from(scale_size);
        let scaled_font = font.as_scaled(scale);
        let (mut draw_x, mut draw_y) = (x, y);

        // Measure text width for anchor
        if anchor == "mm" || anchor == "lm" || anchor == "rm" {
            let mut width = 0.0;
            for ch in text.chars() {
                width += scaled_font.h_advance(scaled_font.glyph_id(ch));
            }
            if anchor == "mm" {
                draw_x -= (width / 2.0) as i32;
                draw_y -= (scale_size / 2.0) as i32;
            } else if anchor == "lm" {
                draw_y -= (scale_size / 2.0) as i32;
            } else if anchor == "rm" {
                draw_x -= width as i32;
                draw_y -= (scale_size / 2.0) as i32;
            }
        }

        // Draw outline / stroke if specified
        if stroke_width > 0 {
            for dx in -stroke_width..=stroke_width {
                for dy in -stroke_width..=stroke_width {
                    if dx * dx + dy * dy <= stroke_width * stroke_width && (dx != 0 || dy != 0) {
                        draw_text_mut(
                            target,
                            stroke_color,
                            draw_x + dx,
                            draw_y + dy,
                            scale,
                            font,
                            text,
                        );
                    }
                }
            }
        }

        draw_text_mut(target, color, draw_x, draw_y, scale, font, text);
    }

    pub fn draw_text<F: Font>(
        target: &mut RgbaImage,
        color: Rgba<u8>,
        x: i32,
        y: i32,
        scale_size: f32,
        font: &F,
        text: &str,
        anchor: &str,
    ) {
        Self::draw_text_with_stroke(
            target,
            color,
            x,
            y,
            scale_size,
            font,
            text,
            anchor,
            0,
            Rgba([0, 0, 0, 0]),
        );
    }

    pub fn tricolor_gradient_prism_plus(width: u32, height: u32) -> RgbaImage {
        let colors_list: [(f32, [u8; 3]); 9] = [
            (0.00, [101, 242, 223]),
            (0.05, [101, 242, 223]),
            (0.15, [114, 188, 254]),
            (0.33, [234, 171, 255]),
            (0.44, [255, 197, 213]),
            (0.54, [255, 213, 207]),
            (0.76, [255, 213, 207]),
            (0.86, [255, 255, 255]),
            (1.00, [255, 255, 255]),
        ];

        let mut img = RgbaImage::new(width, height);
        for y in 0..height {
            let t = 1.0 - (y as f32 / (height.saturating_sub(1) as f32).max(1.0));
            let mut rgb = [255, 255, 255];
            for i in 0..colors_list.len() - 1 {
                let (p1, c1) = colors_list[i];
                let (p2, c2) = colors_list[i + 1];
                if t >= p1 && t <= p2 {
                    let rel_t = (t - p1) / (p2 - p1);
                    rgb = [
                        (c1[0] as f32 + (c2[0] as f32 - c1[0] as f32) * rel_t) as u8,
                        (c1[1] as f32 + (c2[1] as f32 - c1[1] as f32) * rel_t) as u8,
                        (c1[2] as f32 + (c2[2] as f32 - c1[2] as f32) * rel_t) as u8,
                    ];
                    break;
                }
            }
            let pixel = Rgba([rgb[0], rgb[1], rgb[2], 255]);
            for x in 0..width {
                img.put_pixel(x, y, pixel);
            }
        }
        img
    }

    pub fn generate_table_bg(width: u32, height: u32, separator_y: i64) -> RgbaImage {
        let mut im = Self::tricolor_gradient_prism_plus(width, height);
        let aurora_path = crate::draw::assets::get_pic_path("aurora.png");
        if let Some(aurora) = crate::draw::assets::load_image(&aurora_path) {
            Self::overlay(&mut im, &aurora, 0, 0);
        }
        let shines_path = crate::draw::assets::get_pic_path("bg_shines.png");
        if let Some(shines) = crate::draw::assets::load_image(&shines_path) {
            Self::overlay(&mut im, &shines, 11, 6);
        }
        let rainbow_path = crate::draw::assets::get_pic_path("rainbow.png");
        if let Some(rainbow) = crate::draw::assets::load_image(&rainbow_path) {
            Self::overlay(&mut im, &rainbow, 318, height as i64 - 545);
        }
        let rainbow_b_path = crate::draw::assets::get_pic_path("rainbow_bottom.png");
        if let Some(rb) = crate::draw::assets::load_image(&rainbow_b_path) {
            Self::overlay(&mut im, &rb, 122, height as i64 - 305);
        }
        let pattern_path = crate::draw::assets::get_pic_path("pattern.png");
        if let Some(pat) = crate::draw::assets::load_image(&pattern_path) {
            let pat_rows = (height / 365) + 1;
            for h in 0..pat_rows {
                Self::overlay(&mut im, &pat, 0, (365 * h) as i64);
            }
        }
        let sep_path = crate::draw::assets::get_pic_path("separator.png");
        if let Some(sep) = crate::draw::assets::load_image(&sep_path) {
            Self::overlay(&mut im, &sep, 100, separator_y);
        }
        im
    }
}
