//! Per-song play record card (upstream `song_play_data`).

use super::base::rate_icon;
use super::canvas::{cover_path, open, paste_file, pic, theme_pic};
use super::design_text;
use super::text::{F, WHITE, draw, draw_stroked};
use crate::core::calc::dx_star;
use crate::core::constants::{badge_map, category};
use crate::core::models::{Service, Slot, Song, Theme};
use crate::util::{ellipsis, py_float};
use image::RgbaImage;

pub fn draw_play_data(song: &Song, slots: &[Slot], theme: Theme, service: Service) -> RgbaImage {
    let color = theme.color();
    let mut im = open(&theme_pic(theme, "play_info.png")).map_or_else(|| RgbaImage::new(1200, 850), |i| (*i).clone());
    paste_file(&mut im, &theme_pic(theme, "logo.png"), 249, 120, 42, 34);
    paste_file(&mut im, &cover_path(song.song_id), 300, 300, 100, 260);
    paste_file(&mut im, &pic(&format!("info_{}.png", category(&song.genre))), 0, 0, 100, 260);
    paste_file(&mut im, &pic(&format!("{}.png", song.version_str)), 183, 90, 295, 205);
    paste_file(&mut im, &pic(&format!("{}.png", song.kind)), 55, 20, 350, 560);

    draw(&mut im, F::Sy, 255, 595, 12.0, &ellipsis(&song.artist, 58), color, "mm");
    draw(&mut im, F::Sy, 255, 622, 18.0, &ellipsis(&song.song_name, 38), color, "mm");
    draw(&mut im, F::Fot, 160, 720, 22.0, &song.song_id.to_string(), color, "mm");
    draw(&mut im, F::Fot, 380, 720, 22.0, &py_float(song.bpm), color, "mm");

    let step = 100;
    for (num, slot) in slots.iter().enumerate() {
        let y = step * num as i32;
        paste_file(&mut im, &pic(&format!("d_{num}.png")), 0, 0, 650, 235 + y);
        match slot {
            Slot::Played(info) => {
                paste_file(&mut im, &theme_pic(theme, "ra_dx.png"), 102, 44, 850, 272 + y);
                let max = song.difficulties.get(num).map_or(0, |d| d.dx_score);
                let star = if max > 0 { dx_star(f64::from(info.dx_score) / f64::from(max) * 100.0) } else { 0 };
                if star != 0 {
                    paste_file(&mut im, &pic(&format!("UI_GAM_Gauge_DXScoreIcon_0{star}.png")), 32, 19, 851, 296 + y);
                }
                draw(&mut im, F::Tb, 916, 304 + y, 13.0, &format!("{}/{max}", info.dx_score), color, "mm");
                paste_file(&mut im, &pic("fcfs.png"), 0, 0, 965, 265 + y);
                if let Some(fc) = info.fc.as_deref().and_then(badge_map) {
                    paste_file(&mut im, &pic(&format!("UI_CHR_PlayBonus_{fc}.png")), 65, 65, 960, 261 + y);
                }
                if let Some(fs) = info.fs.as_deref().and_then(badge_map) {
                    paste_file(&mut im, &pic(&format!("UI_CHR_PlayBonus_{fs}.png")), 65, 65, 1025, 261 + y);
                }
                let rate = info.rate.as_deref().map_or_else(|| "D".to_string(), rate_icon);
                paste_file(&mut im, &theme_pic(theme, &format!("UI_TTR_Rank_{rate}.png")), 100, 45, 737, 272 + y);
                draw(&mut im, F::Fot, 500, 295 + y, 30.0, &format!("{:.4}%", info.achievements), color, "lm");
                draw(&mut im, F::Fot, 685, 248 + y, 20.0, &py_float(info.level_value), WHITE, "mm");
                draw(&mut im, F::Tb, 915, 283 + y, 18.0, &info.rating.to_string(), color, "mm");
            }
            Slot::NotPlayed(info) => {
                draw(&mut im, F::Fot, 685, 248 + y, 25.0, &py_float(info.level_value), WHITE, "mm");
                draw(&mut im, F::Sy, 800, 302 + y, 30.0, "未游玩", color, "mm");
            }
        }
    }
    if slots.len() == 4 {
        draw(&mut im, F::Sy, 800, 302 + step * 4, 30.0, "没有该难度", color, "mm");
    }
    draw_stroked(&mut im, F::Fot, 600, 830, 20.0, &design_text(Some(service)), color, "mm", 3);
    im
}
