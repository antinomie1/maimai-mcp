//! Chart info cards (upstream `song_chart_info` / `song_chart_banquet_info`).

use super::canvas::{cover_path, open, paste_file, pic, theme_pic};
use super::design_text;
use super::text::{F, WHITE, draw, draw_full, draw_stroked};
use crate::core::calc::compute_rating;
use crate::core::constants::ACHIEVEMENT_LIST;
use crate::core::models::{Played, Song, Theme};
use crate::util::{ellipsis, py_float, py_round};
use image::RgbaImage;

/// Ratings at 97 / 98 / 99 / 99.5 / 100 / 100.5 plus the next point, descending.
pub fn best_ratings(ds: f64) -> Vec<u32> {
    let mut ra: Vec<u32> =
        ACHIEVEMENT_LIST[ACHIEVEMENT_LIST.len() - 6..].iter().map(|a| compute_rating(ds, *a)).collect();
    ra.push(compute_rating(ds, 100.5) + 1);
    ra.sort_unstable_by(|a, b| b.cmp(a));
    ra
}

fn new_best_score(song_id: u32, level_index: usize, value: u32, best: &[Played]) -> u32 {
    if let Some(v) = best.iter().find(|v| v.song_id == song_id && v.level_index as usize == level_index) {
        return value.saturating_sub(v.rating);
    }
    value.saturating_sub(best.last().map_or(0, |b| b.rating))
}

/// Player context for rating gains: `(is_full, best list of the song's version half)`.
pub struct Calc<'a> {
    pub is_full: bool,
    pub best: &'a [Played],
}

pub fn draw_chart_info(song: &Song, calc: Option<Calc>, theme: Theme) -> RgbaImage {
    let mut im = open(&theme_pic(theme, "chart_info.png")).map_or_else(|| RgbaImage::new(1200, 1250), |i| (*i).clone());
    let color = theme.color();
    paste_file(&mut im, &theme_pic(theme, "logo.png"), 249, 120, 65, 25);
    if song.isnew {
        paste_file(&mut im, &pic("UI_CMN_TabTitle_NewSong.png"), 249, 120, 842, 100);
    }
    paste_file(&mut im, &cover_path(song.song_id), 242, 242, 133, 197);
    paste_file(&mut im, &pic(&format!("{}.png", song.version_str)), 182, 90, 800, 370);
    paste_file(&mut im, &pic(&format!("{}.png", song.kind)), 80, 30, 295, 410);

    draw(&mut im, F::Fot, 405, 220, 28.0, &ellipsis(&song.song_name, 40), color, "lm");
    draw(&mut im, F::Fot, 407, 265, 20.0, &ellipsis(&song.artist, 50), color, "lm");
    draw(&mut im, F::Fot, 460, 345, 24.0, &py_float(song.bpm), color, "lm");
    draw(&mut im, F::Fot, 405, 435, 22.0, &format!("ID {}", song.song_id), color, "lm");
    draw(&mut im, F::Sy, 665, 435, 24.0, &song.genre, color, "mm");

    for (index, v) in song.difficulties.iter().enumerate() {
        let sp = 70 * index as i32;
        draw(&mut im, F::Fot, 120, 590 + sp, 22.0, &format!("{}({})", v.level, py_float(v.level_value)), WHITE, "mm");
        let fitting = v.stats.as_ref().map_or_else(|| "-".to_string(), |s| format!("擬 - {}", py_round(s.fit_diff, 2)));
        draw(&mut im, F::Fot, 120, 613 + sp, 15.0, &fitting, WHITE, "mm");
        draw(&mut im, F::Sy, 310, 590 + sp, 20.0, &ellipsis(&v.note_designer, 19), color, "mm");
        for (n, value) in v.notes.fields().iter().enumerate() {
            draw(&mut im, F::Fot, 480 + 122 * n as i32, 590 + sp, 25.0, &value.to_string(), color, "mm");
        }
        if index <= 1 {
            continue;
        }
        for (n, value) in best_ratings(v.level_value).into_iter().enumerate() {
            let (size, text) = match &calc {
                None => (22.0, value.to_string()),
                Some(c) if !c.is_full => (17.0, format!("{value}(↑{value})")),
                Some(c) if value > c.best.last().map_or(0, |b| b.rating) => {
                    match new_best_score(song.song_id, index, value, c.best) {
                        0 => (22.0, value.to_string()),
                        up => (17.0, format!("{value}(↑{up})")),
                    }
                }
                Some(_) => (22.0, value.to_string()),
            };
            draw(&mut im, F::Fot, 295 + 125 * n as i32, 1017 + 46 * (index as i32 - 2), size, &text, color, "mm");
        }
    }
    draw(&mut im, F::Sy, 295, 985, 12.0, "*未实装", WHITE, "mm");
    draw_stroked(&mut im, F::Fot, 600, 1220, 25.0, &design_text(None), color, "mm", 3);
    im
}

pub fn draw_banquet_info(song: &Song) -> RgbaImage {
    let mut im = open(&pic("chart_info_enkaijou.png")).map_or_else(|| RgbaImage::new(1200, 1150), |i| (*i).clone());
    let stroke: [u8; 4] = [210, 57, 174, 255];
    let buddy = song.is_buddy.unwrap_or(false);
    paste_file(&mut im, &pic("utg_kanji.png"), 0, 0, 140, if buddy { 660 } else { 730 });
    let (player, p_y, base_y, step_y) = if buddy { ("utg_2p.png", 715, 820, 100) } else { ("utg_1p.png", 785, 890, 0) };
    if buddy {
        paste_file(&mut im, &pic("utg_buddy.png"), 0, 0, 255, 660);
    }
    paste_file(&mut im, &pic(player), 0, 0, 98, p_y);
    paste_file(&mut im, &theme_pic(Theme::PrismPlus, "logo.png"), 249, 120, 10, 35);
    if song.isnew {
        paste_file(&mut im, &pic("UI_CMN_TabTitle_NewSong.png"), 249, 120, 950, 165);
    }
    paste_file(&mut im, &cover_path(song.song_id), 242, 242, 133, 246);
    paste_file(&mut im, &pic(&format!("{}.png", song.version_str)), 182, 90, 800, 415);

    let s = |im: &mut RgbaImage, x: i32, y: i32, size: f32, t: &str, anchor: &str| {
        draw_full(im, F::Fot, x, y, size, t, WHITE, anchor, 3, stroke);
    };
    draw(&mut im, F::Fot, 216, p_y - 28, 18.0, song.kanji.as_deref().unwrap_or("None"), WHITE, "mm");
    s(&mut im, 405, 265, 28.0, &ellipsis(&song.song_name, 36), "lm");
    s(&mut im, 407, 320, 20.0, &ellipsis(&song.artist, 50), "lm");
    s(&mut im, 460, 393, 24.0, &py_float(song.bpm), "lm");
    s(&mut im, 405, 475, 22.0, &format!("ID {}", song.song_id), "lm");
    s(&mut im, 680, 475, 22.0, &song.genre, "mm");
    draw(&mut im, F::Fot, 595, 595, 25.0, song.description.as_deref().unwrap_or("None"), WHITE, "mm");
    let level = song.difficulties.first().map_or("", |d| d.level.as_str());
    s(&mut im, 180, p_y + 28, 24.0, &format!("Lv. {level}"), "mm");
    for (index, v) in song.difficulties.iter().enumerate() {
        for (n, value) in v.notes.fields().iter().enumerate() {
            s(&mut im, 330 + 140 * n as i32, base_y + step_y * index as i32, 25.0, &value.to_string(), "mm");
        }
    }
    draw_full(&mut im, F::Fot, 600, 1100, 25.0, &design_text(None), stroke, "mm", 3, WHITE);
    im
}
