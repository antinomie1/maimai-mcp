use crate::config::CONFIG;
use crate::core::database::music_db::MUSIC_DB;
use crate::core::models::score::{ComboStatus, ScoreRecord};
use crate::draw::assets::{get_cover_path, get_pic_path, load_image, FONTS};
use crate::draw::canvas::CanvasHelper;
use crate::error::AppError;
use image::{Rgba, RgbaImage};
use std::collections::{BTreeMap, HashMap};

pub struct RatingTableRenderer;

impl RatingTableRenderer {
    pub fn format_rank_str(rate: &str) -> &'static str {
        match rate.to_lowercase().as_str() {
            "sssp" | "sss+" => "SSSp",
            "sss" => "SSS",
            "ssp" | "ss+" => "SSp",
            "ss" => "SS",
            "sp" | "s+" => "Sp",
            "s" => "S",
            "aaa" => "AAA",
            "aa" => "AA",
            "a" => "A",
            "bbb" => "BBB",
            "bb" => "BB",
            "b" => "B",
            "c" => "C",
            _ => "D",
        }
    }

    pub fn render(rating_level: &str, scores: &[ScoreRecord], is_fc_plan: bool) -> Result<RgbaImage, AppError> {
        let table_bg_path = CONFIG.static_dir.join("mai/rating_table").join(format!("{rating_level}.png"));
        let font_color = Rgba([114, 188, 254, 255]);
        let white_color = Rgba([255, 255, 255, 255]);

        let fot = FONTS.fot_new_rodin.as_ref().or(FONTS.siyuan.as_ref())
            .ok_or_else(|| AppError::Other("FOT-NewRodin font not found".into()))?;
        let tb = FONTS.torus.as_ref().or(FONTS.siyuan.as_ref())
            .ok_or_else(|| AppError::Other("Torus font not found".into()))?;

        // 1. Fetch all charts in this rating level from database
        let mut group_by_ds: BTreeMap<String, Vec<(u32, usize, f64)>> = BTreeMap::new();
        {
            let db = MUSIC_DB.read().unwrap();
            for song in &db.songs {
                if song.song_type == crate::core::models::music::SongType::Utage {
                    continue;
                }
                let song_id_num: u32 = song.id.parse().unwrap_or(0);
                if song_id_num >= 100000 {
                    continue;
                }
                for (diff_idx, lv_str) in song.level.iter().enumerate() {
                    if lv_str == rating_level {
                        let ds_val = song.ds.get(diff_idx).copied().unwrap_or(0.0);
                        let ds_key = format!("{:.1}", ds_val);
                        group_by_ds.entry(ds_key).or_default().push((song_id_num, diff_idx, ds_val));
                    }
                }
            }
        }

        // Map scores by (song_id, diff_index)
        let mut score_map: HashMap<(u32, usize), &ScoreRecord> = HashMap::new();
        for s in scores {
            score_map.insert((s.song_id, s.level_index as usize), s);
        }

        // Compute row layout
        let mut total_rows = 0;
        for list in group_by_ds.values() {
            if !list.is_empty() {
                total_rows += ((list.len() - 1) / 14 + 1) as i32;
            }
        }

        let is_prebaked = table_bg_path.exists();
        let dynamic_h = (450 + total_rows * 85 + (group_by_ds.len() as i32 * 30) + 230).max(1200) as u32;

        let mut base_img = if is_prebaked {
            load_image(&table_bg_path).unwrap()
        } else {
            CanvasHelper::generate_table_bg(1400, dynamic_h, 360)
        };

        // 2. Level header title (exact original position: 495, 160 and 750, 160)
        CanvasHelper::draw_text_with_stroke(&mut base_img, font_color, 495, 160, 70.0, fot, "Level.", "lm", 8, white_color);
        CanvasHelper::draw_text_with_stroke(&mut base_img, font_color, 750, 160, 100.0, fot, rating_level, "lm", 8, white_color);

        // 3. Render grid of songs (strictly preserving start_y = 450, start_x = 140, gap = 85, row_count = 14)
        let complete_1_img = load_image(&get_pic_path("complete_1.png"));
        let unfinished_1_img = load_image(&get_pic_path("unfinished_1.png"));

        let row_count = 14;
        let gap = 85;
        let mut current_y = 450;

        for (ds, songs_in_ds) in group_by_ds.iter().rev() {
            if !is_prebaked {
                let ds_dot = ds.split('.').last().unwrap_or("0");
                CanvasHelper::draw_text_with_stroke(
                    &mut base_img,
                    font_color,
                    70,
                    current_y + 35,
                    40.0,
                    fot,
                    &format!(".{ds_dot}"),
                    "lm",
                    4,
                    white_color,
                );
            }

            for (num, &(song_id, diff_idx, _level_val)) in songs_in_ds.iter().enumerate() {
                let row = (num / row_count) as i32;
                let col = (num % row_count) as i32;
                let x = 140 + col * gap;
                let y = current_y + row * gap;

                if !is_prebaked {
                    let cover_path = get_cover_path(song_id);
                    if let Some(cover) = load_image(&cover_path) {
                        let resized = CanvasHelper::resize(&cover, 75, 75);
                        CanvasHelper::overlay(&mut base_img, &resized, x as i64, y as i64);
                    }
                    let diff_s = match diff_idx {
                        0 => "basic",
                        1 => "advanced",
                        2 => "expert",
                        3 => "master",
                        _ => "remaster",
                    };
                    let table_diff_bg_path = get_pic_path(&format!("border_{diff_s}.png"));
                    if let Some(td_bg) = load_image(&table_diff_bg_path) {
                        CanvasHelper::overlay(&mut base_img, &td_bg, (x - 5) as i64, (y - 5) as i64);
                    }
                    let diff_color = match diff_idx {
                        4 => Rgba([138, 0, 226, 255]),
                        _ => Rgba([255, 255, 255, 255]),
                    };
                    CanvasHelper::draw_text(&mut base_img, diff_color, x + 56, y + 4, 13.0, tb, &song_id.to_string(), "mm");
                }

                if let Some(record) = score_map.get(&(song_id, diff_idx)) {
                    if !is_fc_plan {
                        let bg = if record.achievements >= 100.0 {
                            complete_1_img.as_ref()
                        } else {
                            unfinished_1_img.as_ref()
                        };
                        if let Some(bg_img) = bg {
                            CanvasHelper::overlay(&mut base_img, bg_img, (x + 1) as i64, (y + 1) as i64);
                        }

                        let rank_str = Self::format_rank_str(&record.rate);
                        let rank_icon_path = get_pic_path(&format!("prism_plus/UI_TTR_Rank_{rank_str}.png"));
                        if let Some(rank_img) = load_image(&rank_icon_path) {
                            let resized = CanvasHelper::resize(&rank_img, 78, 35);
                            CanvasHelper::overlay(&mut base_img, &resized, x as i64, (y + 20) as i64);
                        }
                    } else if record.fc != ComboStatus::None {
                        if let Some(bg_img) = complete_1_img.as_ref() {
                            CanvasHelper::overlay(&mut base_img, bg_img, (x + 1) as i64, (y + 1) as i64);
                        }
                        let fc_icon_name = match record.fc {
                            ComboStatus::FC => "FC",
                            ComboStatus::FCP => "FCp",
                            ComboStatus::AP => "AP",
                            ComboStatus::APP => "APp",
                            ComboStatus::None => "FC",
                        };
                        let fc_path = get_pic_path(&format!("UI_MSS_MBase_Icon_{fc_icon_name}.png"));
                        if let Some(fc_img) = load_image(&fc_path) {
                            let resized = CanvasHelper::resize(&fc_img, 50, 50);
                            CanvasHelper::overlay(&mut base_img, &resized, (x + 15) as i64, (y + 13) as i64);
                        }
                    }
                }
            }

            let group_rows = ((songs_in_ds.len() - 1) / row_count + 1) as i32;
            current_y += group_rows * gap + 30;
        }

        // Footer text
        let footer_text = "Designed by Yuri-YuzuChaN & BlueDeer233. maimai-mcp Rust Engine";
        CanvasHelper::draw_text(
            &mut base_img,
            font_color,
            700,
            (dynamic_h as i32) - 75,
            30.0,
            fot,
            footer_text,
            "mm",
        );

        // Final downscale to 80% with Lanczos3
        let final_w = (base_img.width() as f64 * 0.8) as u32;
        let final_h = (base_img.height() as f64 * 0.8) as u32;
        let final_img = CanvasHelper::resize(&base_img, final_w, final_h);

        Ok(final_img)
    }
}
