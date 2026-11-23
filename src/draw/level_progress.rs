use crate::core::models::music::SongType;
use crate::draw::assets::{get_cover_path, get_pic_path, load_image, FONTS};
use crate::draw::canvas::CanvasHelper;
use crate::error::AppError;
use image::{imageops, Rgba, RgbaImage};

#[derive(Debug, Clone)]
pub struct RiseRecommendation {
    pub song_id: u32,
    pub song_name: String,
    pub level_index: usize,
    pub level_value: f64,
    pub song_type: SongType,
    pub old_achievements: f64,
    pub old_rating: u32,
    pub target_achievements: f64,
    pub target_rating: u32,
    pub rate_str: String,
}

pub struct LevelProgressRenderer;

impl LevelProgressRenderer {
    pub fn diff_name(level_index: usize) -> &'static str {
        match level_index {
            0 => "basic",
            1 => "advanced",
            2 => "expert",
            3 => "master",
            _ => "remaster",
        }
    }

    pub fn diff_text_color(level_index: usize) -> Rgba<u8> {
        match level_index {
            0 => Rgba([106, 185, 29, 255]),
            1 => Rgba([240, 158, 25, 255]),
            2 => Rgba([244, 78, 102, 255]),
            3 => Rgba([159, 78, 237, 255]),
            _ => Rgba([219, 170, 255, 255]),
        }
    }

    pub fn render_rise_score(
        sd_list: &[RiseRecommendation],
        dx_list: &[RiseRecommendation],
    ) -> Result<RgbaImage, AppError> {
        let max_items = sd_list.len().max(dx_list.len());
        let total_height = (160 + max_items * 140 + 120) as u32;

        let mut canvas = RgbaImage::new(1400, total_height);

        // Gradient & patterns
        let aurora_path = get_pic_path("aurora.png");
        if let Some(aurora) = load_image(&aurora_path) {
            CanvasHelper::overlay(&mut canvas, &aurora, 0, 0);
        }
        let shines_path = get_pic_path("shines.png");
        if let Some(shines) = load_image(&shines_path) {
            CanvasHelper::overlay(&mut canvas, &shines, 11, 6);
        }

        let pattern_path = get_pic_path("pattern.png");
        if let Some(pat) = load_image(&pattern_path) {
            let pat_rows = (total_height / 365) + 1;
            for h in 0..pat_rows {
                CanvasHelper::overlay(&mut canvas, &pat, 0, (365 * h) as i64);
            }
        }

        let siyuan = FONTS.siyuan.as_ref()
            .ok_or_else(|| AppError::Other("SiYuan font not found".into()))?;
        let tb = FONTS.torus.as_ref().or(FONTS.siyuan.as_ref())
            .ok_or_else(|| AppError::Other("Torus font not found".into()))?;
        let default_text_color = Rgba([114, 188, 254, 255]);

        // Title headers
        let title_path = get_pic_path("prism_plus/title.png");
        if let Some(title_img) = load_image(&title_path) {
            let resized = CanvasHelper::resize(&title_img, 273, 80);
            CanvasHelper::overlay(&mut canvas, &resized, 314, 30);
            CanvasHelper::overlay(&mut canvas, &resized, 814, 30);
        }

        CanvasHelper::draw_text(&mut canvas, default_text_color, 450, 68, 18.0, siyuan, "旧版本谱面推荐", "mm");
        CanvasHelper::draw_text(&mut canvas, default_text_color, 950, 68, 18.0, siyuan, "新版本谱面推荐", "mm");

        // Helper closure to render columns
        let mut render_column = |list: &[RiseRecommendation], base_x: i32| {
            for (index, item) in list.iter().enumerate() {
                let x = base_x;
                let y = 120 + index as i32 * 140;

                // Rise card background
                let diff_s = Self::diff_name(item.level_index);
                let rise_bg_path = get_pic_path(&format!("rise_score_{diff_s}.png"));
                if let Some(rise_bg) = load_image(&rise_bg_path) {
                    CanvasHelper::overlay(&mut canvas, &rise_bg, (x + 30) as i64, y as i64);
                }

                // Cover
                let cover_path = get_cover_path(item.song_id);
                if let Some(cover) = load_image(&cover_path) {
                    let resized = CanvasHelper::resize(&cover, 80, 80);
                    CanvasHelper::overlay(&mut canvas, &resized, (x + 55) as i64, (y + 41) as i64);
                }

                // Type banner
                let type_s = match item.song_type {
                    SongType::Standard => "SD",
                    SongType::Deluxe => "DX",
                    SongType::Utage => "UTAGE",
                };
                let type_path = get_pic_path(&format!("{type_s}.png"));
                if let Some(type_img) = load_image(&type_path) {
                    let resized = CanvasHelper::resize(&type_img, 60, 22);
                    CanvasHelper::overlay(&mut canvas, &resized, (x + 240) as i64, (y + 114) as i64);
                }

                // Rank
                let rank_path = get_pic_path(&format!("prism_plus/UI_TTR_Rank_{}.png", item.rate_str));
                if let Some(rank_img) = load_image(&rank_path) {
                    let resized = CanvasHelper::resize(&rank_img, 63, 28);
                    CanvasHelper::overlay(&mut canvas, &resized, (x + 305) as i64, (y + 82) as i64);
                }

                let diff_color = Self::diff_text_color(item.level_index);
                let id_color = diff_color;

                // Title
                let title_trunc = CanvasHelper::truncate_title_by_font(&item.song_name, siyuan, 17.0, 220.0);
                CanvasHelper::draw_text(&mut canvas, diff_color, x + 142, y + 44, 17.0, siyuan, &title_trunc, "lm");

                // ID
                CanvasHelper::draw_text(&mut canvas, id_color, x + 145, y + 124, 18.0, tb, &format!("ID: {}", item.song_id), "lm");

                // Old ach & rating
                let old_ach_str = format!("{:.4}%", item.old_achievements);
                CanvasHelper::draw_text(&mut canvas, diff_color, x + 210, y + 71, 25.0, tb, &old_ach_str, "mm");
                CanvasHelper::draw_text(&mut canvas, diff_color, x + 245, y + 96, 17.0, tb, &format!("Ra: {}", item.old_rating), "mm");

                // Target ach & rating
                let target_ach_str = format!("{:.4}%", item.target_achievements);
                CanvasHelper::draw_text(&mut canvas, diff_color, x + 370, y + 71, 25.0, tb, &target_ach_str, "mm");
                CanvasHelper::draw_text(&mut canvas, diff_color, x + 415, y + 96, 17.0, tb, &format!("Ra: {}", item.target_rating), "mm");

                // DS & Gain
                CanvasHelper::draw_text(&mut canvas, id_color, x + 315, y + 124, 18.0, tb, &format!("ds:{:.1}", item.level_value), "lm");
                let gain = item.target_rating.saturating_sub(item.old_rating);
                CanvasHelper::draw_text(&mut canvas, id_color, x + 390, y + 124, 18.0, tb, &format!("Ra +{gain}"), "lm");
            }
        };

        render_column(sd_list, 200);
        render_column(dx_list, 700);

        // Footer
        let h = canvas.height() as i32;
        CanvasHelper::draw_text(&mut canvas, default_text_color, 700, h - 84, 25.0, siyuan, "「谱面推荐不使用任何算法，仅供参考」", "mm");
        CanvasHelper::draw_text(&mut canvas, default_text_color, 700, h - 36, 18.0, siyuan, "Designed by Yuri-YuzuChaN & BlueDeer233. maimai-mcp Rust Engine", "mm");

        // Crop center 1000px width (from x=200 to 1200)
        let cropped = imageops::crop_imm(&canvas, 200, 0, 1000, total_height).to_image();
        Ok(cropped)
    }
}
