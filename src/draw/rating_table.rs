//! Level constant table / completion table (upstream `DrawRatingTable`).

use super::base::{DEFAULT_TEXT, TABLE_FONT};
use super::canvas::{mai_dir, open, open_sized, paste, paste_file, pic, resize_with, theme_pic};
use super::text::{F, draw_stroked};
use crate::core::calc::compute_rate;
use crate::core::constants::{ACHIEVEMENT_LIST, COMBO_SP, RANK_SP, STATISTICS_KEYS, SYNC_D_SP, badge_map, rank_map};
use crate::core::models::{Played, Theme};
use crate::core::music::MaiData;
use image::{RgbaImage, imageops::FilterType};
use std::collections::HashMap;

const START_X: i32 = 140;
const START_Y: i32 = 450;
const GAP: i32 = 85;
const ROW: usize = 14;

pub fn base_path(level: &str) -> std::path::PathBuf {
    mai_dir("rating_table").join(format!("{level}.png"))
}

fn base_image(level: &str) -> Option<RgbaImage> {
    image::open(base_path(level)).ok().map(|i| i.to_rgba8())
}

fn header(im: &mut RgbaImage, y: i32, level: &str) {
    draw_stroked(im, F::Fot, 495, y, 70.0, "Level.", TABLE_FONT, "ld", 8);
    draw_stroked(im, F::Fot, 750, y, 100.0, level, TABLE_FONT, "ld", 8);
}

/// `X定数表`: the pre-rendered table with only the level title.
pub fn draw_level_text(level: &str) -> Option<RgbaImage> {
    let mut im = base_image(level)?;
    header(&mut im, 220, level);
    Some(im)
}

struct Rec {
    achievements: f64,
    fc: Option<String>,
    fs: Option<String>,
}

fn rank_icon(rate: &str, w: u32, h: u32) -> Option<std::sync::Arc<RgbaImage>> {
    open_sized(&theme_pic(Theme::PrismPlus, &format!("UI_TTR_Rank_{rate}.png")), w, h)
}

/// `X完成表`: completion overlay; `plan` is a `CB_SYNC_MAP` value or `None` for ranks.
pub fn draw_completion(data: &MaiData, level: &str, results: &[Played], plan: Option<&[&str]>) -> Option<RgbaImage> {
    let mut im = base_image(level)?;
    header(&mut im, 160, level);

    let mut stats: HashMap<&str, u32> = STATISTICS_KEYS.iter().map(|k| (*k, 0)).collect();
    let mut played: HashMap<(u32, u8), Rec> = HashMap::new();
    let rank_sp = &RANK_SP[RANK_SP.len() - 6..];
    for d in results.iter().filter(|d| d.level == level) {
        played.insert(
            (d.song_id, d.level_index),
            Rec { achievements: d.achievements, fc: d.fc.clone(), fs: d.fs.clone() },
        );
        let rate = compute_rate(d.achievements).to_lowercase();
        if d.achievements >= 80.0 {
            *stats.get_mut("clear").unwrap() += 1;
        }
        if let Some(i) = rank_sp.iter().position(|r| *r == rate) {
            rank_sp[..=i].iter().for_each(|r| *stats.get_mut(r).unwrap() += 1);
        }
        if let Some(i) = d.fc.as_deref().and_then(|fc| COMBO_SP.iter().position(|c| *c == fc)) {
            COMBO_SP[..=i].iter().for_each(|r| *stats.get_mut(r).unwrap() += 1);
        }
        match d.fs.as_deref() {
            Some("sync") => *stats.get_mut("sync").unwrap() += 1,
            Some(fs) => {
                if let Some(i) = SYNC_D_SP.iter().position(|c| *c == fs) {
                    SYNC_D_SP[..=i].iter().for_each(|r| *stats.get_mut(r).unwrap() += 1);
                }
            }
            None => {}
        }
    }

    let groups = data.level_charts(level).cloned().unwrap_or_default();
    let total: usize = groups.iter().map(|(_, v)| v.len()).sum();
    paste_file(&mut im, &pic("complete.png"), 0, 0, 251, 190);
    let tb = |im: &mut RgbaImage, x: i32, y: i32, text: &str, stroke: i32| {
        draw_stroked(im, F::Tb, x, y, 30.0, text, DEFAULT_TEXT, "mm", stroke);
    };
    tb(&mut im, 394, 238, &format!("{}/{total}", stats["clear"]), 5);
    for (n, key) in STATISTICS_KEYS[1..].iter().enumerate() {
        let (x, y) = if n < 6 { (534 + n as i32 * 102, 238) } else { (292 + (n as i32 - 6) % 9 * 102, 323) };
        tb(&mut im, x, y, &stats[key].to_string(), 2);
    }

    let status = |r: &Rec| -> Option<String> {
        let plan = plan?;
        let s = if SYNC_D_SP.contains(&plan[0]) { r.fs.clone() } else { r.fc.clone() };
        s.filter(|s| plan.contains(&s.as_str()))
    };
    let badge = |s: &str, prefix: &str, size: u32| {
        badge_map(s).and_then(|b| open_sized(&pic(&format!("{prefix}{b}.png")), size, size))
    };
    let mut values: Vec<f64> = Vec::new();
    let complete_bg = open(&pic("complete_1.png"));
    let unfinished_bg = open(&pic("unfinished_1.png"));

    if level == "15" {
        let charts = groups.iter().find(|(k, _)| k == "15.0").map_or(&[][..], |(_, v)| v.as_slice());
        for (num, chart) in charts.iter().enumerate() {
            let (x, y) = (100 + (num % 3) as i32 * 425, 500 + (num / 3) as i32 * 450);
            let Some(rec) = played.get(&(chart.song_id, chart.diff.level_index)) else {
                continue;
            };
            match plan {
                None => {
                    values.push(rec.achievements);
                    if let Some(icon) = rank_icon(compute_rate(rec.achievements), 0, 0) {
                        paste(&mut im, &icon, x + 55, y + 115);
                    }
                }
                Some(p) => {
                    if let Some(s) = status(rec) {
                        values.push(p.iter().position(|v| *v == s).unwrap_or(0) as f64);
                        if let Some(icon) = badge(&s, "UI_CHR_PlayBonus_", 200) {
                            paste(&mut im, &icon, x + 75, y + 80);
                        }
                    }
                }
            }
        }
    } else {
        let mut current_y = START_Y;
        for (_, charts) in &groups {
            for (num, chart) in charts.iter().enumerate() {
                let x = START_X + (num % ROW) as i32 * GAP;
                let y = current_y + (num / ROW) as i32 * GAP;
                let Some(rec) = played.get(&(chart.song_id, chart.diff.level_index)) else {
                    continue;
                };
                match plan {
                    None => {
                        values.push(rec.achievements);
                        let bg = if rec.achievements >= 100.0 { &complete_bg } else { &unfinished_bg };
                        if let Some(bg) = bg {
                            paste(&mut im, bg, x + 1, y + 1);
                        }
                        if let Some(icon) = rank_icon(compute_rate(rec.achievements), 78, 35) {
                            paste(&mut im, &icon, x, y + 20);
                        }
                    }
                    Some(p) => {
                        if let Some(s) = status(rec) {
                            values.push(p.iter().position(|v| *v == s).unwrap_or(0) as f64);
                            if let Some(bg) = &complete_bg {
                                paste(&mut im, bg, x + 1, y + 1);
                            }
                            if let Some(icon) = badge(&s, "UI_MSS_MBase_Icon_", 50) {
                                paste(&mut im, &icon, x + 13, y + 13);
                            }
                        }
                    }
                }
            }
            current_y += ((charts.len().max(1) - 1) / ROW + 1) as i32 * GAP + 30;
        }
    }

    if values.len() == total && total > 0 {
        let thresholds: Vec<f64> = match plan {
            Some(p) => (0..p.len()).map(|i| i as f64).collect(),
            None => ACHIEVEMENT_LIST[ACHIEVEMENT_LIST.len() - 6..].to_vec(),
        };
        let r = thresholds.iter().take_while(|t| values.iter().all(|v| v >= t)).count();
        if r > 0 {
            let name = match plan {
                Some(p) => badge_map(p[r - 1]).unwrap_or_default().to_string(),
                None => rank_map(rank_sp[r - 1]),
            };
            paste_file(&mut im, &pic(&format!("UI_MSS_Allclear_Icon_{name}.png")), 0, 0, 40, 40);
        }
    }
    let (w, h) = ((im.width() as f32 * 0.8) as u32, (im.height() as f32 * 0.8) as u32);
    Some(resize_with(&im, w, h, FilterType::Lanczos3))
}
