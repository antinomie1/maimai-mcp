//! Song list image for searches with many results (upstream `song_list`).

use super::base::DEFAULT_TEXT;
use super::canvas::{covers, frosted_card, gradient, open_sized, paste, paste_file, pic, theme_pic};
use super::design_text;
use super::text::{F, WHITE, draw, draw_stroked};
use crate::core::models::{Song, Theme};
use crate::util::{ellipsis, py_float};
use image::RgbaImage;

pub const PAGE_SIZE: usize = 14;

pub fn draw_song_list(songs: &[Song], page: usize) -> RgbaImage {
    let total_page = songs.len().div_ceil(PAGE_SIZE).max(1);
    let page = page.clamp(1, total_page);
    let songs = &songs[(page - 1) * PAGE_SIZE..(page * PAGE_SIZE).min(songs.len())];
    let lines = (songs.len() / 2 + songs.len() % 2) as i32;
    let height = 200 + lines * 145 + 200;

    let mut bg = gradient(1000, height as u32);
    let layer = |bg: &mut RgbaImage, name: &str, w: u32, h: u32, x: i32, y: i32| {
        if let Some(img) = open_sized(&pic(name), w, h) {
            paste(bg, &img, x, y);
        }
    };
    layer(&mut bg, "aurora.png", 1000, 174, 0, 0);
    layer(&mut bg, "bg_shines.png", 1000, 442, 0, 0);
    if let Some(pattern) = open_sized(&pic("pattern.png"), 1000, 256) {
        for h in 0..=(height / 256) {
            paste(&mut bg, &pattern, 0, (256 + 6) * h);
        }
    }
    layer(&mut bg, "rainbow.png", 550, 288, 225, height - 435);
    layer(&mut bg, "rainbow_bottom.png", 786, 164, 107, height - 260);
    let mut im = frosted_card(&bg, (50, 150, 950, (150 + lines * 145 + 100) as u32), 0.2);

    paste_file(&mut im, &theme_pic(Theme::PrismPlus, "chara_left.png"), 156, 187, 800, 0);
    paste_file(&mut im, &pic("moon.png"), 120, 120, 60, 20);
    paste_file(&mut im, &pic("maimai でらっくす PRiSM PLUS.png"), 210, 101, 15, 20);

    let cov = covers(&songs.iter().map(|s| s.song_id).collect::<Vec<_>>(), 80, 80);
    for (num, song) in songs.iter().enumerate() {
        let x = 70 + (num % 2) as i32 * 450;
        let y = 200 + (num / 2) as i32 * 145;
        let utage = song.song_id > 100_000;
        paste_file(&mut im, &pic("song_card.png"), 0, 0, x, y);
        if let Some(c) = cov.get(&song.song_id) {
            paste(&mut im, c, x + 10, y + 10);
        }
        paste_file(&mut im, &pic(&format!("{}.png", song.version_str)), 104, 50, x + 315, y - 30);
        paste_file(&mut im, &pic(if utage { "sl_diff_utg.png" } else { "sl_diff.png" }), 0, 0, x + 100, y + 95);
        paste_file(&mut im, &pic(&format!("{}.png", song.kind.to_uppercase())), 40, 15, x + 50, y + 75);

        draw(&mut im, F::Fot, x + 50, y + 105, 15.0, &song.song_id.to_string(), DEFAULT_TEXT, "mm");
        draw(&mut im, F::Fot, x + 100, y + 25, 20.0, &ellipsis(&song.song_name, 20), DEFAULT_TEXT, "lm");
        draw(&mut im, F::Fot, x + 100, y + 50, 12.0, &ellipsis(&song.artist, 26), DEFAULT_TEXT, "lm");
        draw(&mut im, F::Fot, x + 100, y + 80, 15.0, &format!("BPM: {}", py_float(song.bpm)), DEFAULT_TEXT, "lm");
        draw(&mut im, F::Sy, x + 230, y + 80, 12.0, &song.genre, DEFAULT_TEXT, "lm");
        let diffs = if utage { &song.difficulties[..song.difficulties.len().min(1)] } else { &song.difficulties[..] };
        for d in diffs {
            let color = if !utage && d.level_index == 4 { [138, 0, 226, 255] } else { WHITE };
            let dx = x + 125 + 50 * i32::from(d.level_index);
            draw(&mut im, F::Fot, dx, y + 105, 15.0, &py_float(d.level_value), color, "mm");
        }
    }
    draw_stroked(&mut im, F::Fot, 500, 70, 55.0, "曲目列表", DEFAULT_TEXT, "mm", 3);
    draw_stroked(&mut im, F::Fot, 500, height - 100, 35.0, &format!("Page {page}/{total_page}"), DEFAULT_TEXT, "mm", 3);
    draw_stroked(&mut im, F::Fot, 500, height - 30, 18.0, &design_text(None), DEFAULT_TEXT, "mm", 3);
    im
}
