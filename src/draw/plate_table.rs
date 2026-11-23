use crate::config::CONFIG;
use crate::core::models::music::Song;
use crate::core::models::score::{ComboStatus, ScoreRecord, SyncStatus};
use crate::draw::assets::{get_cover_path, get_pic_path, load_image, FONTS};
use crate::draw::canvas::CanvasHelper;
use crate::error::AppError;
use image::{Rgba, RgbaImage};
use std::collections::{HashMap, HashSet};

pub struct PlateTableRenderer;

impl PlateTableRenderer {
    pub const LEVEL_ORDER: [&'static str; 23] = [
        "15", "14+", "14", "13+", "13", "12+", "12", "11+", "11", "10+", "10",
        "9+", "9", "8+", "8", "7+", "7", "6", "5", "4", "3", "2", "1",
    ];

    pub const ID_TEXT_COLORS: [Rgba<u8>; 5] = [
        Rgba([129, 217, 85, 255]),   // Basic (Green)
        Rgba([245, 189, 21, 255]),   // Advanced (Yellow)
        Rgba([255, 129, 141, 255]),  // Expert (Red/Pink)
        Rgba([159, 81, 220, 255]),   // Master (Purple)
        Rgba([138, 0, 226, 255]),   // Re:Master (Deep Purple)
    ];

    pub fn is_qualified(record: Option<&ScoreRecord>, plan: &str) -> bool {
        let rec = match record {
            Some(r) => r,
            None => return false,
        };
        match plan {
            "者" | "霸者" => rec.achievements >= 80.0,
            "将" => rec.achievements >= 100.0,
            "极" | "極" => rec.fc != ComboStatus::None,
            "神" => rec.fc == ComboStatus::AP || rec.fc == ComboStatus::APP,
            "舞舞" => matches!(rec.fs, SyncStatus::FDX | SyncStatus::FDXP),
            _ => false,
        }
    }

    pub fn render_plate_table(
        version: &str,
        plan: &str,
        song_ids: &[u32],
        scores: &[ScoreRecord],
    ) -> Result<RgbaImage, AppError> {
        let is_wu = version == "舞" || version == "霸" || version == "霸者";
        let slot_num = if is_wu { 5 } else { 4 };

        let font_color = Rgba([114, 188, 254, 255]);
        let white_color = Rgba([255, 255, 255, 255]);
        let default_text_color = Rgba([255, 255, 255, 255]);

        let fot = FONTS.fot_new_rodin.as_ref().or(FONTS.siyuan.as_ref())
            .ok_or_else(|| AppError::Other("FOT-NewRodin font not found".into()))?;
        let tb = FONTS.torus.as_ref().or(FONTS.siyuan.as_ref())
            .ok_or_else(|| AppError::Other("Torus font not found".into()))?;

        // 1. Fetch songs from database
        let mut songs_map: HashMap<u32, Song> = HashMap::new();
        {
            let db = crate::core::database::music_db::MUSIC_DB.read().unwrap();
            for &sid in song_ids {
                if sid >= 100000 {
                    continue;
                }
                if let Some(song) = db.get_by_id(&sid.to_string()) {
                    if song.song_type == crate::core::models::music::SongType::Utage {
                        continue;
                    }
                    songs_map.insert(sid, song.clone());
                }
            }
        }

        // Map user scores by (song_id, diff_index)
        let mut score_map: HashMap<(u32, usize), &ScoreRecord> = HashMap::new();
        for s in scores {
            score_map.insert((s.song_id, s.level_index as usize), s);
        }

        // 2. Group songs by Master difficulty level
        let mut level_dict: HashMap<&str, Vec<&Song>> = HashMap::new();
        for lv in Self::LEVEL_ORDER {
            level_dict.insert(lv, Vec::new());
        }

        for song in songs_map.values() {
            let master_level = song.level.get(3).map(|s| s.as_str()).unwrap_or("1");
            if let Some(list) = level_dict.get_mut(master_level) {
                list.push(song);
            }
        }

        // Sort songs in each level group by Master constant (ds) descending
        for list in level_dict.values_mut() {
            list.sort_by(|a, b| {
                let ds_a = a.ds.get(3).copied().unwrap_or(0.0);
                let ds_b = b.ds.get(3).copied().unwrap_or(0.0);
                ds_b.partial_cmp(&ds_a).unwrap_or(std::cmp::Ordering::Equal)
            });
        }

        // 3. Compute stats
        let total_songs_count = songs_map.len();
        let mut slot_qualified_counts = vec![0usize; slot_num];
        let mut finished_songs: HashSet<u32> = HashSet::new();

        for song in songs_map.values() {
            let sid: u32 = song.id.parse().unwrap_or(0);
            let mut all_slots_qualified = true;
            let available_diffs = song.ds.len().min(slot_num);

            for slot in 0..slot_num {
                if slot < available_diffs {
                    let rec = score_map.get(&(sid, slot)).copied();
                    if Self::is_qualified(rec, plan) {
                        slot_qualified_counts[slot] += 1;
                    } else {
                        all_slots_qualified = false;
                    }
                }
            }

            if all_slots_qualified {
                finished_songs.insert(sid);
            }
        }

        let completed_songs_count = finished_songs.len();

        // 4. Calculate dynamic height
        let row_count = 12;
        let gap = 96;
        let start_x = 180;
        let start_y = 490;
        let mut current_y = start_y;

        for &lv in &Self::LEVEL_ORDER {
            if let Some(list) = level_dict.get(lv) {
                if !list.is_empty() {
                    let rows = ((list.len() - 1) / row_count + 1) as i32;
                    current_y += rows * gap + 30;
                }
            }
        }

        let dynamic_h = (current_y + 180).max(1000) as u32;

        let template_name = if is_wu { format!("{version}-1") } else { version.to_string() };
        let template_path = CONFIG.static_dir.join("mai/plate_table").join(format!("{template_name}.png"));
        let is_prebaked = template_path.exists();

        let mut base_img = if is_prebaked {
            load_image(&template_path).unwrap()
        } else {
            CanvasHelper::generate_table_bg(1400, dynamic_h, 400)
        };

        // 5. Plate Progress Header BG
        let prog_bg_name = if is_wu { "plate_progress_wu.png" } else { "plate_progress.png" };
        if let Some(prog_bg) = load_image(&get_pic_path(prog_bg_name)) {
            CanvasHelper::overlay(&mut base_img, &prog_bg, 175, 20);
        }

        // 6. Plate Version Banner
        let norm_ver = match version {
            "晓" | "暁" => "暁",
            "樱" | "櫻" => "櫻",
            "堇" | "菫" => "菫",
            "辉" | "輝" => "輝",
            "华" | "華" => "華",
            "熊&华" => "熊",
            "爽&煌" => "爽",
            "宙&星" => "宙",
            "祭&祝" => "祭",
            "双&宴" => "双",
            v => v,
        };
        let norm_plan = match plan {
            "极" | "極" => "極",
            "将" => "将",
            "神" => "神",
            "舞舞" => "舞舞",
            "者" | "霸者" => "者",
            p => p,
        };
        let banner_name = if version == "霸" || version == "霸者" || plan == "霸者" || (version == "舞" && norm_plan == "者") {
            "霸者.png".to_string()
        } else {
            format!("{norm_ver}{norm_plan}.png")
        };
        let plate_banner_path = CONFIG.static_dir.join("mai/plate_version").join(&banner_name);
        let banner_img_opt = load_image(&plate_banner_path)
            .or_else(|| load_image(&CONFIG.static_dir.join("mai/plate_version").join(format!("{version}{plan}.png"))));

        if let Some(banner_img) = banner_img_opt {
            let resized = CanvasHelper::resize(&banner_img, 1000, 161);
            CanvasHelper::overlay(&mut base_img, &resized, 200, 45);
        }

        // 7. Progress bars & overall counts
        let progress_ratio = if total_songs_count > 0 {
            completed_songs_count as f64 / total_songs_count as f64
        } else {
            0.0
        };

        if progress_ratio > 0.0 {
            if let Some(prog_big) = load_image(&get_pic_path("progress_big.png")) {
                let bar_w = (993.0 * progress_ratio) as u32;
                if bar_w > 0 {
                    let cropped_bar = image::imageops::crop_imm(&prog_big, 0, 0, bar_w.min(prog_big.width()), prog_big.height()).to_image();
                    CanvasHelper::overlay(&mut base_img, &cropped_bar, 204, 219);
                }
            }
        }

        let summary_text = if completed_songs_count == total_songs_count && total_songs_count > 0 {
            "COMPLETED!!!".to_string()
        } else {
            format!("{completed_songs_count}/{total_songs_count}")
        };
        CanvasHelper::draw_text_with_stroke(&mut base_img, default_text_color, 700, 240, 30.0, fot, &summary_text, "mm", 3, white_color);
        let pct_str = format!("{:.2}%", progress_ratio * 100.0);
        CanvasHelper::draw_text_with_stroke(&mut base_img, default_text_color, 1190, 240, 30.0, fot, &pct_str, "rm", 3, white_color);

        // 8. 4 / 5 Difficulty slot sub-progress bars & counters
        let (stats_start_x, stats_gap_x, progress_width, progress_x_offset, progress_text_x) = if is_wu {
            (292, 204, 176, 88, 89)
        } else {
            (320, 253, 230, 115, 115)
        };

        let prog_small_img = if is_wu {
            load_image(&get_pic_path("progress_small_wu.png")).or_else(|| load_image(&get_pic_path("progress_small.png")))
        } else {
            load_image(&get_pic_path("progress_small.png"))
        };

        for l in 0..slot_num {
            let x = stats_start_x + l as i32 * stats_gap_x;
            let sum_group = slot_qualified_counts[l];
            let plate_count = total_songs_count;
            let progress_group = if plate_count > 0 { sum_group as f64 / plate_count as f64 } else { 0.0 };

            if progress_group > 0.0 {
                if let Some(ref ps_img) = prog_small_img {
                    let w = ((progress_width as f64) * progress_group) as u32;
                    if w > 0 {
                        let cropped = image::imageops::crop_imm(ps_img, 0, 0, w.min(ps_img.width()), ps_img.height().min(46)).to_image();
                        CanvasHelper::overlay(&mut base_img, &cropped, (x - progress_x_offset) as i64, 326);
                    }
                }
            }

            let text_color = Self::ID_TEXT_COLORS[l];
            CanvasHelper::draw_text_with_stroke(&mut base_img, text_color, x, 300, 40.0, fot, &sum_group.to_string(), "mm", 4, white_color);
            CanvasHelper::draw_text_with_stroke(&mut base_img, text_color, x + progress_text_x, 320, 14.0, fot, &format!("/{plate_count}"), "rd", 3, white_color);
            let grp_pct = format!("{:.2}%", progress_group * 100.0);
            CanvasHelper::draw_text_with_stroke(&mut base_img, default_text_color, x + progress_text_x, 343, 20.0, fot, &grp_pct, "rm", 2, white_color);
        }

        // 9. Draw Grid of Songs
        let complete_bg = load_image(&get_pic_path("complete_2.png"));
        let finished_bg_icons: Vec<Option<RgbaImage>> = (0..5)
            .map(|i| load_image(&get_pic_path(&format!("t_{i}.png"))))
            .collect();
        let table_id_bg = load_image(&get_pic_path("table_id.png"));

        current_y = start_y;
        for &lv in &Self::LEVEL_ORDER {
            if let Some(list) = level_dict.get(lv) {
                if list.is_empty() {
                    continue;
                }

                // Level label on the left (x = 72)
                if !is_prebaked {
                    CanvasHelper::draw_text_with_stroke(
                        &mut base_img,
                        font_color,
                        72,
                        current_y + 40,
                        40.0,
                        fot,
                        lv,
                        "lm",
                        4,
                        white_color,
                    );
                }

                let rows = ((list.len() - 1) / row_count + 1) as i32;

                for (idx, &song) in list.iter().enumerate() {
                    let sid: u32 = song.id.parse().unwrap_or(0);
                    let row = (idx / row_count) as i32;
                    let col = (idx % row_count) as i32;
                    let x = start_x + col * gap;
                    let y = current_y + row * gap;

                    if !is_prebaked {
                        let cover_path = get_cover_path(sid);
                        if let Some(cover) = load_image(&cover_path) {
                            let resized = CanvasHelper::resize(&cover, 80, 80);
                            CanvasHelper::overlay(&mut base_img, &resized, x as i64, y as i64);
                        }
                        if let Some(ref tid) = table_id_bg {
                            CanvasHelper::overlay(&mut base_img, tid, (x - 5) as i64, (y - 5) as i64);
                        }
                        CanvasHelper::draw_text(&mut base_img, white_color, x + 56, y + 4, 16.0, tb, &song.id, "mm");
                    }

                    // Check difficulties
                    let mut has_master_qualified = false;
                    let available_diffs = song.ds.len().min(slot_num);

                    for diff_idx in 0..available_diffs {
                        let rec = score_map.get(&(sid, diff_idx)).copied();
                        if Self::is_qualified(rec, plan) {
                            if let Some(Some(t_icon)) = finished_bg_icons.get(diff_idx) {
                                CanvasHelper::overlay(&mut base_img, t_icon, (x + 4 + 19 * diff_idx as i32) as i64, (y + 63) as i64);
                            }
                            if diff_idx == 3 {
                                has_master_qualified = true;
                            }
                        }
                    }

                    // Master badge overlay
                    if has_master_qualified {
                        if let Some(ref comp_img) = complete_bg {
                            CanvasHelper::overlay(&mut base_img, comp_img, (x + 1) as i64, (y + 1) as i64);
                        }

                        if let Some(master_rec) = score_map.get(&(sid, 3)) {
                            if plan == "将" || plan == "者" || plan == "霸者" {
                                let rank_str = crate::draw::rating_table::RatingTableRenderer::format_rank_str(&master_rec.rate);
                                let rank_icon_path = get_pic_path(&format!("prism_plus/UI_TTR_Rank_{rank_str}.png"));
                                if let Some(rank_img) = load_image(&rank_icon_path) {
                                    let resized = CanvasHelper::resize(&rank_img, 80, 36);
                                    CanvasHelper::overlay(&mut base_img, &resized, x as i64, (y + 22) as i64);
                                }
                            } else if plan == "极" || plan == "極" {
                                let fc_icon_name = match master_rec.fc {
                                    ComboStatus::FC => "FC",
                                    ComboStatus::FCP => "FCp",
                                    ComboStatus::AP => "AP",
                                    ComboStatus::APP => "APp",
                                    ComboStatus::None => "FC",
                                };
                                let fc_path = get_pic_path(&format!("UI_CHR_PlayBonus_{fc_icon_name}.png"));
                                if let Some(fc_img) = load_image(&fc_path) {
                                    let resized = CanvasHelper::resize(&fc_img, 60, 60);
                                    CanvasHelper::overlay(&mut base_img, &resized, (x + 10) as i64, (y + 12) as i64);
                                }
                            } else if plan == "神" {
                                let fc_icon_name = match master_rec.fc {
                                    ComboStatus::AP => "AP",
                                    ComboStatus::APP => "APp",
                                    _ => "AP",
                                };
                                let fc_path = get_pic_path(&format!("UI_CHR_PlayBonus_{fc_icon_name}.png"));
                                if let Some(fc_img) = load_image(&fc_path) {
                                    let resized = CanvasHelper::resize(&fc_img, 60, 60);
                                    CanvasHelper::overlay(&mut base_img, &resized, (x + 10) as i64, (y + 12) as i64);
                                }
                            } else if plan == "舞舞" {
                                let fs_icon_name = match master_rec.fs {
                                    SyncStatus::FDXP => "FSDp",
                                    _ => "FSD",
                                };
                                let fs_path = get_pic_path(&format!("UI_CHR_PlayBonus_{fs_icon_name}.png"));
                                if let Some(fs_img) = load_image(&fs_path) {
                                    let resized = CanvasHelper::resize(&fs_img, 60, 60);
                                    CanvasHelper::overlay(&mut base_img, &resized, (x + 10) as i64, (y + 12) as i64);
                                }
                            }
                        }
                    }
                }

                current_y += rows * gap + 30;
            }
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

        Ok(base_img)
    }
}
