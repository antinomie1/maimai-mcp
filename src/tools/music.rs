use super::b50::MESSAGE;
use super::{Reply, ensure_data, id_lines, render, require_qq, wants_image};
use crate::config::CONFIG;
use crate::core::clients::yuzuchan::{self, SongsByAlias};
use crate::core::constants::{FORTUNE, parse_difficulty};
use crate::core::models::{Alias, Song, User};
use crate::core::music::{Filter, data};
use crate::core::{service, store};
use crate::draw::canvas::{cover_path, open};
use crate::draw::chart::{Calc, draw_banquet_info, draw_chart_info};
use crate::draw::song_list::draw_song_list;
use crate::error::{AppError, Result};
use crate::util::{Rng, py_float, qq_hash};
use schemars::JsonSchema;
use serde::Deserialize;

/// Optional user for personal rating gains (`GetUserAndAuthOrNone`).
fn optional_user(qq: Option<&str>) -> Option<User> {
    qq.and_then(|q| service::auth_user(q).ok())
}

fn chart_text(song: &Song) -> String {
    let mut t = format!(
        "ID {} {}\n曲师：{}\nBPM：{} 分类：{} 版本：{}\n",
        song.song_id,
        song.song_name,
        song.artist,
        py_float(song.bpm),
        song.genre,
        song.version_str
    );
    for d in &song.difficulties {
        let n = &d.notes;
        t += &format!(
            "{}({}) 谱师：{} 物量 {} (TAP {} HOLD {} SLIDE {} TOUCH {} BREAK {})\n",
            d.level,
            py_float(d.level_value),
            d.note_designer,
            n.total,
            n.tap,
            n.hold,
            n.slide,
            n.touch,
            n.brk
        );
    }
    t + MESSAGE
}

/// `draw_chart_info(song, user)` as a reply, with `prefix` text before the caption.
async fn chart_reply(song: Song, user: Option<User>, prefix: &str, format: Option<&str>) -> Result<Reply> {
    let text = format!("{prefix}{}", chart_text(&song));
    if !wants_image(format) {
        return Ok(Reply::Text(text));
    }
    let (theme, ctx) = service::chart_context(user.as_ref(), &song).await;
    let img = render(move || {
        Some(if song.song_id >= 100_000 {
            draw_banquet_info(&song)
        } else {
            let calc = ctx.as_ref().map(|(is_full, best)| Calc { is_full: *is_full, best });
            draw_chart_info(&song, calc, theme)
        })
    })
    .await?;
    Ok(Reply::image(img, "chart_info", text))
}

async fn list_reply(songs: Vec<Song>, page: usize, prefix: String, format: Option<&str>) -> Result<Reply> {
    let text = format!("{prefix}{}", id_lines(songs.iter().take(50).map(|s| (&s.song_id, &s.song_name))));
    if !wants_image(format) {
        return Ok(Reply::Text(text));
    }
    let img = render(move || Some(draw_song_list(&songs, page))).await?;
    Ok(Reply::image(img, "song_list", text))
}

#[derive(Debug, Deserialize, JsonSchema)]
pub struct ChartInfoParams {
    /// Alias, song id ("834" / "id834") or title (XXX是什么歌 / id xxx)
    pub song: String,
    /// User QQ number, for personal rating gains on the card (optional)
    pub qq: Option<String>,
    /// Result page when many titles match
    pub page: Option<usize>,
    pub output_format: Option<String>,
}

/// XXX是什么歌 / id xxx.
pub async fn chart_info(p: ChartInfoParams) -> Result<Reply> {
    let d = data();
    ensure_data(&d)?;
    let name = p.song.trim().to_string();
    let user = optional_user(p.qq.as_deref());
    let fmt = p.output_format.as_deref();
    let error_msg = format!(
        "未找到别名为「{name}」的歌曲\n※ 可以使用「添加别名」指令给该乐曲添加别名\n※ 如果是歌名的一部分，请使用「查歌」指令查询哦。"
    );
    let mut aliases: Vec<Alias> = d.alias_by_alias(&name).into_iter().cloned().collect();
    if aliases.is_empty() {
        aliases = d.alias_by_alias(&name.to_lowercase()).into_iter().cloned().collect();
    }
    if aliases.is_empty() {
        match yuzuchan::songs_by_alias(&name).await {
            Ok(SongsByAlias::Votes(votes)) if !votes.is_empty() => {
                let mut msg = format!("未找到别名为「{name}」的歌曲，但找到与此相同别名的投票：\n");
                for (tag, id) in votes {
                    msg += &format!("- {tag}\n    ID {id}: {name}\n");
                }
                msg += "※ 可以使用指令「同意别名 XXXXX」进行投票";
                return Ok(Reply::Text(msg));
            }
            Ok(SongsByAlias::Songs(list)) => {
                aliases =
                    list.into_iter().map(|a| Alias { song_id: a.song_id, song_name: a.name, alias: a.alias }).collect();
            }
            _ => {}
        }
    }
    match aliases.len() {
        0 => {}
        1 => {
            return match d.by_id(aliases[0].song_id) {
                Some(s) => chart_reply(s.clone(), user, "您要找的是不是：", fmt).await,
                None => Ok(Reply::Text(error_msg)),
            };
        }
        n => {
            let mut msg = format!("找到{n}个相同别名的曲目：\n");
            for a in &aliases {
                msg += &format!("{}：{}\n", a.song_id, a.song_name);
            }
            return Ok(Reply::Text(msg + "※ 请使用「id xxxxx」查询指定曲目"));
        }
    }
    let id_text = name.strip_prefix("id").or_else(|| name.strip_prefix("ID")).map(str::trim).unwrap_or(&name);
    if let Ok(id) = id_text.parse::<u32>() {
        return match d.by_id(id) {
            Some(s) => chart_reply(s.clone(), user, "您要找的是不是：", fmt).await,
            None => Ok(Reply::Text(format!("未找到ID为「{id}」的乐曲"))),
        };
    }
    let mut found = d.filter(&Filter { title: Some(&name), ..Default::default() });
    let head = format!("未找到别名为「{name}」的歌曲，但找到「{}」个相似标题的曲目：\n", found.len());
    match found.len() {
        0 => Ok(Reply::Text(error_msg)),
        1 => chart_reply(found.remove(0), user, "您要找的是不是：", fmt).await,
        2..=5 => {
            found.sort_by_key(|s| s.song_id);
            Ok(Reply::Text(format!(
                "{head}{}※ 请使用「id xxxxx」查询指定曲目",
                id_lines(found.iter().map(|s| (&s.song_id, &s.song_name)))
            )))
        }
        _ => list_reply(found, p.page.unwrap_or(1), head, fmt).await,
    }
}

#[derive(Debug, Deserialize, JsonSchema)]
pub struct SearchParams {
    /// Search arguments: title / artist / charter name, "13.7" or "13.0 13.5" for 定数, "180" or "150 200" for bpm
    pub keyword: String,
    /// "title" (default), "ds" (定数查歌), "bpm", "artist" (曲师), "charter" (谱师)
    pub mode: Option<String>,
    /// Page of the result list
    pub page: Option<usize>,
    /// User QQ number, for personal rating gains when exactly one song matches
    pub qq: Option<String>,
    pub output_format: Option<String>,
}

fn range_args(args: &[f64], page: &mut usize) -> Option<(f64, f64)> {
    match *args {
        [a] => Some((a, a)),
        [a, b] if a > b => {
            *page = (b as usize).max(1);
            Some((a, a))
        }
        [a, b] => Some((a, b)),
        [a, b, c] => {
            *page = (c as usize).max(1);
            Some((a, b))
        }
        _ => None,
    }
}

/// 查歌 / 定数查歌 / bpm查歌 / 曲师查歌 / 谱师查歌.
pub async fn search_song(p: SearchParams) -> Result<Reply> {
    let d = data();
    ensure_data(&d)?;
    let words: Vec<&str> = p.keyword.split_whitespace().collect();
    let mut page = p.page.unwrap_or(1).max(1);
    let not_found = "没有找到这样的乐曲。\n※ 如果是别名请使用「XXX是什么歌」指令进行查询哦。";
    if words.is_empty() {
        return Ok(Reply::Text(not_found.into()));
    }
    // Trailing number = page for name searches (supports names with spaces).
    let name_page = |page: &mut usize| match words.as_slice() {
        [head @ .., last] if !head.is_empty() && last.chars().all(|c| c.is_ascii_digit()) => {
            *page = last.parse().unwrap_or(1);
            head.join(" ")
        }
        _ => words.join(" "),
    };
    let numbers: Option<Vec<f64>> = words.iter().map(|w| w.parse().ok()).collect();
    let mode = p.mode.as_deref().unwrap_or("title").to_lowercase();
    let songs = match mode.as_str() {
        "ds" | "定数" => {
            let range = numbers.as_deref().and_then(|n| range_args(n, &mut page)).ok_or_else(|| {
                AppError::Msg("定数查歌参数错误，请输入正确格式，页数为可选：\n定数查歌「定数」「页数」\n定数查歌「最小定数」「最大定数」「页数」".into())
            })?;
            d.filter(&Filter { level_value: Some(range), ..Default::default() })
        }
        "bpm" => {
            let range = numbers.as_deref().and_then(|n| range_args(n, &mut page)).ok_or_else(|| {
                AppError::Msg("bpm查歌参数错误，请输入正确格式，页数为可选：\nbpm查歌「bpm」「页数」\nbpm查歌「最小bpm」「最大bpm」「页数」".into())
            })?;
            d.filter(&Filter { bpm: Some(range), ..Default::default() })
        }
        "artist" | "曲师" => d.filter(&Filter { artist: Some(&name_page(&mut page)), ..Default::default() }),
        "charter" | "谱师" => {
            d.filter(&Filter { charter: Some(&name_page(&mut page)), matched_diffs_only: true, ..Default::default() })
        }
        _ => d.filter(&Filter { title: Some(&name_page(&mut page)), ..Default::default() }),
    };
    let fmt = p.output_format.as_deref();
    match songs.len() {
        0 => Ok(Reply::Text(not_found.into())),
        1 => {
            chart_reply(
                songs.into_iter().next().unwrap_or_else(|| unreachable!()),
                optional_user(p.qq.as_deref()),
                "",
                fmt,
            )
            .await
        }
        2..=5 => Ok(Reply::Text(id_lines(songs.iter().map(|s| (&s.song_id, &s.song_name))))),
        n => list_reply(songs, page, format!("共找到「{n}」首曲目：\n"), fmt).await,
    }
}

#[derive(Debug, Deserialize, JsonSchema)]
pub struct AliasParams {
    /// Alias, title or song id (prefix "id" to force id lookup, e.g. "id834")
    pub song: String,
}

/// XXX有什么别名.
pub async fn alias_query(p: AliasParams) -> Result<Reply> {
    let d = data();
    ensure_data(&d)?;
    let raw = p.song.trim().to_lowercase();
    let not_found = || Ok(Reply::Text("未找到此歌曲\n可以使用「添加别名」指令给该乐曲添加别名".into()));
    let (find_id, name) = match raw.strip_prefix("id") {
        Some(rest) if rest.trim().chars().all(|c| c.is_ascii_digit()) && !rest.trim().is_empty() => {
            (true, rest.trim().to_string())
        }
        _ => (false, raw.clone()),
    };
    let mut aliases = if find_id { Vec::new() } else { d.alias_by_alias(&name) };
    if aliases.is_empty() {
        match name.parse::<u32>() {
            Ok(id) => aliases = d.alias_by_id(id),
            Err(_) => return not_found(),
        }
    }
    if aliases.is_empty() {
        return not_found();
    }
    if aliases.len() != 1 {
        let parts: Vec<String> = aliases.iter().map(|a| format!("ID：{}\n{}", a.song_id, a.alias.join("\n"))).collect();
        return Ok(Reply::Text(format!("找到{}个相同别名的曲目：\n{}", aliases.len(), parts.join("\n======\n"))));
    }
    let a = aliases[0];
    let real: Vec<&String> = a.alias.iter().filter(|x| x.to_lowercase() != a.song_name.to_lowercase()).collect();
    if real.is_empty() {
        return Ok(Reply::Text("该曲目没有别名".into()));
    }
    let list: Vec<&str> = real.iter().map(|s| s.as_str()).collect();
    Ok(Reply::Text(format!("该曲目有以下别名：\nID：{}\n{}", a.song_id, list.join("\n"))))
}

#[derive(Debug, Deserialize, JsonSchema)]
pub struct RandomParams {
    /// Level such as "13+" (required)
    pub level: String,
    /// Difficulty colour 绿/黄/红/紫/白 the level must be on (optional)
    pub difficulty: Option<String>,
    /// "dx" or "sd"/"标准" (optional)
    pub song_type: Option<String>,
    /// User QQ number, for personal rating gains (optional)
    pub qq: Option<String>,
    pub output_format: Option<String>,
}

/// 随个[dx/sd][绿黄红紫白]<等级>.
pub async fn random_song(p: RandomParams) -> Result<Reply> {
    let d = data();
    ensure_data(&d)?;
    let level = p.level.trim().to_string();
    let kinds: &[&str] = match p.song_type.as_deref().map(str::to_lowercase).as_deref() {
        Some("dx") => &["DX"],
        Some("sd" | "标准" | "standard") => &["SD"],
        _ => &["SD", "DX"],
    };
    let levels = [level.as_str()];
    let mut songs = d.filter(&Filter { level: Some(&levels), kind: Some(kinds), ..Default::default() });
    if let Some(ci) = p.difficulty.as_deref().filter(|s| !s.trim().is_empty()).and_then(parse_difficulty) {
        songs.retain(|s| s.difficulties.get(ci).is_some_and(|x| x.level == level));
    }
    let Some(song) = Rng::from_time().choice(&songs).cloned() else {
        return Ok(Reply::Text("没有这样的乐曲哦。".into()));
    };
    chart_reply(song, optional_user(p.qq.as_deref()), "", p.output_format.as_deref()).await
}

#[derive(Debug, Deserialize, JsonSchema)]
pub struct WhatParams {
    /// User QQ number (optional)
    pub qq: Option<String>,
    /// true = recommend a chart that can raise the user's rating (推分/上分/加分)
    pub rise: Option<bool>,
    pub output_format: Option<String>,
}

/// mai什么 / 今日mai打什么上分.
pub async fn mai_what(p: WhatParams) -> Result<Reply> {
    let d = data();
    ensure_data(&d)?;
    let user = optional_user(p.qq.as_deref());
    let mut song = d.random(&mut Rng::from_time()).cloned();
    if p.rise.unwrap_or(false)
        && let Some(u) = &user
        && let Some(s) = service::mai_what_rise(u).await
    {
        song = Some(s);
    }
    let song = song.ok_or_else(|| AppError::Msg("曲目库为空。".into()))?;
    chart_reply(song, user, "", p.output_format.as_deref()).await
}

#[derive(Debug, Deserialize, JsonSchema)]
pub struct FortuneParams {
    /// User QQ number
    pub qq: Option<String>,
    pub output_format: Option<String>,
}

/// 今日运势 / 今日舞萌.
pub async fn fortune(p: FortuneParams) -> Result<Reply> {
    let d = data();
    ensure_data(&d)?;
    let qq = require_qq(p.qq.as_deref())?;
    store::get_user(qq)?;
    let hash = qq_hash(qq.parse().unwrap_or(0));
    let mut msg = format!("今日人品值：{}\n", hash % 100);
    let mut h = hash;
    for item in FORTUNE.iter().take(11) {
        match h & 3 {
            3 => msg += &format!("宜 {item}\n"),
            0 => msg += &format!("忌 {item}\n"),
            _ => {}
        }
        h >>= 2;
    }
    let song = d.random(&mut Rng::new(hash)).cloned().ok_or_else(|| AppError::Msg("曲目库为空。".into()))?;
    let ds: Vec<String> = song.difficulties.iter().map(|x| py_float(x.level_value)).collect();
    msg += &format!(
        "{} Bot提醒您：打机时不要大力拍打或滑动哦\n今日推荐歌曲：ID.{} - {}\n{}",
        CONFIG.bot_name,
        song.song_id,
        song.song_name,
        ds.join("/")
    );
    if !wants_image(p.output_format.as_deref()) {
        return Ok(Reply::Text(msg));
    }
    let img = render(move || open(&cover_path(song.song_id)).map(|i| (*i).clone())).await?;
    Ok(Reply::image(img, "fortune", msg))
}
