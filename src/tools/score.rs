use super::b50::MESSAGE;
use super::{Reply, ensure_data, find_song, render, require_qq, wants_image};
use crate::core::constants::{LEVEL_LIST, parse_difficulty};
use crate::core::models::{Rise, Slot};
use crate::core::music::data;
use crate::core::service;
use crate::draw::{info::draw_play_data, score::draw_rise};
use crate::error::{AppError, Result};
use schemars::JsonSchema;
use serde::Deserialize;

#[derive(Debug, Deserialize, JsonSchema)]
pub struct PlayScoreParams {
    /// Song id, title or alias
    pub song: String,
    /// User QQ number
    pub qq: Option<String>,
    pub output_format: Option<String>,
}

/// info: the user's records on every difficulty of a song.
pub async fn play_score(p: PlayScoreParams) -> Result<Reply> {
    let d = data();
    ensure_data(&d)?;
    let user = service::auth_user(require_qq(p.qq.as_deref())?)?;
    let song = find_song(&d, &p.song)?;
    let slots = service::song_slots(&user, &song).await?;
    let mut text = format!("【{}】{}\n", song.song_id, song.song_name);
    for (i, s) in slots.iter().enumerate() {
        let name = crate::core::constants::DIFFS.get(i).copied().unwrap_or("?");
        text += &match s {
            Slot::Played(p) => {
                format!("{name}: {:.4}% Ra {} {}\n", p.achievements, p.rating, p.fc.clone().unwrap_or_default())
            }
            Slot::NotPlayed(_) => format!("{name}: 未游玩\n"),
        };
    }
    text += MESSAGE;
    if !wants_image(p.output_format.as_deref()) {
        return Ok(Reply::Text(text));
    }
    let (theme, svc) = (user.theme, user.service);
    let img = render(move || Some(draw_play_data(&song, &slots, theme, svc))).await?;
    Ok(Reply::image(img, "play_score", text))
}

#[derive(Debug, Deserialize, JsonSchema)]
pub struct SongStatsParams {
    /// Song id, title or alias
    pub song: String,
    /// Difficulty: 绿/黄/红/紫/白 or basic..re:master (default 紫 Master)
    pub difficulty: Option<String>,
}

/// ginfo: global play statistics of a chart.
pub async fn song_stats(p: SongStatsParams) -> Result<Reply> {
    let d = data();
    ensure_data(&d)?;
    let li = match p.difficulty.as_deref().filter(|s| !s.trim().is_empty()) {
        Some(s) => parse_difficulty(s).ok_or_else(|| AppError::Msg("无法识别的难度".into()))?,
        None => 3,
    };
    let song = find_song(&d, &p.song)?;
    let diff = song.difficulties.get(li).ok_or_else(|| AppError::Msg("该乐曲没有这个等级".into()))?;
    let s = diff.stats.as_ref().ok_or_else(|| AppError::Msg("该乐曲还没有统计信息".into()))?;
    Ok(Reply::Text(format!(
        "{} {} 「{}」\n游玩次数：{}\n拟合难度：{:.2}\n平均达成率：{:.2}%\n平均 DX 分数：{:.1}\n谱面成绩标准差：{:.2}",
        song.song_id,
        song.song_name,
        crate::core::constants::DIFFS[li],
        s.cnt.round(),
        s.fit_diff,
        s.avg,
        s.avg_dx,
        s.std_dev
    )))
}

const SCORE_LINE_HELP: &str = "此功能为查找某首歌分数线设计。
命令格式：分数线「难度+歌曲id」「分数线」
例如：分数线 紫799 100
命令将返回分数线允许的「TAP」「GREAT」容错，
以及「BREAK」50落等价的「TAP」「GREAT」数。
以下为「TAP」「GREAT」的对应表：
        GREAT / GOOD / MISS
TAP         1 / 2.5  / 5
HOLD        2 / 5    / 10
SLIDE       3 / 7.5  / 15
TOUCH       1 / 2.5  / 5
BREAK       5 / 12.5 / 25 (外加200落)";

#[derive(Debug, Deserialize, JsonSchema)]
pub struct ScoreLineParams {
    /// Song id (omit all fields to get the help text)
    pub song_id: Option<u32>,
    /// Difficulty: 绿/黄/红/紫/白
    pub difficulty: Option<String>,
    /// Target achievement line, e.g. 100 or 100.5
    pub line: Option<f64>,
}

/// 分数线: TAP GREAT tolerance for a target achievement.
pub async fn score_line(p: ScoreLineParams) -> Result<Reply> {
    let (Some(id), Some(diff), Some(line)) = (p.song_id, p.difficulty.as_deref(), p.line) else {
        return Ok(Reply::Text(SCORE_LINE_HELP.into()));
    };
    let err = || AppError::Msg("格式错误，输入“分数线 帮助”以查看帮助信息".into());
    let li = parse_difficulty(diff).ok_or_else(err)?;
    let d = data();
    let song = d.by_id(id).ok_or_else(err)?;
    let chart = song.difficulties.get(li).ok_or_else(err)?;
    let n = &chart.notes;
    let total = f64::from(n.tap * 500 + n.slide * 1500 + n.hold * 1000 + n.touch * 500 + n.brk * 2500);
    let reduce = 101.0 - line;
    if n.brk == 0 || reduce <= 0.0 || reduce >= 101.0 {
        return Err(err());
    }
    let break_50 = total * (0.01 / f64::from(n.brk)) / 4.0;
    let labels = ["Basic", "Advanced", "Expert", "Master", "Re:MASTER"];
    Ok(Reply::Text(format!(
        "{}「{}」\n分数线「{}%」\n允许的最多「TAP」「GREAT」数量为\n「{:.2}」(每个-{:.4}%),\n「BREAK」50落(一共「{}」个)\n等价于「{:.3}」个「TAP」「GREAT」(-{:.4}%)",
        song.song_name,
        labels[li],
        crate::util::py_float(line),
        total * reduce / 10000.0,
        10000.0 / total,
        n.brk,
        break_50 / 100.0,
        break_50 / total * 100.0
    )))
}

#[derive(Debug, Deserialize, JsonSchema)]
pub struct RiseParams {
    /// User QQ number
    pub qq: Option<String>,
    /// Restrict to a level, e.g. "13+" (optional)
    pub level: Option<String>,
    /// Minimum rating gain per chart (optional, default 1)
    pub score: Option<u32>,
    pub output_format: Option<String>,
}

fn rise_text(title: &str, list: &[Rise]) -> String {
    let mut out = format!("--- {title} ---\n");
    for r in list {
        out += &format!(
            "[{}] {} ds{} {:.4}% -> {:.4}% (Ra {} -> {})\n",
            r.song_id,
            r.song_name,
            crate::util::py_float(r.level_value),
            r.old_achievements,
            r.achievements,
            r.old_rating,
            r.rating
        );
    }
    out
}

/// 我要上分: random charts that raise the B50 floor.
pub async fn rise_score(p: RiseParams) -> Result<Reply> {
    let d = data();
    ensure_data(&d)?;
    let user = service::auth_user(require_qq(p.qq.as_deref())?)?;
    let level = p.level.as_deref().map(str::trim).filter(|l| !l.is_empty());
    if level.is_some_and(|l| !LEVEL_LIST.contains(&l)) {
        return Err(AppError::Msg("无此等级".into()));
    }
    let (sd, sd_low, dx, dx_low) = service::rise(&user, level, p.score).await?;
    let text = format!(
        "【上分推荐】\n{}{}「谱面推荐不使用任何算法，仅供参考」",
        rise_text("旧版本谱面推荐", &sd),
        rise_text("新版本谱面推荐", &dx)
    );
    if !wants_image(p.output_format.as_deref()) {
        return Ok(Reply::Text(text));
    }
    let svc = user.service;
    let img = render(move || Some(draw_rise(svc, &sd, sd_low, &dx, dx_low))).await?;
    Ok(Reply::image(img, "rise_score", text))
}
