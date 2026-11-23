use crate::core::models::music::{Song, SongType};
use crate::core::models::score::{ComboStatus, ScoreRecord, SyncStatus};
use crate::draw::assets::{get_cover_path, get_pic_path, load_image, FONTS};
use crate::draw::b50::B50Theme;
use crate::draw::canvas::CanvasHelper;
use crate::error::AppError;
use image::{Rgba, RgbaImage};

pub struct PlayScoreRenderer;

impl PlayScoreRenderer {
    pub fn render_song_play_data(
        song: &Song,
        play_results: &[Option<ScoreRecord>],
        theme: B50Theme,
    ) -> Result<RgbaImage, AppError> {
        let bg_path = get_pic_path(&format!("{}/play_info.png", theme.as_str()));
        let mut base_img = load_image(&bg_path)
            .ok_or_else(|| AppError::Io(format!("Play info template not found: {:?}", bg_path)))?;

        let theme_color = match theme {
            B50Theme::Circle => Rgba([249, 62, 172, 255]),
            B50Theme::PrismPlus => Rgba([124, 129, 255, 255]),
        };
        let white_color = Rgba([255, 255, 255, 255]);

        let fot = FONTS.fot_new_rodin.as_ref().or(FONTS.siyuan.as_ref())
            .ok_or_else(|| AppError::Other("FOT-NewRodin font not found".into()))?;
        let siyuan = FONTS.siyuan.as_ref()
            .ok_or_else(|| AppError::Other("SiYuan font not found".into()))?;
        let tb = FONTS.torus.as_ref().or(FONTS.siyuan.as_ref())
            .ok_or_else(|| AppError::Other("Torus font not found".into()))?;

        // 1. Logo (249x120 at (42, 34))
        let logo_path = get_pic_path(&format!("{}/logo.png", theme.as_str()));
        if let Some(logo) = load_image(&logo_path) {
            let resized = CanvasHelper::resize(&logo, 249, 120);
            CanvasHelper::overlay(&mut base_img, &resized, 42, 34);
        }

        // 2. Cover (300x300 at (100, 260))
        let song_id_num: u32 = song.id.parse().unwrap_or(0);
        let cover_path = get_cover_path(song_id_num);
        if let Some(cover) = load_image(&cover_path) {
            let resized = CanvasHelper::resize(&cover, 300, 300);
            CanvasHelper::overlay(&mut base_img, &resized, 100, 260);
        }

        // 3. Version banner (183x90 at (295, 205))
        let ver_name = if song.basic_info.version.is_empty() {
            "maimai でらっくす"
        } else {
            &song.basic_info.version
        };
        let ver_path = get_pic_path(&format!("{ver_name}.png"));
        if let Some(ver_img) = load_image(&ver_path) {
            let resized = CanvasHelper::resize(&ver_img, 183, 90);
            CanvasHelper::overlay(&mut base_img, &resized, 295, 205);
        }

        // 4. Type banner (55x20 at (350, 560))
        let type_str = match song.song_type {
            SongType::Standard => "SD",
            SongType::Deluxe => "DX",
            SongType::Utage => "UTAGE",
        };
        let type_path = get_pic_path(&format!("{type_str}.png"));
        if let Some(type_img) = load_image(&type_path) {
            let resized = CanvasHelper::resize(&type_img, 55, 20);
            CanvasHelper::overlay(&mut base_img, &resized, 350, 560);
        }

        // 5. Artist (255, 595)
        let artist_trunc = CanvasHelper::truncate_title_by_font(&song.basic_info.artist, siyuan, 20.0, 320.0);
        CanvasHelper::draw_text(&mut base_img, theme_color, 255, 595, 20.0, siyuan, &artist_trunc, "mm");

        // 6. Title (255, 625)
        let title_trunc = CanvasHelper::truncate_title_by_font(&song.title, fot, 26.0, 320.0);
        CanvasHelper::draw_text(&mut base_img, theme_color, 255, 625, 26.0, fot, &title_trunc, "mm");

        // 7. ID & BPM
        CanvasHelper::draw_text(&mut base_img, theme_color, 160, 720, 22.0, fot, &song.id, "mm");
        CanvasHelper::draw_text(&mut base_img, theme_color, 380, 720, 22.0, fot, &song.basic_info.bpm.to_string(), "mm");

        // 8. Difficulties (0..=4)
        let y_step = 100;
        let diff_count = song.ds.len().min(5);
        for num in 0..diff_count {
            let y_pos = 235 + y_step * num as i32;

            // Difficulty indicator icon d_{num}.png at (650, y_pos)
            let d_path = get_pic_path(&format!("d_{num}.png"));
            if let Some(d_img) = load_image(&d_path) {
                CanvasHelper::overlay(&mut base_img, &d_img, 650, y_pos as i64);
            }

            let ds_val = song.ds.get(num).copied().unwrap_or(0.0);
            let ds_str = format!("{ds_val:.1}");

            if let Some(Some(rec)) = play_results.get(num) {
                // ra_dx.png at (850, 272 + y_step * num)
                let ra_dx_path = get_pic_path(&format!("{}/ra_dx.png", theme.as_str()));
                if let Some(ra_dx) = load_image(&ra_dx_path) {
                    let resized = CanvasHelper::resize(&ra_dx, 102, 44);
                    CanvasHelper::overlay(&mut base_img, &resized, 850, (272 + y_step * num as i32) as i64);
                }

                // Rank icon at (737, 272 + y_step * num)
                let rank_path = get_pic_path(&format!("{}/UI_TTR_Rank_{}.png", theme.as_str(), rec.rate));
                if let Some(rank_img) = load_image(&rank_path) {
                    let resized = CanvasHelper::resize(&rank_img, 100, 45);
                    CanvasHelper::overlay(&mut base_img, &resized, 737, (272 + y_step * num as i32) as i64);
                }

                // FC / FS icons
                let fcfs_path = get_pic_path("fcfs.png");
                if let Some(fcfs) = load_image(&fcfs_path) {
                    CanvasHelper::overlay(&mut base_img, &fcfs, 965, (265 + y_step * num as i32) as i64);
                }

                if rec.fc != ComboStatus::None {
                    let fc_name = match rec.fc {
                        ComboStatus::FC => "FC",
                        ComboStatus::FCP => "FCp",
                        ComboStatus::AP => "AP",
                        ComboStatus::APP => "APp",
                        ComboStatus::None => "FC",
                    };
                    let fc_path = get_pic_path(&format!("UI_CHR_PlayBonus_{fc_name}.png"));
                    if let Some(fc_img) = load_image(&fc_path) {
                        let resized = CanvasHelper::resize(&fc_img, 65, 65);
                        CanvasHelper::overlay(&mut base_img, &resized, 960, (261 + y_step * num as i32) as i64);
                    }
                }

                if rec.fs != SyncStatus::None {
                    let fs_name = match rec.fs {
                        SyncStatus::Sync => "Sync",
                        SyncStatus::FS => "FS",
                        SyncStatus::FSP => "FSp",
                        SyncStatus::FDX => "FSD",
                        SyncStatus::FDXP => "FSDp",
                        SyncStatus::None => "FS",
                    };
                    let fs_path = get_pic_path(&format!("UI_CHR_PlayBonus_{fs_name}.png"));
                    if let Some(fs_img) = load_image(&fs_path) {
                        let resized = CanvasHelper::resize(&fs_img, 65, 65);
                        CanvasHelper::overlay(&mut base_img, &resized, 1025, (261 + y_step * num as i32) as i64);
                    }
                }

                // Achievement
                let ach_str = format!("{:.4}%", rec.achievements);
                CanvasHelper::draw_text(&mut base_img, theme_color, 500, 295 + y_step * num as i32, 30.0, fot, &ach_str, "lm");
                CanvasHelper::draw_text(&mut base_img, white_color, 685, 248 + y_step * num as i32, 20.0, fot, &ds_str, "mm");
                CanvasHelper::draw_text(&mut base_img, theme_color, 915, 283 + y_step * num as i32, 18.0, tb, &rec.dx_rating.to_string(), "mm");

                let max_dx = song.max_dx_score(num);
                let dx_str = format!("{}/{}", rec.dx_score, max_dx);
                CanvasHelper::draw_text(&mut base_img, theme_color, 916, 304 + y_step * num as i32, 13.0, tb, &dx_str, "mm");
            } else {
                CanvasHelper::draw_text(&mut base_img, white_color, 685, 248 + y_step * num as i32, 25.0, fot, &ds_str, "mm");
                CanvasHelper::draw_text(&mut base_img, theme_color, 800, 302 + y_step * num as i32, 30.0, siyuan, "未游玩", "mm");
            }
        }

        if diff_count == 4 {
            CanvasHelper::draw_text(&mut base_img, theme_color, 800, 302 + y_step * 4, 30.0, siyuan, "没有该难度", "mm");
        }

        // Footer
        let footer_text = "Designed by Yuri-YuzuChaN & BlueDeer233. maimai-mcp Rust Engine";
        CanvasHelper::draw_text_with_stroke(&mut base_img, theme_color, 600, 830, 20.0, fot, footer_text, "mm", 3, white_color);

        Ok(base_img)
    }
}
