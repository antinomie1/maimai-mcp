use super::{Reply, ensure_data, render, require_qq, wants_image};
use crate::core::clients::divingfish::DivingFish;
use crate::core::models::{Best50, Played, Player, Service, Theme, User};
use crate::core::music::data;
use crate::core::{service, store};
use crate::draw::{best50::draw_best50, text::text_to_image};
use crate::error::{AppError, Result};
use crate::util::cn_datetime;
use schemars::JsonSchema;
use serde::Deserialize;

pub const MESSAGE: &str = "可使用「主题」指令更换主题，「数据源」指令更换指定查分器。";

#[derive(Debug, Deserialize, JsonSchema)]
pub struct B50Params {
    /// User QQ number (from context)
    pub qq: Option<String>,
    /// DivingFish username to query another player instead of the QQ's binding
    pub username: Option<String>,
    /// true = AP 50 (best 50 among AP/AP+ charts)
    pub all_perfect: Option<bool>,
    /// "image" (default), "text" or "both"
    pub output_format: Option<String>,
}

fn summary(player: &Player, b: &Best50, ap: bool) -> String {
    let line = |i: usize, p: &Played| {
        format!("{}. [{}] {} ({}) {:.4}% Ra {}\n", i + 1, p.song_id, p.song_name, p.level, p.achievements, p.rating)
    };
    let mut out = format!(
        "【{} 的 {}】DX Rating: {} (B35: {} + B15: {})\n",
        player.name,
        if ap { "AP50" } else { "B50" },
        player.rating,
        b.sd_total,
        b.dx_total
    );
    out += "--- 旧版本 B35 ---\n";
    b.sd.iter().enumerate().for_each(|(i, p)| out += &line(i, p));
    out += "--- 新版本 B15 ---\n";
    b.dx.iter().enumerate().for_each(|(i, p)| out += &line(i, p));
    out + MESSAGE
}

pub async fn b50(p: B50Params) -> Result<Reply> {
    let d = data();
    ensure_data(&d)?;
    let username = p.username.as_deref().map(str::trim).filter(|u| !u.is_empty()).map(str::to_string);
    let qq = p.qq.as_deref().map(str::trim).filter(|q| !q.is_empty());
    let user = match (qq, &username) {
        (Some(qq), None) => service::auth_user(qq)?,
        (Some(qq), Some(_)) => store::get_user(qq)?,
        (None, Some(_)) => User::default(),
        (None, None) => return Err(AppError::Msg("请提供 QQ 号或水鱼用户名。".into())),
    };
    let ap = p.all_perfect.unwrap_or(false);
    if ap && user.service == Service::Lxns && username.is_none() && crate::config::CONFIG.lxns_dev_token.is_none() {
        return Err(AppError::Msg("BOT管理员尚未配置落雪开发者Token，无法查询 AP50。".into()));
    }
    let (player, b) = service::best50(&user, username.as_deref(), ap).await?;
    let text = summary(&player, &b, ap);
    if !wants_image(p.output_format.as_deref()) {
        return Ok(Reply::Text(text));
    }
    let (theme, svc) =
        if username.is_some() { (Theme::PrismPlus, Service::DivingFish) } else { (user.theme, user.service) };
    let extras = service::b50_extras(&player, username.is_none().then_some(user.qq.as_str())).await;
    let img = render(move || Some(draw_best50(&d, &player, &b, theme, svc, extras))).await?;
    Ok(Reply::image(img, "b50", text))
}

#[derive(Debug, Deserialize, JsonSchema)]
pub struct RankingParams {
    /// DivingFish username to locate in the ranking (optional)
    pub name: Option<String>,
    /// Page of the ranking list (50 per page)
    pub page: Option<usize>,
    /// Set together with `mine=true` to find the QQ user's own rank
    pub qq: Option<String>,
    /// true = 我的排名 (rank of the QQ user's bound DivingFish account)
    pub mine: Option<bool>,
    pub output_format: Option<String>,
}

/// 查看排名 / 我的排名 (DivingFish only).
pub async fn rating_ranking(p: RankingParams) -> Result<Reply> {
    let ranks = DivingFish::rating_ranking().await?;
    if p.mine.unwrap_or(false) {
        let qq = require_qq(p.qq.as_deref())?;
        let info = DivingFish::new(Some(qq), None).query_b50().await?;
        let me = info.username.unwrap_or_default();
        return Ok(Reply::Text(match ranks.iter().position(|(u, _)| *u == me) {
            Some(i) => format!("您的Rating为「{}」，排名第「{}」名", ranks[i].1, i + 1),
            None => "未在查分器排行榜中找到您的记录。".into(),
        }));
    }
    let now = cn_datetime();
    if let Some(name) = p.name.as_deref().map(|n| n.trim().to_lowercase()).filter(|n| !n.is_empty()) {
        return Ok(Reply::Text(match ranks.iter().position(|(u, _)| u.to_lowercase() == name) {
            Some(i) => format!("截止至「{now}」玩家「{}」\n在查分器已注册用户 RA 排行第「{}」位", ranks[i].0, i + 1),
            None => format!("未在查分器排行榜前「{}」名中找到玩家「{name}」", ranks.len()),
        }));
    }
    let total_pages = ranks.len().div_ceil(50).max(1);
    let page = p.page.unwrap_or(1).clamp(1, total_pages);
    let start = (page - 1) * 50;
    let mut msg = format!("截止至「{now}」，查分器已注册用户 RA 排行：\n");
    for (i, (u, ra)) in ranks.iter().enumerate().skip(start).take(50) {
        msg += &format!("No.{:02}.「{ra}」 {u}\n", i + 1);
    }
    msg += &format!("\n第「{page} / {total_pages}」页，共「{}」名玩家", ranks.len());
    if !wants_image(p.output_format.as_deref()) {
        return Ok(Reply::Text(msg));
    }
    let text = msg.clone();
    let img = render(move || Some(text_to_image(&text))).await?;
    Ok(Reply::image(img, "ranking", msg))
}

#[derive(Debug, Deserialize, JsonSchema)]
pub struct GroupRankParams {
    /// QQ numbers of group members to rank
    pub qq_list: Vec<String>,
}

/// Group leaderboard by DX Rating (each member's configured source).
pub async fn group_rank(p: GroupRankParams) -> Result<Reply> {
    let mut set = tokio::task::JoinSet::new();
    for qq in p.qq_list.into_iter().filter(|q| !q.trim().is_empty()) {
        set.spawn(async move {
            let user = store::get_user(&qq).ok()?;
            let (player, _) = service::best50(&user, None, false).await.ok()?;
            Some((qq, player.name, player.rating))
        });
    }
    let mut ranks: Vec<(String, String, u32)> = set.join_all().await.into_iter().flatten().collect();
    if ranks.is_empty() {
        return Ok(Reply::Text("未查询到任何成员的成绩数据。".into()));
    }
    ranks.sort_by_key(|r| std::cmp::Reverse(r.2));
    let mut out = format!("【群内 DX Rating 排行】共 {} 位玩家\n", ranks.len());
    for (i, (qq, name, ra)) in ranks.iter().enumerate() {
        out += &format!("No.{:02}.「{ra}」 {name} ({qq})\n", i + 1);
    }
    Ok(Reply::Text(out))
}
