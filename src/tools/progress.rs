use super::{Reply, ensure_data, render, require_qq, wants_image};
use crate::core::clients::divingfish::RecordsQuery;
use crate::core::constants::{
    ACHIEVEMENT_LIST, COMBO_PLUS, COMBO_SP, LEVEL_LIST, RANK_PLUS, SYNC_D_SP, SYNC_PLUS, SYNC_SP, cb_sync_map,
    plate_cn, version_map,
};
use crate::core::models::{NotPlayed, Played};
use crate::core::music::data;
use crate::core::service;
use crate::draw::canvas::pic;
use crate::draw::plate_table::{PlateQuery, draw_plate_progress, draw_plate_table};
use crate::draw::rating_table::{draw_completion, draw_level_text};
use crate::draw::score::{
    Plan, draw_category_notplayed, draw_category_played, draw_plan, draw_score_list, notplayed_rows, played_rows,
};
use crate::error::{AppError, Result};
use crate::util::py_float;
use schemars::JsonSchema;
use serde::Deserialize;

fn table_level(level: &str, what: &str) -> Result<String> {
    let level = level.trim();
    match LEVEL_LIST.iter().position(|l| *l == level) {
        Some(i) if i < 6 => Err(AppError::Msg(format!("只支持查询lv7-15的{what}。"))),
        Some(_) => Ok(level.to_string()),
        None => Err(AppError::Msg(if what == "定数表" {
            "无法识别的定数。".into()
        } else {
            "无法识别的表格。".into()
        })),
    }
}

#[derive(Debug, Deserialize, JsonSchema)]
pub struct RatingTableParams {
    /// Level 7 - 15, e.g. "13+"
    pub level: String,
}

/// X定数表.
pub async fn rating_table(p: RatingTableParams) -> Result<Reply> {
    let level = table_level(&p.level, "定数表")?;
    let lv = level.clone();
    let img = render(move || draw_level_text(&lv)).await?;
    Ok(Reply::image(img, "rating_table", format!("Level.{level} 定数表")))
}

#[derive(Debug, Deserialize, JsonSchema)]
pub struct LevelTableParams {
    /// Level 7 - 15, e.g. "13+"
    pub level: String,
    /// Optional goal: fc / fc+ / ap / ap+ / fs / fs+ / fdx / fdx+ (default: achievement ranks)
    pub plan: Option<String>,
    /// User QQ number
    pub qq: Option<String>,
    pub output_format: Option<String>,
}

/// X[plan]完成表.
pub async fn level_table(p: LevelTableParams) -> Result<Reply> {
    let user = service::auth_user(require_qq(p.qq.as_deref())?)?;
    let level = table_level(&p.level, "完成表")?;
    let plan = p.plan.as_deref().and_then(|s| cb_sync_map(&s.trim().to_lowercase()));
    let mut q = RecordsQuery { level: Some(level.clone()), ..Default::default() };
    if let Some(pl) = plan {
        let v: Vec<String> = pl.iter().map(|s| s.to_string()).collect();
        if pl.contains(&"app") {
            q.fc = v.clone();
        }
        if pl.contains(&"fsdp") {
            q.fs = v;
        }
    }
    let results = service::results(&user, &q).await?;
    let mine: Vec<&Played> = results.iter().filter(|r| r.level == level).collect();
    let text = format!("Level.{level} 完成表：已游玩 {} 个谱面", mine.len());
    let d = data();
    let lv = level.clone();
    let img = render(move || draw_completion(&d, &lv, &results, plan)).await?;
    Ok(Reply::image(img, "level_table", text))
}

#[derive(Debug, Deserialize, JsonSchema)]
pub struct LevelProgressParams {
    /// Level, e.g. "14"
    pub level: String,
    /// Goal: sss+ / sss / ss+ / ss / s+ / s, fc / fc+ / ap / ap+, fs / fs+ / fdx / fdx+
    pub plan: String,
    /// "已完成", "未完成" or "未游玩" to list one category (optional)
    pub category: Option<String>,
    /// Page for category lists
    pub page: Option<usize>,
    /// User QQ number
    pub qq: Option<String>,
    pub output_format: Option<String>,
}

#[derive(PartialEq)]
enum Category {
    Default,
    Completed,
    Unfinished,
    NotPlayed,
}

/// X<plan>[分类]进度.
pub async fn level_progress(p: LevelProgressParams) -> Result<Reply> {
    let user = service::auth_user(require_qq(p.qq.as_deref())?)?;
    let level = p.level.trim().to_string();
    let plan = p.plan.trim().to_lowercase();
    let li = LEVEL_LIST.iter().position(|l| *l == level).ok_or_else(|| AppError::Msg("无此等级".into()))?;
    let rank = RANK_PLUS.iter().position(|r| *r == plan);
    let (kind, value) = if let Some(i) = rank {
        (0, if i == 0 { ACHIEVEMENT_LIST[ACHIEVEMENT_LIST.len() - 1] } else { ACHIEVEMENT_LIST[i - 1] })
    } else if let Some(i) = COMBO_PLUS.iter().position(|r| *r == plan) {
        (1, i as f64)
    } else if let Some(i) = SYNC_PLUS.iter().position(|r| *r == plan) {
        (2, i as f64)
    } else {
        return Err(AppError::Msg("无此评价等级".into()));
    };
    if li < 11 || rank.is_some_and(|i| i < 8) {
        return Err(AppError::Msg("兄啊，有点志向好不好".into()));
    }
    let category = match p.category.as_deref().map(str::trim).filter(|c| !c.is_empty()) {
        None => Category::Default,
        Some("已完成") => Category::Completed,
        Some("未完成") => Category::Unfinished,
        Some("未开始" | "未游玩") => Category::NotPlayed,
        Some(c) => return Err(AppError::Msg(format!("无法指定查询「{c}」"))),
    };

    let results = service::results(&user, &RecordsQuery { level: Some(level.clone()), ..Default::default() }).await?;
    let d = data();
    ensure_data(&d)?;
    let ok = |r: &Played| match kind {
        0 => r.achievements >= value,
        1 => r.fc.as_deref().and_then(|f| COMBO_SP.iter().position(|c| *c == f)).is_some_and(|i| i as f64 >= value),
        _ => {
            r.fs.as_deref()
                .and_then(|f| SYNC_D_SP.iter().chain(SYNC_SP.iter()).position(|c| *c == f))
                .is_some_and(|i| (i % 4) as f64 >= value)
        }
    };
    let (mut completed, mut unfinished, mut notplayed) = (Vec::new(), Vec::new(), Vec::new());
    for (song_id, diffs) in d.by_level(&level) {
        for diff in diffs {
            match results.iter().find(|r| r.song_id == song_id && r.level_index == diff.level_index && r.level == level)
            {
                Some(r) if ok(r) => completed.push(r.clone()),
                Some(r) => unfinished.push(r.clone()),
                None => {
                    notplayed.push(NotPlayed { song_id, level_index: diff.level_index, level_value: diff.level_value })
                }
            }
        }
    }
    let key = |r: &Played| match kind {
        0 => format!("{:020.4}", r.achievements),
        1 => r.fc.clone().unwrap_or_default(),
        _ => r.fs.clone().unwrap_or_default(),
    };
    completed.sort_by_key(|r| std::cmp::Reverse(key(r)));
    unfinished.sort_by_key(|r| std::cmp::Reverse(key(r)));
    notplayed.sort_by(|a, b| b.level_value.partial_cmp(&a.level_value).unwrap_or(std::cmp::Ordering::Equal));

    let up = plan.to_uppercase();
    let text = format!(
        "「{level}」共计「{}」个谱面：已完成「{}」个，未完成「{}」个，未游玩「{}」个（目标「{up}」）",
        completed.len() + unfinished.len() + notplayed.len(),
        completed.len(),
        unfinished.len(),
        notplayed.len()
    );
    if !wants_image(p.output_format.as_deref()) {
        return Ok(Reply::Text(text));
    }
    let svc = user.service;
    let page = p.page.unwrap_or(1);
    let img = render(move || {
        Some(match category {
            Category::Default => {
                let comp_limit = if unfinished.is_empty() && notplayed.is_empty() { 60 } else { 30 };
                let c_y = played_rows(completed.len().min(comp_limit)) * 109 + 140;
                let u_y = played_rows(unfinished.len().min(30)) * 109 + 140;
                let n_y = notplayed_rows(notplayed.len().min(100)) * 65 + 140;
                let plan_data = Plan {
                    level: &level,
                    plan: &plan,
                    completed: &completed,
                    completed_y: c_y,
                    completed_len: comp_limit,
                    unfinished: &unfinished,
                    unfinished_y: u_y,
                    notplayed: &notplayed,
                };
                draw_plan(svc, &d, &plan_data, (150 + c_y + u_y + n_y) as u32)
            }
            Category::Completed | Category::Unfinished => {
                let list = if category == Category::Completed { &completed } else { &unfinished };
                let total = list.len().div_ceil(80).max(1);
                let page = page.clamp(1, total);
                let shown = list.len().saturating_sub((page - 1) * 80).min(80);
                let height = 240 + played_rows(shown) * 109 + 120;
                draw_category_played(svc, &d, category == Category::Completed, list, page, total, height as u32)
            }
            Category::NotPlayed => {
                let height = (240 + notplayed_rows(notplayed.len()) * 65 + 120).max(600);
                draw_category_notplayed(&notplayed, height as u32)
            }
        })
    })
    .await?;
    Ok(Reply::image(img, "level_progress", text))
}

#[derive(Debug, Deserialize, JsonSchema)]
pub struct ScoreListParams {
    /// Level ("13+") or internal constant with one decimal ("13.7")
    pub level: String,
    pub page: Option<usize>,
    /// User QQ number
    pub qq: Option<String>,
    pub output_format: Option<String>,
}

/// X分数列表.
pub async fn level_score_list(p: ScoreListParams) -> Result<Reply> {
    let user = service::auth_user(require_qq(p.qq.as_deref())?)?;
    let raw = p.level.trim().to_string();
    let mut q = RecordsQuery::default();
    let ds = if raw.contains('.') {
        let ok = raw.split_once('.').is_some_and(|(a, b)| {
            !a.is_empty()
                && a.chars().all(|c| c.is_ascii_digit())
                && b.len() == 1
                && b.chars().all(|c| c.is_ascii_digit())
        });
        if !ok {
            return Err(AppError::Msg("输入有误，定数仅有一位小数。".into()));
        }
        let v = crate::util::round_n(raw.parse().unwrap_or(0.0), 1);
        q.level_value = Some(v);
        Some(v)
    } else if LEVEL_LIST.contains(&raw.as_str()) {
        q.level = Some(raw.clone());
        None
    } else {
        return Err(AppError::Msg("无此等级".into()));
    };
    let mut list: Vec<Played> = service::results(&user, &q)
        .await?
        .into_iter()
        .filter(|r| ds.map_or(r.level == raw, |v| r.level_value == v))
        .collect();
    list.sort_by(|a, b| b.achievements.partial_cmp(&a.achievements).unwrap_or(std::cmp::Ordering::Equal));
    let label = ds.map_or(raw.clone(), py_float);
    let end_page = list.len().div_ceil(80).max(1);
    let page = p.page.unwrap_or(1).clamp(1, end_page);
    let mut text = format!("「{label}」共计「{}」个成绩，第「{page} / {end_page}」页\n", list.len());
    for (i, r) in list.iter().enumerate().skip((page - 1) * 80).take(80) {
        text += &format!("{}. [{}] {} {:.4}% Ra {}\n", i + 1, r.song_id, r.song_name, r.achievements, r.rating);
    }
    if !wants_image(p.output_format.as_deref()) {
        return Ok(Reply::Text(text));
    }
    let to_page = if page < end_page || list.len() % 80 == 0 { 80 } else { list.len() % 80 };
    let line = to_page.div_ceil(5) as i32;
    let plc = if page < end_page {
        line * 109 + 130 * 4
    } else {
        let multiplier = to_page.div_ceil(20) as i32;
        let actual = if to_page <= 20 { 4 } else { line };
        actual * 109 + 130 * multiplier
    };
    let (svc, d) = (user.service, data());
    let img = render(move || Some(draw_score_list(svc, &d, &label, &list, page, end_page, (280 + plc) as u32))).await?;
    Ok(Reply::image(img, "score_list", text))
}

#[derive(Debug, Deserialize, JsonSchema)]
pub struct PlateParams {
    /// Plate version character: 真 超 檄 橙 晓 桃 樱 紫 堇 白 雪 辉 舞 霸 熊 华 爽 煌 宙 星 祭 祝 双 宴 镜 彩. Omit for the plate requirement chart (牌子条件).
    pub version: Option<String>,
    /// Plate type: 将 / 极 / 神 / 舞舞 / 者 (霸者)
    pub plan: Option<String>,
    /// Page (舞/霸者 tables have 2 pages)
    pub page: Option<usize>,
    /// User QQ number
    pub qq: Option<String>,
    pub output_format: Option<String>,
}

async fn plate_common(p: PlateParams, progress: bool) -> Result<Reply> {
    let Some(raw) = p.version.as_deref().map(str::trim).filter(|v| !v.is_empty()) else {
        return Ok(Reply::File { path: pic("table_condition.jpg"), mime: "image/jpeg", text: "牌子条件".into() });
    };
    let user = service::auth_user(require_qq(p.qq.as_deref())?)?;
    let plan = p.plan.as_deref().map(str::trim).unwrap_or_default().to_string();
    if !matches!(plan.as_str(), "将" | "极" | "極" | "神" | "舞舞" | "者") {
        return Err(AppError::Msg("请指定牌子类型：将 / 极 / 神 / 舞舞 / 者".into()));
    }
    let version = plate_cn(raw).to_string();
    if format!("{version}{plan}") == "真将" {
        return Err(AppError::Msg("真系没有真将哦。".into()));
    }
    let (versions, key) = version_map(&version).ok_or_else(|| AppError::Msg(format!("无法识别的版本「{raw}」")))?;
    let q = RecordsQuery { version: versions.iter().map(|v| v.to_string()).collect(), ..Default::default() };
    let results = service::results(&user, &q).await?;
    let d = data();
    ensure_data(&d)?;
    let page = p.page.unwrap_or(1).clamp(1, 2);
    let text = format!("{version}{plan}{}", if progress { "进度" } else { "完成表" });
    let key = key.to_string();
    let img = render(move || {
        let q = PlateQuery { version: &version, version_name: &key, plan: &plan, page };
        if progress { Some(draw_plate_progress(&d, &q, &results)) } else { draw_plate_table(&d, &q, &results) }
    })
    .await?;
    Ok(Reply::image(img, if progress { "plate_progress" } else { "plate_table" }, text))
}

/// XX完成表 (or 牌子条件 without version).
pub async fn plate_table(p: PlateParams) -> Result<Reply> {
    plate_common(p, false).await
}

/// XX进度.
pub async fn plate_progress(p: PlateParams) -> Result<Reply> {
    plate_common(p, true).await
}
