#![allow(clippy::too_many_arguments)]
//! Score tables on the PRiSM PLUS background (upstream `DrawScore`).

use super::base::{DEFAULT_TEXT, DIFF_TEXT, ID_TEXT, idx, rate_icon, score_grid};
use super::canvas::{covers, crop, open_sized, paste, paste_file, pic, table_bg, theme_pic};
use super::design_text;
use super::text::{F, WHITE, draw, draw_full, draw_stroked};
use crate::core::models::{NotPlayed, Played, Rise, Service, Theme};
use crate::core::music::MaiData;
use crate::util::{ellipsis, py_float};
use image::RgbaImage;

const THEME: Theme = Theme::PrismPlus;
const RISE_BG: [&str; 5] = [
    "rise_score_basic.png",
    "rise_score_advanced.png",
    "rise_score_expert.png",
    "rise_score_master.png",
    "rise_score_remaster.png",
];
const PROGRESS_BG: [&str; 5] = [
    "border_progress_basic.png",
    "border_progress_advanced.png",
    "border_progress_expert.png",
    "border_progress_master.png",
    "border_progress_remaster.png",
];

pub fn rows(count: usize, size: usize) -> usize {
    count.div_ceil(size)
}

pub fn played_rows(count: usize) -> i32 {
    rows(count, 5).max(4) as i32
}

pub fn notplayed_rows(count: usize) -> i32 {
    rows(count, 20).max(4) as i32
}

fn title_lengthen(im: &mut RgbaImage, y: i32) {
    paste_file(im, &theme_pic(THEME, "title_lengthen.png"), 0, 0, 475, y);
}

fn footer(im: &mut RgbaImage, service: Service, offset: i32) {
    let h = im.height() as i32;
    draw_stroked(im, F::Sy, 700, h - offset, 25.0, &design_text(Some(service)), DEFAULT_TEXT, "mm", 2);
}

/// Grid of unplayed charts (`_while_pic`), 20 per row.
fn notplayed_grid(im: &mut RgbaImage, data: &[NotPlayed], start_y: i32) {
    let cov = covers(&data.iter().map(|v| v.song_id).collect::<Vec<_>>(), 55, 55);
    for (num, v) in data.iter().enumerate() {
        let x = 55 + (num % 20) as i32 * 65;
        let y = start_y + (num / 20) as i32 * 65;
        if let Some(c) = cov.get(&v.song_id) {
            paste(im, c, x, y);
        }
        let li = idx(v.level_index);
        paste_file(im, &pic(PROGRESS_BG[li]), 0, 0, x - 4, y - 4);
        draw(im, F::Tb, x + 36, y + 3, 12.0, &v.song_id.to_string(), DIFF_TEXT[li], "mm");
    }
}

fn rise_column(im: &mut RgbaImage, data: &[Rise], low_score: u32, base_x: i32) {
    let cov = covers(&data.iter().map(|r| r.song_id).collect::<Vec<_>>(), 80, 80);
    for (index, d) in data.iter().enumerate() {
        let (x, y) = (base_x, 120 + index as i32 * 140);
        let li = idx(d.level_index);
        paste_file(im, &pic(RISE_BG[li]), 0, 0, x + 30, y);
        if let Some(c) = cov.get(&d.song_id) {
            paste(im, c, x + 55, y + 41);
        }
        paste_file(im, &pic(&format!("{}.png", d.kind.to_uppercase())), 60, 22, x + 240, y + 114);
        let old = theme_pic(THEME, &format!("UI_TTR_Rank_{}.png", rate_icon(&d.old_rate)));
        paste_file(im, &old, 63, 28, x + 145, y + 82);
        paste_file(im, &theme_pic(THEME, &format!("UI_TTR_Rank_{}.png", rate_icon(&d.rate))), 63, 28, x + 305, y + 82);

        let (dc, ic) = (DIFF_TEXT[li], ID_TEXT[li]);
        draw(im, F::Sy, x + 142, y + 44, 17.0, &ellipsis(&d.song_name, 26), dc, "lm");
        draw(im, F::Tb, x + 145, y + 124, 18.0, &format!("ID: {}", d.song_id), ic, "lm");
        draw(im, F::Tb, x + 210, y + 71, 25.0, &format!("{:.4}%", d.old_achievements), dc, "mm");
        draw(im, F::Tb, x + 245, y + 96, 17.0, &format!("Ra: {}", d.old_rating), dc, "mm");
        draw(im, F::Tb, x + 370, y + 71, 25.0, &format!("{:.4}%", d.achievements), dc, "mm");
        draw(im, F::Tb, x + 415, y + 96, 17.0, &format!("Ra: {}", d.rating), dc, "mm");
        draw(im, F::Tb, x + 315, y + 124, 18.0, &format!("ds:{}", py_float(d.level_value)), ic, "lm");
        let gain = if d.old_rating > low_score { d.rating - d.old_rating } else { d.rating.saturating_sub(low_score) };
        draw(im, F::Tb, x + 390, y + 124, 18.0, &format!("Ra +{gain}"), ic, "lm");
    }
}

/// 上分推荐 (`draw_rise`): 1400x960 canvas cropped to x 200..1200.
pub fn draw_rise(service: Service, sd: &[Rise], sd_low: u32, dx: &[Rise], dx_low: u32) -> RgbaImage {
    let mut im = table_bg(960, None);
    if let Some(title) = open_sized(&theme_pic(THEME, "title.png"), 273, 80) {
        paste(&mut im, &title, 314, 30);
        paste(&mut im, &title, 814, 30);
    }
    draw(&mut im, F::Sy, 450, 68, 18.0, "旧版本谱面推荐", DEFAULT_TEXT, "mm");
    rise_column(&mut im, sd, sd_low, 200);
    draw(&mut im, F::Sy, 950, 68, 18.0, "新版本谱面推荐", DEFAULT_TEXT, "mm");
    rise_column(&mut im, dx, dx_low, 700);
    let h = im.height() as i32;
    draw(&mut im, F::Sy, 700, h - 84, 25.0, "「谱面推荐不使用任何算法，仅供参考」", DEFAULT_TEXT, "mm");
    draw_stroked(&mut im, F::Sy, 700, h - 36, 18.0, &design_text(Some(service)), DEFAULT_TEXT, "mm", 2);
    crop(&im, 200, 0, 1000, 960)
}

pub struct Plan<'a> {
    pub level: &'a str,
    pub plan: &'a str,
    pub completed: &'a [Played],
    pub completed_y: i32,
    pub completed_len: usize,
    pub unfinished: &'a [Played],
    pub unfinished_y: i32,
    pub notplayed: &'a [NotPlayed],
}

/// 等级进度总览 (`draw_plan`).
pub fn draw_plan(service: Service, data: &MaiData, p: &Plan, height: u32) -> RgbaImage {
    let mut im = table_bg(height, None);
    let (cy, uy) = (p.completed_y, p.unfinished_y);
    let total = p.completed.len() + p.unfinished.len() + p.notplayed.len();
    let plan = p.plan.to_uppercase();
    for y in [30, 30 + cy, 30 + cy + uy] {
        title_lengthen(&mut im, y);
    }
    let hint = |what: &str| format!("可使用「{}{plan}{what}进度」\n指令查询详细列表", p.level);
    draw(&mut im, F::Sy, 700, 77, 25.0, &format!("已完成谱面「{}」个", p.completed.len()), DEFAULT_TEXT, "mm");
    draw_full(&mut im, F::Sy, 1300, 77, 20.0, &hint("已完成"), DEFAULT_TEXT, "rm", 2, WHITE);
    draw(&mut im, F::Sy, 700, 77 + cy, 25.0, &format!("未完成谱面「{}」个", p.unfinished.len()), DEFAULT_TEXT, "mm");
    draw_full(&mut im, F::Sy, 1300, 77 + cy, 20.0, &hint("未完成"), DEFAULT_TEXT, "rm", 2, WHITE);
    draw(
        &mut im,
        F::Sy,
        700,
        77 + cy + uy,
        25.0,
        &format!("未游玩谱面「{}」个", p.notplayed.len()),
        DEFAULT_TEXT,
        "mm",
    );

    score_grid(&mut im, data, &p.completed[..p.completed.len().min(p.completed_len)], THEME, 140);
    score_grid(&mut im, data, &p.unfinished[..p.unfinished.len().min(30)], THEME, 140 + cy);
    notplayed_grid(&mut im, &p.notplayed[..p.notplayed.len().min(100)], 140 + cy + uy);

    let h = im.height() as i32;
    paste_file(&mut im, &theme_pic(THEME, "design.png"), 0, 0, 200, h - 133);
    let rest = p.unfinished.len() + p.notplayed.len();
    let msg = format!("「{}」共计「{total}」个谱面，剩余「{rest}」个谱面未完成「{plan}」", p.level);
    draw(&mut im, F::Sy, 700, h - 90, 22.0, &msg, DEFAULT_TEXT, "mm");
    footer(&mut im, service, 30);
    im
}

/// 已完成 / 未完成 progress page (`draw_category`, played branch).
pub fn draw_category_played(
    service: Service,
    data: &MaiData,
    completed: bool,
    list: &[Played],
    page: usize,
    end_page: usize,
    height: u32,
) -> RgbaImage {
    let mut im = table_bg(height, None);
    let txt = if completed { "已完成" } else { "未完成" };
    let shown = &list[((page - 1) * 80).min(list.len())..(page * 80).min(list.len())];
    title_lengthen(&mut im, 30);
    draw(&mut im, F::Sy, 700, 77, 28.0, &format!("{txt}谱面"), DEFAULT_TEXT, "mm");
    score_grid(&mut im, data, shown, THEME, 140);
    let h = im.height() as i32;
    paste_file(&mut im, &theme_pic(THEME, "design.png"), 0, 0, 200, h - 133);
    let msg = format!(
        "{txt}谱面共计「{}」个，当前第「{}-{}」个，第「{page} / {end_page}」页",
        list.len(),
        (page - 1) * 80 + 1,
        80 * (page - 1) + shown.len()
    );
    draw(&mut im, F::Sy, 700, h - 90, 25.0, &msg, DEFAULT_TEXT, "mm");
    footer(&mut im, service, 30);
    im
}

/// 未游玩 progress page (`draw_category`, not-played branch).
pub fn draw_category_notplayed(list: &[NotPlayed], height: u32) -> RgbaImage {
    let mut im = table_bg(height, None);
    title_lengthen(&mut im, 30);
    draw(&mut im, F::Sy, 700, 77, 28.0, "未游玩谱面", DEFAULT_TEXT, "mm");
    notplayed_grid(&mut im, list, 200);
    let h = im.height() as i32;
    paste_file(&mut im, &theme_pic(THEME, "design.png"), 0, 0, 200, h - 113);
    draw(&mut im, F::Sy, 700, h - 70, 25.0, &format!("未游玩谱面共计「{}」个", list.len()), DEFAULT_TEXT, "mm");
    im
}

/// 分数列表 (`draw_score_list`), 20 cards per block.
pub fn draw_score_list(
    service: Service,
    data: &MaiData,
    rating: &str,
    list: &[Played],
    page: usize,
    end_page: usize,
    height: u32,
) -> RgbaImage {
    let mut im = table_bg(height, None);
    let start = (page - 1) * 80;
    let current = &list[start.min(list.len())..(page * 80).min(list.len())];
    let block = 140 + 4 * 114;
    for (i, chunk) in current.chunks(20).enumerate() {
        let base_y = i as i32 * block;
        title_lengthen(&mut im, base_y + 20);
        let (a, b) = (start + i * 20 + 1, start + i * 20 + chunk.len());
        draw(&mut im, F::Fot, 700, base_y + 67, 28.0, &format!("No.{a}- No.{b}"), DEFAULT_TEXT, "mm");
        score_grid(&mut im, data, chunk, THEME, base_y + 140);
    }
    let h = im.height() as i32;
    paste_file(&mut im, &theme_pic(THEME, "design.png"), 0, 0, 200, h - 153);
    let msg = format!(
        "「{rating}」共计「{}」个成绩，当前第「{}-{}」个，第「{page} / {end_page}」页",
        list.len(),
        start + 1,
        start + current.len()
    );
    draw(&mut im, F::Sy, 700, h - 110, 25.0, &msg, DEFAULT_TEXT, "mm");
    footer(&mut im, service, 35);
    im
}
