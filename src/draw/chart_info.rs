use crate::core::calc::rating::compute_single_rating;
use crate::core::models::music::{Song, SongType};
use crate::draw::assets::{get_cover_path, get_pic_path, load_image, FONTS};
use crate::draw::b50::B50Theme;
use crate::draw::canvas::CanvasHelper;
use crate::error::AppError;
use image::{Rgba, RgbaImage};

pub struct ChartInfoRenderer;

impl ChartInfoRenderer {
    pub fn get_best_rating(level_value: f64) -> Vec<u32> {
        let thresholds = [97.0, 98.0, 99.0, 99.5, 100.0, 100.5];
        let mut ra: Vec<u32> = thresholds
            .iter()
            .map(|&ach| compute_single_rating(level_value, ach))
            .collect();
        ra.push(compute_single_rating(level_value, 100.5) + 1);
        ra.sort_by(|a, b| b.cmp(a));
        ra
    }

    pub fn render_chart_info(song: &Song, theme: B50Theme) -> Result<RgbaImage, AppError> {
        let bg_path = get_pic_path(&format!("{}/chart_info.png", theme.as_str()));
        let mut base_img = load_image(&bg_path)
            .ok_or_else(|| AppError::Io(format!("Failed to load chart_info bg: {:?}", bg_path)))?;

        let theme_color = match theme {
            B50Theme::Circle => Rgba([249, 62, 172, 255]),
            B50Theme::PrismPlus => Rgba([124, 129, 255, 255]),
        };
        let white_color = Rgba([255, 255, 255, 255]);

        let fot = FONTS.fot_new_rodin.as_ref().or(FONTS.siyuan.as_ref())
            .ok_or_else(|| AppError::Other("FOT-NewRodin font not found".into()))?;
        let siyuan = FONTS.siyuan.as_ref()
            .ok_or_else(|| AppError::Other("SiYuan font not found".into()))?;

        // 1. Logo
        let logo_path = get_pic_path(&format!("{}/logo.png", theme.as_str()));
        if let Some(logo) = load_image(&logo_path) {
            let resized = CanvasHelper::resize(&logo, 249, 120);
            CanvasHelper::overlay(&mut base_img, &resized, 65, 25);
        }

        // 2. New Song Banner
        if song.basic_info.is_new {
            let new_path = get_pic_path("UI_CMN_TabTitle_NewSong.png");
            if let Some(new_img) = load_image(&new_path) {
                let resized = CanvasHelper::resize(&new_img, 249, 120);
                CanvasHelper::overlay(&mut base_img, &resized, 842, 100);
            }
        }

        // 3. Cover
        let song_id_num: u32 = song.id.parse().unwrap_or(0);
        let cover_path = get_cover_path(song_id_num);
        if let Some(cover) = load_image(&cover_path) {
            let resized = CanvasHelper::resize(&cover, 242, 242);
            CanvasHelper::overlay(&mut base_img, &resized, 133, 197);
        }

        // 4. Version banner
        let ver_name = if song.basic_info.version.is_empty() {
            "maimai でらっくす"
        } else {
            &song.basic_info.version
        };
        let ver_path = get_pic_path(&format!("{ver_name}.png"));
        if let Some(ver_img) = load_image(&ver_path) {
            let resized = CanvasHelper::resize(&ver_img, 182, 90);
            CanvasHelper::overlay(&mut base_img, &resized, 800, 370);
        }

        // 5. Type banner (SD/DX)
        let type_str = match song.song_type {
            SongType::Standard => "SD",
            SongType::Deluxe => "DX",
            SongType::Utage => "UTAGE",
        };
        let type_path = get_pic_path(&format!("{type_str}.png"));
        if let Some(type_img) = load_image(&type_path) {
            let resized = CanvasHelper::resize(&type_img, 80, 30);
            CanvasHelper::overlay(&mut base_img, &resized, 295, 410);
        }

        // 6. Title
        let title_truncated = CanvasHelper::truncate_title_by_font(&song.title, fot, 28.0, 480.0);
        CanvasHelper::draw_text(&mut base_img, theme_color, 405, 220, 28.0, fot, &title_truncated, "lm");

        // 7. Artist
        let artist_truncated = CanvasHelper::truncate_title_by_font(&song.basic_info.artist, fot, 20.0, 500.0);
        CanvasHelper::draw_text(&mut base_img, theme_color, 407, 265, 20.0, fot, &artist_truncated, "lm");

        // 8. BPM
        CanvasHelper::draw_text(&mut base_img, theme_color, 460, 345, 24.0, fot, &song.basic_info.bpm.to_string(), "lm");

        // 9. ID
        CanvasHelper::draw_text(&mut base_img, theme_color, 405, 435, 22.0, fot, &format!("ID {}", song.id), "lm");

        // 10. Genre
        let genre = if song.basic_info.genre.is_empty() { "maimai" } else { &song.basic_info.genre };
        let genre_trunc = CanvasHelper::truncate_title_by_font(genre, fot, 28.0, 200.0);
        CanvasHelper::draw_text(&mut base_img, theme_color, 665, 435, 28.0, fot, &genre_trunc, "mm");

        // 11. Difficulties
        for index in 0..song.ds.len().min(5) {
            let spacing = 70 * index as i32;
            let level_str = song.level.get(index).cloned().unwrap_or_else(|| "-".into());
            let ds_val = song.ds.get(index).copied().unwrap_or(0.0);

            // Level (ds)
            let lv_text = format!("{level_str}({ds_val:.1})");
            CanvasHelper::draw_text(&mut base_img, white_color, 120, 590 + spacing, 22.0, fot, &lv_text, "mm");

            // Fit diff
            let fit_text = "-";
            CanvasHelper::draw_text(&mut base_img, white_color, 120, 613 + spacing, 15.0, fot, fit_text, "mm");

            // Charter / Note Designer
            let charter = song.charts.get(index).and_then(|c| c.charter.as_deref()).unwrap_or("-");
            let charter_trunc = CanvasHelper::truncate_title_by_font(charter, fot, 28.0, 220.0);
            CanvasHelper::draw_text(&mut base_img, theme_color, 310, 590 + spacing, 28.0, fot, &charter_trunc, "mm");

            // Notes: total, tap, hold, slide, touch, break
            let notes_vec = song.charts.get(index).map(|c| &c.notes);
            let notes_counts = if let Some(n_vec) = notes_vec {
                if n_vec.len() >= 5 {
                    let total: u32 = n_vec.iter().sum();
                    [
                        total.to_string(),
                        n_vec[0].to_string(),
                        n_vec[1].to_string(),
                        n_vec[2].to_string(),
                        n_vec[3].to_string(),
                        n_vec[4].to_string(),
                    ]
                } else if n_vec.len() == 4 {
                    let total: u32 = n_vec.iter().sum();
                    [
                        total.to_string(),
                        n_vec[0].to_string(),
                        n_vec[1].to_string(),
                        n_vec[2].to_string(),
                        "-".to_string(),
                        n_vec[3].to_string(),
                    ]
                } else {
                    ["-".into(), "-".into(), "-".into(), "-".into(), "-".into(), "-".into()]
                }
            } else {
                ["-".into(), "-".into(), "-".into(), "-".into(), "-".into(), "-".into()]
            };

            for (n, n_val) in notes_counts.iter().enumerate() {
                let x = 480 + 122 * n as i32;
                let y = 590 + spacing;
                CanvasHelper::draw_text(&mut base_img, theme_color, x, y, 25.0, fot, n_val, "mm");
            }

            // Rating Targets for Expert, Master, Re:Master (index >= 2)
            if index >= 2 {
                let ra_list = Self::get_best_rating(ds_val);
                for (_n, ra_val) in ra_list.iter().enumerate() {
                    let rx = 295 + 125 * _n as i32;
                    let ry = 1017 + 46 * (index as i32 - 2);
                    CanvasHelper::draw_text(&mut base_img, theme_color, rx, ry, 22.0, fot, &ra_val.to_string(), "mm");
                }
            }
        }

        // Footer note
        CanvasHelper::draw_text(&mut base_img, theme_color, 295, 985, 12.0, siyuan, "*未实装", "mm");
        let footer_text = "Designed by Yuri-YuzuChaN & BlueDeer233. maimai-mcp Rust Engine";
        CanvasHelper::draw_text_with_stroke(
            &mut base_img,
            theme_color,
            600,
            1220,
            25.0,
            fot,
            footer_text,
            "mm",
            3,
            white_color,
        );

        Ok(base_img)
    }
}
