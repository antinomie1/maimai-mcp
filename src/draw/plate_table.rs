//! Plate completion table / plate progress (upstream `DrawPlateTable` / `DrawPlateProgress`).

use super::base::{DEFAULT_TEXT, ID_TEXT, TABLE_FONT};
use super::canvas::{
    covers, crop, frosted_card, mai_dir, open, open_sized, paste, paste_file, pic, table_bg, theme_pic,
};
use super::design_text;
use super::text::{F, WHITE, draw, draw_full, draw_stroked};
use crate::core::calc::compute_rate;
use crate::core::constants::{COMBO_SP, DIFFS, LEVEL_LIST, badge_map};
use crate::core::models::{Played, Theme};
use crate::core::music::MaiData;
use crate::util::py_round;
use image::RgbaImage;
use std::collections::{HashMap, HashSet};

const START_X: i32 = 180;
const START_Y: i32 = 490;
const GAP: i32 = 96;
const ROW: usize = 12;

pub fn table_path(name: &str) -> std::path::PathBuf {
    mai_dir("plate_table").join(format!("{name}.png"))
}

/// Whether a chart satisfies the plate requirement (`_is_qualified`).
pub fn qualified(play: Option<&Played>, plan: &str) -> bool {
    let Some(p) = play else { return false };
    match plan {
        "将" => p.achievements >= 100.0,
        "者" => p.achievements >= 80.0,
        "极" | "極" => p.fc.as_deref().is_some_and(|f| COMBO_SP.contains(&f)),
        "神" => matches!(p.fc.as_deref(), Some("ap" | "app")),
        "舞舞" => matches!(p.fs.as_deref(), Some("fsd" | "fsdp" | "fsdpx" | "fsdp+")),
        _ => false,
    }
}

struct SongProgress {
    results: Vec<Option<Played>>,
    qualified: Vec<usize>,
}

struct ChartProgress {
    song_id: u32,
    level_value: f64,
    qualified: bool,
}

struct Progress {
    total: usize,
    remaster: usize,
    levels: Vec<(String, Vec<SongProgress>)>,
    display: Vec<String>,
    by_slot: Vec<Vec<ChartProgress>>,
    slot_counts: Vec<usize>,
    completed: usize,
}

/// Plate table query (`PlateTable` constructor arguments).
pub struct PlateQuery<'a> {
    /// Plate character after `PLATE_CN` mapping (`舞`, `霸`, `暁`, ...).
    pub version: &'a str,
    /// Plate data key from `VERSION_MAP`.
    pub version_name: &'a str,
    pub plan: &'a str,
    pub page: usize,
}

impl PlateQuery<'_> {
    pub fn is_wu(&self) -> bool {
        matches!(self.version, "舞" | "霸")
    }

    /// Pre-rendered base table file name.
    pub fn plate_name(&self) -> String {
        if self.is_wu() { format!("舞-{}", self.page) } else { self.version.to_string() }
    }

    fn slots(&self) -> usize {
        if self.is_wu() { 5 } else { 4 }
    }

    fn background(&self) -> Option<std::sync::Arc<RgbaImage>> {
        let plan = if self.plan == "极" { "極" } else { self.plan };
        open_sized(&mai_dir("plate_version").join(format!("{}{plan}.png", self.version)), 1000, 161)
    }
}

type LevelSlots = HashMap<String, Vec<(u32, Vec<Option<Played>>)>>;

fn process(data: &MaiData, q: &PlateQuery, results: &[Played]) -> Progress {
    let wu = q.is_wu();
    let ids: Vec<u32> = data.plate(if wu { "舞" } else { q.version_name }).to_vec();
    let re_set: HashSet<u32> = if wu { data.plate("舞ReMASTER").iter().copied().collect() } else { HashSet::new() };
    let mut songs = data.by_ids(&ids);
    let uses_re = |s: &crate::core::models::Song| wu && re_set.contains(&s.song_id) && s.difficulties.len() > 4;
    let display_diff = |s: &crate::core::models::Song| if uses_re(s) { 4 } else { 3 };
    songs.retain(|s| s.difficulties.len() > 3);
    songs.sort_by(|a, b| {
        let (x, y) = (a.difficulties[display_diff(a)].level_value, b.difficulties[display_diff(b)].level_value);
        y.partial_cmp(&x).unwrap_or(std::cmp::Ordering::Equal)
    });

    // level -> song slots, in LEVEL_LIST descending order for 舞, sort order otherwise.
    let mut order: Vec<String> = Vec::new();
    let mut slots: LevelSlots = HashMap::new();
    if wu {
        order = LEVEL_LIST.iter().rev().map(|s| s.to_string()).collect();
    }
    let mut level_of: HashMap<u32, String> = HashMap::new();
    for s in &songs {
        let lv = s.difficulties[display_diff(s)].level.clone();
        if !order.contains(&lv) {
            if wu {
                continue;
            }
            order.push(lv.clone());
        }
        let n = if wu && re_set.contains(&s.song_id) { 5 } else { 4 };
        slots.entry(lv.clone()).or_default().push((s.song_id, vec![None; n]));
        level_of.insert(s.song_id, lv);
    }
    for r in results {
        let Some(lv) = level_of.get(&r.song_id) else {
            continue;
        };
        if !wu && r.level_index == 4 {
            continue;
        }
        if let Some((_, res)) = slots.get_mut(lv).and_then(|v| v.iter_mut().find(|(id, _)| *id == r.song_id))
            && let Some(slot) = res.get_mut(r.level_index as usize)
        {
            *slot = Some(r.clone());
        }
    }
    order.retain(|lv| slots.contains_key(lv));
    let display = if wu {
        let idx = order.iter().position(|l| l == "13").unwrap_or(order.len());
        if q.page == 1 { order[..idx].to_vec() } else { order[idx..].to_vec() }
    } else {
        order.clone()
    };

    let by_id: HashMap<u32, &crate::core::models::Song> = songs.iter().map(|s| (s.song_id, *s)).collect();
    let mut by_slot: Vec<Vec<ChartProgress>> = (0..q.slots()).map(|_| Vec::new()).collect();
    let mut levels = Vec::new();
    let mut completed = 0;
    for lv in &order {
        let mut list = Vec::new();
        for (song_id, res) in slots.remove(lv).unwrap_or_default() {
            let ok: Vec<usize> = (0..res.len()).filter(|&i| qualified(res[i].as_ref(), q.plan)).collect();
            for slot in 0..res.len() {
                let value = by_id[&song_id].difficulties.get(slot).map_or(0.0, |d| d.level_value);
                if let Some(v) = by_slot.get_mut(slot) {
                    v.push(ChartProgress { song_id, level_value: value, qualified: ok.contains(&slot) });
                }
            }
            if ok.len() == res.len() {
                completed += 1;
            }
            list.push(SongProgress { results: res, qualified: ok });
        }
        levels.push((lv.clone(), list));
    }
    for charts in &mut by_slot {
        charts.sort_by(|a, b| b.level_value.partial_cmp(&a.level_value).unwrap_or(std::cmp::Ordering::Equal));
    }
    let slot_counts = by_slot.iter().map(|c| c.iter().filter(|x| x.qualified).count()).collect();
    Progress { total: ids.len(), remaster: re_set.len(), levels, display, by_slot, slot_counts, completed }
}

fn plate_icon(play: &Played, plan: &str) -> Option<std::sync::Arc<RgbaImage>> {
    if matches!(plan, "将" | "者") {
        let rate = compute_rate(play.achievements);
        return open_sized(&theme_pic(Theme::PrismPlus, &format!("UI_TTR_Rank_{rate}.png")), 80, 36);
    }
    let v = if plan == "舞舞" { play.fs.as_deref() } else { play.fc.as_deref() }?;
    let name = badge_map(v).unwrap_or(v);
    open_sized(&pic(&format!("UI_CHR_PlayBonus_{name}.png")), 60, 60)
}

fn pct(done: usize, total: usize) -> f64 {
    if total == 0 { 0.0 } else { done as f64 / total as f64 }
}

fn overall(im: &mut RgbaImage, p: &Progress) {
    let text = if p.completed == p.total { "COMPLETED!!!".to_string() } else { format!("{}/{}", p.completed, p.total) };
    let progress = pct(p.completed, p.total);
    if progress != 0.0
        && let Some(big) = open(&pic("progress_big.png"))
    {
        paste(im, &crop(&big, 0, 0, (993.0 * progress) as u32, 92), 204, 219);
    }
    draw_stroked(im, F::Fot, 700, 240, 30.0, &text, DEFAULT_TEXT, "mm", 3);
    draw_stroked(im, F::Fot, 1190, 240, 30.0, &format!("{}%", py_round(progress * 100.0, 2)), DEFAULT_TEXT, "rm", 3);
}

/// `XX完成表`: overlay on the pre-rendered plate table.
pub fn draw_plate_table(data: &MaiData, q: &PlateQuery, results: &[Played]) -> Option<RgbaImage> {
    let p = process(data, q, results);
    let mut im = image::open(table_path(&q.plate_name())).ok()?.to_rgba8();
    let wu = q.is_wu();
    let (bg, small, width, start_x, gap_x) = if wu {
        ("plate_progress_wu.png", "progress_small_wu.png", 176.0, 292, 204)
    } else {
        ("plate_progress.png", "progress_small.png", 230.0, 320, 253)
    };
    paste_file(&mut im, &pic(bg), 0, 0, 175, 20);
    if let Some(b) = q.background() {
        paste(&mut im, &b, 200, 45);
    }
    let complete_bg = open(&pic("complete_2.png"));
    let finished: Vec<_> = (0..5).map(|i| open(&pic(&format!("t_{i}.png")))).collect();
    let finished_small: Vec<_> = (0..5).map(|i| open_sized(&pic(&format!("t_{i}.png")), 14, 14)).collect();

    let mut current_y = START_Y;
    for (level, songs) in &p.levels {
        if !p.display.contains(level) {
            continue;
        }
        for (i, song) in songs.iter().enumerate() {
            let x = START_X + (i % ROW) as i32 * GAP;
            let y = current_y + (i / ROW) as i32 * GAP;
            let last = song.results.len() - 1;
            if song.qualified.contains(&last) {
                if let Some(bg) = &complete_bg {
                    paste(&mut im, bg, x + 1, y + 1);
                }
                if let Some(icon) = song.results[last].as_ref().and_then(|play| plate_icon(play, q.plan)) {
                    let (dx, dy) = if matches!(q.plan, "将" | "者") { (x, y + 22) } else { (x + 10, y + 12) };
                    paste(&mut im, &icon, dx, dy);
                }
            }
            for &s in &song.qualified {
                if wu && song.results.len() == 5 {
                    if let Some(Some(t)) = finished_small.get(s) {
                        paste(&mut im, t, x + 1 + 16 * s as i32, y + 64);
                    }
                } else if let Some(Some(t)) = finished.get(s) {
                    paste(&mut im, t, x + 4 + 19 * s as i32, y + 63);
                }
            }
        }
        current_y += ((songs.len().max(1) - 1) / ROW + 1) as i32 * GAP + 30;
    }
    overall(&mut im, &p);

    let small_img = open(&pic(small));
    for (l, &done) in p.slot_counts.iter().enumerate() {
        let x = start_x + l as i32 * gap_x;
        let (text_x, bar_x) = if wu { (89, 88) } else { (115, 115) };
        let count = if l != 4 { p.total } else { p.remaster };
        let progress = pct(done, count);
        if let (true, Some(bar)) = (progress != 0.0, &small_img) {
            paste(&mut im, &crop(bar, 0, 0, (width * progress) as u32, 46), x - bar_x, 326);
        }
        draw_stroked(&mut im, F::Fot, x, 300, 40.0, &done.to_string(), ID_TEXT[l], "mm", 4);
        draw_stroked(&mut im, F::Fot, x + text_x, 320, 14.0, &format!("/{count}"), ID_TEXT[l], "rd", 3);
        draw_stroked(
            &mut im,
            F::Fot,
            x + text_x,
            343,
            20.0,
            &format!("{}%", py_round(progress * 100.0, 2)),
            DEFAULT_TEXT,
            "rm",
            2,
        );
    }
    Some(im)
}

fn display_rows(count: usize) -> i32 {
    if count == 0 { 1 } else { ((count - 1) / 13 + 1).min(4) as i32 }
}

/// `XX进度`: unfinished charts per difficulty.
pub fn draw_plate_progress(data: &MaiData, q: &PlateQuery, results: &[Played]) -> RgbaImage {
    let p = process(data, q, results);
    let slots = p.by_slot.len();
    let pending: Vec<Vec<&ChartProgress>> =
        p.by_slot.iter().rev().map(|c| c.iter().filter(|x| !x.qualified).collect()).collect();
    let totals: Vec<usize> = p.by_slot.iter().rev().map(Vec::len).collect();
    let mut current_y = 395;
    for c in &pending {
        current_y += display_rows(c.len()) * GAP + 100;
    }
    let height = current_y + 180;
    let bg = table_bg(height as u32, Some(305));
    let mut im = frosted_card(&bg, (50, 349, 1350, current_y as u32), 0.4);
    paste_file(&mut im, &pic("plate_progress_2.png"), 0, 0, 175, 20);
    if let Some(b) = q.background() {
        paste(&mut im, &b, 200, 35);
    }

    let counts: Vec<usize> = p.slot_counts.iter().rev().copied().collect();
    let colors: Vec<_> = ID_TEXT[..slots].iter().rev().copied().collect();
    let diffs: Vec<_> = DIFFS[..slots].iter().rev().copied().collect();
    let big = open(&pic("progress_big.png"));
    let bottom = open(&pic("progress_bg.png"));
    let id_bg = open(&pic("border_table_base.png"));
    let mut start_y = 455;
    for (n, charts) in pending.iter().enumerate() {
        if let Some(b) = &bottom {
            paste(&mut im, b, 198, start_y - 85);
        }
        let progress = pct(counts[n], totals[n]);
        if let (true, Some(bar)) = (progress != 0.0, &big) {
            paste(&mut im, &crop(bar, 0, 0, (993.0 * progress) as u32, 92), 204, start_y - 79);
        }
        let c_text =
            if counts[n] == totals[n] { "COMPLETED!!!".to_string() } else { format!("{}/{}", counts[n], totals[n]) };
        draw_stroked(&mut im, F::Fot, 220, start_y - 57, 34.0, diffs[n], colors[n], "lm", 4);
        draw_stroked(&mut im, F::Fot, 700, start_y - 57, 36.0, &c_text, colors[n], "mm", 4);
        draw_stroked(
            &mut im,
            F::Fot,
            1190,
            start_y - 57,
            20.0,
            &format!("{}%", py_round(progress * 100.0, 2)),
            colors[n],
            "rm",
            2,
        );

        let cov = covers(&charts.iter().take(52).map(|c| c.song_id).collect::<Vec<_>>(), 80, 80);
        let mut max_row = 0;
        for (num, chart) in charts.iter().enumerate() {
            let row = num / 13;
            max_row = max_row.max(row);
            let x = 84 + (num % 13) as i32 * GAP;
            let y = start_y + row as i32 * GAP;
            if num >= 51 && charts.len() - num != 1 {
                let msg = format!("余「{}」\n个未完成", charts.len() - num);
                draw_full(&mut im, F::Fot, x, y + 35, 20.0, &msg, DEFAULT_TEXT, "lm", 0, WHITE);
                break;
            }
            if let Some(c) = cov.get(&chart.song_id) {
                paste(&mut im, c, x, y);
            }
            if let Some(b) = &id_bg {
                paste(&mut im, b, x - 5, y - 5);
            }
            draw(&mut im, F::Tb, x + 56, y + 4, 16.0, &chart.song_id.to_string(), WHITE, "mm");
        }
        start_y += (max_row as i32 + 1) * GAP + 100;
    }
    overall(&mut im, &p);
    draw(&mut im, F::Fot, 700, height - 15, 30.0, &design_text(None), TABLE_FONT, "mm");
    im
}
