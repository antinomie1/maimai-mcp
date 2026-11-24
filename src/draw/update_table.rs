//! Pre-rendered base tables (upstream `UpdateTable`), generated in parallel.

use super::base::{DIFF_TEXT, TABLE_FONT};
use super::canvas::{cover_path, covers, frosted_card, open, open_sized, paste, paste_file, pic, table_bg};
use super::design_text;
use super::plate_table::table_path;
use super::rating_table::base_path;
use super::save_png;
use super::text::{F, WHITE, draw, draw_stroked};
use crate::core::constants::{LEVEL_LIST, dx_version, plate_cn, version_map};
use crate::core::models::Song;
use crate::core::music::MaiData;
use image::RgbaImage;
use rayon::prelude::*;
use std::time::Instant;

/// Concurrent table renders; bounds peak memory (~80 MB per canvas).
const TABLE_THREADS: usize = 4;

const DIFF_BORDER: [&str; 5] =
    ["border_basic.png", "border_advanced.png", "border_expert.png", "border_master.png", "border_remaster.png"];

fn footer(im: &mut RgbaImage, height: i32) {
    draw(im, F::Fot, 700, height - 75, 30.0, &design_text(None), TABLE_FONT, "mm");
}

fn rating_table(data: &MaiData, level: &str) -> Option<RgbaImage> {
    let groups = data.level_charts(level)?;
    let mut current_y = 450;
    for (_, charts) in groups {
        current_y += ((charts.len() - 1) / 14 + 1) as i32 * 85 + 30;
    }
    let height = current_y + 230;
    let mut im = frosted_card(&table_bg(height as u32, Some(360)), (50, 404, 1350, current_y as u32), 0.4);
    footer(&mut im, height);
    let ids: Vec<u32> = groups.iter().flat_map(|(_, c)| c.iter().map(|x| x.song_id)).collect();
    let cov = covers(&ids, 75, 75);
    let mut y0 = 450;
    for (ds, charts) in groups {
        let frac = ds.rsplit('.').next().unwrap_or("0");
        draw_stroked(&mut im, F::Fot, 70, y0 + 35, 40.0, &format!(".{frac}"), TABLE_FONT, "lm", 4);
        let mut max_row = 0;
        for (num, c) in charts.iter().enumerate() {
            let (row, col) = (num / 14, num % 14);
            max_row = max_row.max(row);
            let (x, y) = (140 + col as i32 * 85, y0 + row as i32 * 85);
            if let Some(img) = cov.get(&c.song_id) {
                paste(&mut im, img, x, y);
            }
            let li = (c.diff.level_index as usize).min(4);
            paste_file(&mut im, &pic(DIFF_BORDER[li]), 0, 0, x - 5, y - 5);
            draw(&mut im, F::Tb, x + 56, y + 4, 13.0, &c.song_id.to_string(), DIFF_TEXT[li], "mm");
        }
        y0 += (max_row as i32 + 1) * 85 + 30;
    }
    Some(im)
}

fn rating_table_15(data: &MaiData) -> Option<RgbaImage> {
    let charts = data.level_charts("15")?.iter().find(|(k, _)| k == "15.0").map(|(_, v)| v.clone())?;
    let lines = charts.len().div_ceil(3) as i32;
    let height = 650 + lines * 450;
    let mut im = table_bg(height as u32, Some(360));
    footer(&mut im, height);
    let white = open(&pic("chart_white.png"));
    let unknown = open_sized(&cover_path(0), 330, 330);
    for i in 0..(lines * 3) as usize {
        let (x, y) = (100 + (i % 3) as i32 * 425, 500 + (i / 3) as i32 * 450);
        if let Some(w) = &white {
            paste(&mut im, w, x, y);
        }
        if let Some(c) = charts.get(i) {
            paste_file(&mut im, &cover_path(c.song_id), 330, 330, x + 10, y + 10);
            paste_file(&mut im, &pic(&format!("{}.png", c.kind)), 0, 0, x + 200, y + 345);
            paste_file(&mut im, &pic(&format!("{}.png", c.version_str)), 332, 160, x + 9, y - 80);
            draw(&mut im, F::Fot, x + 100, y + 370, 35.0, &c.song_id.to_string(), TABLE_FONT, "mm");
        } else {
            if let Some(u) = &unknown {
                paste(&mut im, u, x + 10, y + 10);
            }
            paste_file(&mut im, &pic("DX.png"), 0, 0, x + 200, y + 345);
            draw(&mut im, F::Fot, x + 100, y + 370, 35.0, "????", TABLE_FONT, "mm");
            draw_stroked(&mut im, F::Fot, x + 175, y + 280, 30.0, "UNKNOWN", TABLE_FONT, "mm", 8);
        }
    }
    Some(im)
}

/// Grid of plate songs grouped by level (`_draw_plate`).
fn plate_image(groups: &[(String, Vec<&Song>)], remaster: Option<&[u32]>, pages: Option<usize>) -> RgbaImage {
    let mut current_y = 490;
    for (_, songs) in groups {
        current_y += ((songs.len() - 1) / 12 + 1) as i32 * 96 + 30;
    }
    let height = current_y + 180;
    let mut im = frosted_card(&table_bg(height as u32, Some(400)), (50, 444, 1350, current_y as u32), 0.4);
    if let Some(p) = pages {
        draw(&mut im, F::Fot, 700, height - 140, 40.0, &format!("Pages {}/2", p + 1), TABLE_FONT, "mm");
    }
    footer(&mut im, height);
    let ids: Vec<u32> = groups.iter().flat_map(|(_, s)| s.iter().map(|x| x.song_id)).collect();
    let cov = covers(&ids, 80, 80);
    let (base_bg, re_bg) = (open(&pic("border_table_base.png")), open(&pic("border_table_remaster.png")));
    let mut y0 = 490;
    for (lv, songs) in groups {
        draw_stroked(&mut im, F::Fot, 72, y0 + 40, 40.0, lv, TABLE_FONT, "lm", 4);
        let mut max_row = 0;
        for (num, s) in songs.iter().enumerate() {
            let (row, col) = (num / 12, num % 12);
            max_row = max_row.max(row);
            let (x, y) = (180 + col as i32 * 96, y0 + row as i32 * 96);
            if let Some(c) = cov.get(&s.song_id) {
                paste(&mut im, c, x, y);
            }
            let is_re = remaster.is_some_and(|r| r.contains(&s.song_id));
            if let Some(bg) = if is_re { &re_bg } else { &base_bg } {
                paste(&mut im, bg, x - 5, y - 5);
            }
            let color = if is_re { [138, 0, 226, 255] } else { WHITE };
            draw(&mut im, F::Tb, x + 56, y + 4, 16.0, &s.song_id.to_string(), color, "mm");
        }
        y0 += (max_row as i32 + 1) * 96 + 30;
    }
    im
}

fn group_by_level<'a>(songs: impl Iterator<Item = (&'a Song, usize)>) -> Vec<(String, Vec<&'a Song>)> {
    let mut groups: Vec<(String, Vec<(&Song, usize)>)> =
        LEVEL_LIST.iter().rev().map(|l| (l.to_string(), Vec::new())).collect();
    for (s, d) in songs {
        if let Some(diff) = s.difficulties.get(d)
            && let Some((_, g)) = groups.iter_mut().find(|(l, _)| *l == diff.level)
        {
            g.push((s, d));
        }
    }
    groups
        .into_iter()
        .filter(|(_, g)| !g.is_empty())
        .map(|(l, mut g)| {
            g.sort_by(|a, b| {
                let (x, y) = (a.0.difficulties[a.1].level_value, b.0.difficulties[b.1].level_value);
                y.partial_cmp(&x).unwrap_or(std::cmp::Ordering::Equal)
            });
            (l, g.into_iter().map(|(s, _)| s).collect())
        })
        .collect()
}

type Job<'a> = (std::path::PathBuf, Box<dyn Fn() -> Option<RgbaImage> + Send + Sync + 'a>);

/// Plate table jobs: every version plate plus the two 舞/霸者 pages.
fn plate_jobs(data: &MaiData) -> Vec<Job<'_>> {
    let mut names: Vec<String> = dx_version().skip(1).map(|(k, _)| plate_cn(k).to_string()).collect();
    names.dedup();
    let mut jobs: Vec<Job> = names
        .into_iter()
        .map(|name| {
            let path = table_path(&name);
            let render = move || {
                let key = version_map(&name).map_or(name.as_str(), |(_, k)| k);
                let songs = data.by_ids(data.plate(key));
                let groups = group_by_level(songs.into_iter().filter(|s| s.difficulties.len() > 3).map(|s| (s, 3)));
                (!groups.is_empty()).then(|| plate_image(&groups, None, None))
            };
            (path, Box::new(render) as Box<dyn Fn() -> Option<RgbaImage> + Send + Sync>)
        })
        .collect();
    for page in 0..2 {
        let render = move || {
            let re_ids = data.plate("舞ReMASTER");
            let wu = data.by_ids(data.plate("舞"));
            let groups = group_by_level(
                wu.into_iter()
                    .filter(|s| s.difficulties.len() > 3)
                    .map(|s| (s, if re_ids.contains(&s.song_id) { 4 } else { 3 })),
            );
            let rank = |l: &str| LEVEL_LIST.iter().position(|x| *x == l).unwrap_or(0);
            let split = groups.iter().position(|(l, _)| rank(l) <= rank("13")).unwrap_or(groups.len());
            let part = if page == 0 { &groups[..split] } else { &groups[split..] };
            Some(plate_image(part, Some(re_ids), Some(page)))
        };
        jobs.push((table_path(&format!("舞-{}", page + 1)), Box::new(render)));
    }
    jobs
}

pub fn tables_exist() -> bool {
    base_path("13").exists() && base_path("15").exists() && table_path("舞-1").exists() && table_path("彩").exists()
}

/// Renders and saves every rating table (lv7–15) and plate table.
/// Runs on a small dedicated pool so only a few large canvases are alive at once.
pub fn update_all(data: &MaiData) -> String {
    let start = Instant::now();
    let mut jobs: Vec<Job> = LEVEL_LIST[6..LEVEL_LIST.len() - 1]
        .iter()
        .map(|lv| {
            (
                base_path(lv),
                Box::new(move || rating_table(data, lv)) as Box<dyn Fn() -> Option<RgbaImage> + Send + Sync>,
            )
        })
        .collect();
    jobs.push((base_path("15"), Box::new(|| rating_table_15(data))));
    jobs.extend(plate_jobs(data));
    let run = || {
        jobs.par_iter()
            .filter(|(path, render)| match render().map(|img| save_png(&img, path)) {
                Some(Ok(())) => true,
                Some(Err(e)) => {
                    log::warn!("{} 保存失败: {e}", path.display());
                    false
                }
                None => false,
            })
            .count()
    };
    let saved = match rayon::ThreadPoolBuilder::new().num_threads(TABLE_THREADS).build() {
        Ok(pool) => pool.install(run),
        Err(_) => run(),
    };
    format!("定数表与完成表更新完成（{saved} 张），耗时：{:.3}s", start.elapsed().as_secs_f64())
}
