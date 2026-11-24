//! Shared score-card grid (upstream `ScoreBaseImage.whiledraw`) and palettes.

use super::canvas::{covers, open_sized, paste, paste_file, pic, theme_pic};
use super::text::{Color, F, WHITE, draw};
use crate::core::calc::dx_star;
use crate::core::constants::{badge_map, rank_map};
use crate::core::models::{Played, Theme};
use crate::core::music::MaiData;
use crate::util::{ellipsis, py_float};
use image::RgbaImage;

pub const DEFAULT_TEXT: Color = [124, 129, 255, 255];
pub const DIFF_TEXT: [Color; 5] = [WHITE, WHITE, WHITE, WHITE, [138, 0, 226, 255]];
pub const ID_TEXT: [Color; 5] =
    [[129, 217, 85, 255], [245, 189, 21, 255], [255, 129, 141, 255], [159, 81, 220, 255], [138, 0, 226, 255]];
pub const TABLE_FONT: Color = [114, 188, 254, 255];

const DIFF_BG: [&str; 5] = [
    "b50_score_basic.png",
    "b50_score_advanced.png",
    "b50_score_expert.png",
    "b50_score_master.png",
    "b50_score_remaster.png",
];

/// Rank icon name for a stored rate (`sssp` -> `SSSp`; already-mapped names pass through).
pub fn rate_icon(rate: &str) -> String {
    if rate.chars().all(|c| c.is_ascii_lowercase()) { rank_map(rate) } else { rate.to_string() }
}

pub fn idx(level_index: u8) -> usize {
    (level_index as usize).min(4)
}

/// Draws score cards, 5 per row (`whiledraw`). `start_y` = 235 / 1085 for B50.
pub fn score_grid(im: &mut RgbaImage, data: &MaiData, list: &[Played], theme: Theme, start_y: i32) {
    let ids: Vec<u32> = list.iter().map(|p| p.song_id).collect();
    let cov = covers(&ids, 75, 75);
    for (num, info) in list.iter().enumerate() {
        let (row, col) = (num / 5, num % 5);
        let x = 16 + col as i32 * 276;
        let y = start_y + row as i32 * 114;
        let li = idx(info.level_index);

        paste_file(im, &pic(DIFF_BG[li]), 0, 0, x, y);
        if let Some(c) = cov.get(&info.song_id) {
            paste(im, c, x + 12, y + 12);
        }
        paste_file(im, &pic(&format!("{}.png", info.kind.to_uppercase())), 37, 14, x + 51, y + 91);
        if let Some(rate) = &info.rate {
            paste_file(im, &theme_pic(theme, &format!("UI_TTR_Rank_{}.png", rate_icon(rate))), 63, 28, x + 92, y + 78);
        }
        if let Some(fc) = info.fc.as_deref().and_then(badge_map) {
            paste_file(im, &pic(&format!("UI_MSS_MBase_Icon_{fc}.png")), 34, 34, x + 154, y + 77);
        }
        if let Some(fs) = info.fs.as_deref().and_then(badge_map) {
            paste_file(im, &pic(&format!("UI_MSS_MBase_Icon_{fs}.png")), 34, 34, x + 185, y + 77);
        }
        let dx_max = data.diff(info.song_id, info.level_index).map_or(0, |d| d.dx_score);
        let star = if dx_max > 0 { dx_star(f64::from(info.dx_score) / f64::from(dx_max) * 100.0) } else { 0 };
        if star != 0
            && let Some(icon) = open_sized(&pic(&format!("UI_GAM_Gauge_DXScoreIcon_0{star}.png")), 47, 26)
        {
            paste(im, &icon, x + 217, y + 80);
        }
        let color = DIFF_TEXT[li];
        draw(im, F::Tb, x + 26, y + 98, 13.0, &info.song_id.to_string(), ID_TEXT[li], "mm");
        draw(im, F::Sy, x + 93, y + 14, 14.0, &ellipsis(&info.song_name, 18), color, "lm");
        draw(im, F::Tb, x + 93, y + 38, 30.0, &format!("{:.4}%", info.achievements), color, "lm");
        draw(im, F::Tb, x + 219, y + 65, 15.0, &format!("{}/{dx_max}", info.dx_score), color, "mm");
        let ds = data.level_value(info.song_id, info.level_index);
        draw(im, F::Tb, x + 93, y + 65, 15.0, &format!("{} -> {}", py_float(ds), info.rating), color, "lm");
    }
}
