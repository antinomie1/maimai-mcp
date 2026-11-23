use crate::config::CONFIG;
use crate::core::database::music_db::MUSIC_DB;
use crate::core::models::music::SongType;
use crate::core::models::score::{ComboStatus, PlayerB50, ScoreRecord, SyncStatus};
use crate::draw::assets::{get_cover_path, get_pic_path, load_image, FONTS};
use crate::draw::canvas::CanvasHelper;
use crate::error::AppError;
use image::{Rgba, RgbaImage};
use rayon::prelude::*;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum B50Theme {
    Circle,
    PrismPlus,
}

impl B50Theme {
    pub fn as_str(&self) -> &'static str {
        match self {
            B50Theme::Circle => "circle",
            B50Theme::PrismPlus => "prism_plus",
        }
    }
}

pub struct B50Renderer;

impl B50Renderer {
    fn find_ra_pic(rating: u32, theme: B50Theme) -> String {
        let thresholds = [
            (1000, "01"),
            (2000, "02"),
            (4000, "03"),
            (7000, "04"),
            (10000, "05"),
            (12000, "06"),
            (13000, "07"),
            (14000, "08"),
            (14500, "09"),
            (15000, "10"),
        ];
        for (limit, num) in thresholds {
            if rating < limit {
                return format!("{}/UI_CMN_DXRating_{num}.png", theme.as_str());
            }
        }
        if theme == B50Theme::Circle {
            if rating < 16000 {
                "circle/UI_CMN_DXRating_11.png".to_string()
            } else if rating < 17000 {
                "circle/UI_CMN_DXRating_12.png".to_string()
            } else {
                "circle/UI_CMN_DXRating_11.png".to_string()
            }
        } else {
            "prism_plus/UI_CMN_DXRating_11.png".to_string()
        }
    }

    fn ra_pic_star(rating: u32, theme: B50Theme) -> Option<String> {
        if theme != B50Theme::Circle || rating < 14000 {
            return None;
        }
        let thresholds = [
            14000, 14250, 14500, 14750, 15000, 15250, 15500, 15750, 16000, 16250, 16500, 16750,
        ];
        let num_map = [1, 2, 1, 2, 1, 2, 3, 4, 1, 2, 3, 4];
        let mut idx = 0;
        for (i, &t) in thresholds.iter().enumerate() {
            if rating >= t {
                idx = i;
            }
        }
        Some(format!("circle/UI_CMN_DXRating_Star_0{}.png", num_map[idx]))
    }

    fn get_dx_star_icon(dx_score: u32, max_dx: u32) -> Option<&'static str> {
        if max_dx == 0 {
            return None;
        }
        let percent = (dx_score as f64 / max_dx as f64) * 100.0;
        let star = if percent <= 85.0 {
            0
        } else if percent <= 90.0 {
            1
        } else if percent <= 93.0 {
            2
        } else if percent <= 95.0 {
            3
        } else if percent <= 97.0 {
            4
        } else {
            5
        };
        match star {
            1 => Some("UI_GAM_Gauge_DXScoreIcon_01.png"),
            2 => Some("UI_GAM_Gauge_DXScoreIcon_02.png"),
            3 => Some("UI_GAM_Gauge_DXScoreIcon_03.png"),
            4 => Some("UI_GAM_Gauge_DXScoreIcon_04.png"),
            5 => Some("UI_GAM_Gauge_DXScoreIcon_05.png"),
            _ => None,
        }
    }

    fn render_score_card(record: &ScoreRecord, theme: B50Theme) -> RgbaImage {
        let diff_names = [
            "b50_score_basic.png",
            "b50_score_advanced.png",
            "b50_score_expert.png",
            "b50_score_master.png",
            "b50_score_remaster.png",
        ];
        let diff_idx = (record.level_index as usize).min(4);
        let bg_path = get_pic_path(diff_names[diff_idx]);
        let mut card = load_image(&bg_path)
            .unwrap_or_else(|| image::ImageBuffer::from_pixel(266, 106, Rgba([30, 30, 40, 255])));

        // 1. Song Cover (75x75 at (12, 12)) — normalize % 10000
        let cover_path = get_cover_path(record.song_id);
        if let Some(cover) = load_image(&cover_path) {
            let resized = CanvasHelper::resize(&cover, 75, 75);
            CanvasHelper::overlay(&mut card, &resized, 12, 12);
        }

        // 2. Type (37x14 at (51, 91))
        let type_name = match record.song_type {
            SongType::Deluxe => "DX.png",
            _ => "SD.png",
        };
        if let Some(type_img) = load_image(&get_pic_path(type_name)) {
            let resized = CanvasHelper::resize(&type_img, 37, 14);
            CanvasHelper::overlay(&mut card, &resized, 51, 91);
        }

        // 3. Rate Icon (63x28 at (92, 78))
        let rate_upper = match record.rate.to_lowercase().as_str() {
            "sssp" => "SSSp",
            "sss" => "SSS",
            "ssp" => "SSp",
            "ss" => "SS",
            "sp" => "Sp",
            "s" => "S",
            "aaa" => "AAA",
            "aa" => "AA",
            "a" => "A",
            "bbb" => "BBB",
            "bb" => "BB",
            "b" => "B",
            "c" => "C",
            _ => "D",
        };
        let rate_icon_name = format!("{}/UI_TTR_Rank_{rate_upper}.png", theme.as_str());
        if let Some(rate_img) = load_image(&get_pic_path(&rate_icon_name)) {
            let resized = CanvasHelper::resize(&rate_img, 63, 28);
            CanvasHelper::overlay(&mut card, &resized, 92, 78);
        }

        // 4. FC & FS Icons (34x34 at (154, 77) & (185, 77))
        let fc_icon = match record.fc {
            ComboStatus::FC => Some("UI_MSS_MBase_Icon_FC.png"),
            ComboStatus::FCP => Some("UI_MSS_MBase_Icon_FCp.png"),
            ComboStatus::AP => Some("UI_MSS_MBase_Icon_AP.png"),
            ComboStatus::APP => Some("UI_MSS_MBase_Icon_APp.png"),
            ComboStatus::None => None,
        };
        if let Some(fc_file) = fc_icon {
            if let Some(fc_img) = load_image(&get_pic_path(fc_file)) {
                let resized = CanvasHelper::resize(&fc_img, 34, 34);
                CanvasHelper::overlay(&mut card, &resized, 154, 77);
            }
        }

        let fs_icon = match record.fs {
            SyncStatus::FS => Some("UI_MSS_MBase_Icon_FS.png"),
            SyncStatus::FSP => Some("UI_MSS_MBase_Icon_FSp.png"),
            SyncStatus::FDX => Some("UI_MSS_MBase_Icon_FSD.png"),
            SyncStatus::FDXP => Some("UI_MSS_MBase_Icon_FSDp.png"),
            SyncStatus::Sync => Some("UI_MSS_MBase_Icon_Sync.png"),
            SyncStatus::None => None,
        };
        if let Some(fs_file) = fs_icon {
            if let Some(fs_img) = load_image(&get_pic_path(fs_file)) {
                let resized = CanvasHelper::resize(&fs_img, 34, 34);
                CanvasHelper::overlay(&mut card, &resized, 185, 77);
            }
        }

        // 5. DX Star Icon (47x26 at (217, 80))
        let max_dx = MUSIC_DB.read().map(|db| db.get_max_dx_score(record.song_id, diff_idx)).unwrap_or(0);
        if let Some(star_file) = Self::get_dx_star_icon(record.dx_score, max_dx) {
            if let Some(star_img) = load_image(&get_pic_path(star_file)) {
                let resized = CanvasHelper::resize(&star_img, 47, 26);
                CanvasHelper::overlay(&mut card, &resized, 217, 80);
            }
        }

        // 6. Texts (Exact colors & layout matching original)
        let diff_text_color = if diff_idx == 4 {
            Rgba([138, 0, 226, 255])
        } else {
            Rgba([255, 255, 255, 255])
        };
        let id_colors = [
            Rgba([129, 217, 85, 255]),
            Rgba([245, 189, 21, 255]),
            Rgba([255, 129, 141, 255]),
            Rgba([159, 81, 220, 255]),
            Rgba([138, 0, 226, 255]),
        ];
        let id_text_color = id_colors[diff_idx];

        if let Some(torus) = &FONTS.torus {
            // ID (at 26, 98, size 13)
            CanvasHelper::draw_text(
                &mut card,
                id_text_color,
                26,
                98,
                13.0,
                torus,
                &record.song_id.to_string(),
                "mm",
            );
            // Achievement (at 93, 38, size 30)
            CanvasHelper::draw_text(
                &mut card,
                diff_text_color,
                93,
                38,
                30.0,
                torus,
                &format!("{:.4}%", record.achievements),
                "lm",
            );
            // Rating (at 93, 65, size 16.5)
            let ds_str = format!("{:.1}", record.ds);
            CanvasHelper::draw_text(
                &mut card,
                diff_text_color,
                93,
                65,
                16.5,
                torus,
                &format!("{} -> {}", ds_str, record.dx_rating),
                "lm",
            );
            // DX Score / Max DX (at 219, 65, size 16.5)
            let dx_str = if max_dx > 0 {
                format!("{}/{}", record.dx_score, max_dx)
            } else {
                record.dx_score.to_string()
            };
            CanvasHelper::draw_text(
                &mut card,
                diff_text_color,
                219,
                65,
                16.5,
                torus,
                &dx_str,
                "mm",
            );
        }

        // Song Title using FOT-NewRodin Pro EB (pixel-perfect boundary truncation)
        if let Some(fot) = &FONTS.fot_new_rodin {
            let title = CanvasHelper::truncate_title_by_font(&record.title, fot, 14.0, 160.0);
            CanvasHelper::draw_text(&mut card, diff_text_color, 93, 14, 14.0, fot, &title, "lm");
        }

        card
    }

    pub fn render_b50(b50: &PlayerB50) -> Result<RgbaImage, AppError> {
        Self::render_b50_with_theme(b50, B50Theme::Circle)
    }

    pub fn render_b50_with_theme(b50: &PlayerB50, theme: B50Theme) -> Result<RgbaImage, AppError> {
        let b50_bg_path = get_pic_path(&format!("{}/b50.png", theme.as_str()));
        let mut base_img = load_image(&b50_bg_path)
            .unwrap_or_else(|| image::ImageBuffer::from_pixel(1400, 1600, Rgba([240, 240, 250, 255])));

        // 1. Header: Logo (14, 60, 249x120)
        let logo_path = format!("{}/logo.png", theme.as_str());
        if let Some(logo) = load_image(&get_pic_path(&logo_path)) {
            let resized = CanvasHelper::resize(&logo, 249, 120);
            CanvasHelper::overlay(&mut base_img, &resized, 14, 60);
        }

        // 2. Header: Plate (300, 60, 800x130)
        let default_plate = get_pic_path("UI_Plate_550101.png");
        if let Some(plate) = load_image(&default_plate) {
            let resized = CanvasHelper::resize(&plate, 800, 130);
            CanvasHelper::overlay(&mut base_img, &resized, 300, 60);
        }

        // 3. Header: Icon (305, 65, 120x120)
        let default_icon = get_pic_path("UI_Icon_509506.png");
        if let Some(icon) = load_image(&default_icon) {
            let resized = CanvasHelper::resize(&icon, 120, 120);
            CanvasHelper::overlay(&mut base_img, &resized, 305, 65);
        }

        // 4. Header: Name Base (435, 115)
        if let Some(name_bg) = load_image(&get_pic_path("Name.png")) {
            CanvasHelper::overlay(&mut base_img, &name_bg, 435, 115);
        }

        // 5. Header: Dan (625, 120, 80x32) & Class (620, 60, 90x54)
        let cr = b50.course_rank;
        let dan_num = if cr <= 10 { cr } else { cr + 1 };
        let dan_file = format!("UI_DNM_DaniPlate_{:02}.png", dan_num);
        if let Some(dan) = load_image(&get_pic_path(&dan_file)) {
            let resized = CanvasHelper::resize(&dan, 80, 32);
            CanvasHelper::overlay(&mut base_img, &resized, 625, 120);
        }
        let class_file = format!("UI_FBR_Class_{:02}.png", b50.class_rank);
        if let Some(class_img) = load_image(&get_pic_path(&class_file)) {
            let resized = CanvasHelper::resize(&class_img, 90, 54);
            CanvasHelper::overlay(&mut base_img, &resized, 620, 60);
        }

        // 6. Header: Rating Frame & Digits
        let ra_pic_name = Self::find_ra_pic(b50.rating, theme);
        let dx_rating_size = if theme == B50Theme::Circle && b50.rating >= 14000 {
            (170, 35)
        } else {
            (186, 35)
        };
        if let Some(ra_frame) = load_image(&get_pic_path(&ra_pic_name)) {
            let resized = CanvasHelper::resize(&ra_frame, dx_rating_size.0, dx_rating_size.1);
            CanvasHelper::overlay(&mut base_img, &resized, 435, 72);
        }

        if let Some(star_path) = Self::ra_pic_star(b50.rating, theme) {
            if let Some(star_img) = load_image(&get_pic_path(&star_path)) {
                let resized = CanvasHelper::resize(&star_img, 21, 35);
                CanvasHelper::overlay(&mut base_img, &resized, 590, 72);
            }
        }

        let (rating_num_x, rating_num_y, rating_num_gap, rating_num_size) = if theme == B50Theme::Circle && b50.rating >= 14000 {
            (515, 82, 13, (14, 17))
        } else {
            (520, 80, 15, (17, 20))
        };

        let rating_str = format!("{:05}", b50.rating);
        for (n, ch) in rating_str.chars().enumerate() {
            let digit_file = format!("UI_NUM_Drating_{ch}.png");
            if let Some(digit_img) = load_image(&get_pic_path(&digit_file)) {
                let resized = CanvasHelper::resize(&digit_img, rating_num_size.0, rating_num_size.1);
                CanvasHelper::overlay(
                    &mut base_img,
                    &resized,
                    rating_num_x + (rating_num_gap * n as i64),
                    rating_num_y,
                );
            }
        }

        // 7. Shougou / Trophy (435, 160, 270x27)
        let shougou_path = CONFIG.static_dir.join("mai/shougou/UI_CMN_Shougou_Rainbow.png");
        if let Some(shougou) = load_image(&shougou_path) {
            let resized = CanvasHelper::resize(&shougou, 270, 27);
            CanvasHelper::overlay(&mut base_img, &resized, 435, 160);
        }

        // 8. Texts (Nickname, Trophy Text, Designer Credits)
        let footer_color = match theme {
            B50Theme::Circle => Rgba([249, 62, 172, 255]),
            B50Theme::PrismPlus => Rgba([124, 129, 255, 255]),
        };

        if let Some(fot) = &FONTS.fot_new_rodin {
            CanvasHelper::draw_text(
                &mut base_img,
                Rgba([0, 0, 0, 255]),
                445,
                135,
                20.0,
                fot,
                &b50.nickname,
                "lm",
            );
            CanvasHelper::draw_text_with_stroke(
                &mut base_img,
                footer_color,
                700,
                1570,
                22.0,
                fot,
                "Designed by Yuri-YuzuChaN & BlueDeer233. Data from DivingFish. Generated by maimai-mcp BOT",
                "mm",
                5,
                Rgba([255, 255, 255, 255]),
            );
        }

        if let Some(torus) = &FONTS.torus {
            let trophy_text = format!("B35: {} + B15: {} = {}", b50.b35_rating, b50.b15_rating, b50.rating);
            CanvasHelper::draw_text(
                &mut base_img,
                Rgba([0, 0, 0, 255]),
                570,
                172,
                14.0,
                torus,
                &trophy_text,
                "mm",
            );
        }

        // 9. Parallel Rayon render for B35 cards
        let b35_cards: Vec<RgbaImage> = b50.b35.par_iter().map(|rec| Self::render_score_card(rec, theme)).collect();
        for (num, card) in b35_cards.into_iter().enumerate() {
            let row = (num / 5) as i64;
            let col = (num % 5) as i64;
            let x = 16 + col * 276;
            let y = 235 + row * 114;
            CanvasHelper::overlay(&mut base_img, &card, x, y);
        }

        // 10. Parallel Rayon render for B15 cards
        let b15_cards: Vec<RgbaImage> = b50.b15.par_iter().map(|rec| Self::render_score_card(rec, theme)).collect();
        for (num, card) in b15_cards.into_iter().enumerate() {
            let row = (num / 5) as i64;
            let col = (num % 5) as i64;
            let x = 16 + col * 276;
            let y = 1085 + row * 114;
            CanvasHelper::overlay(&mut base_img, &card, x, y);
        }

        Ok(base_img)
    }
}
