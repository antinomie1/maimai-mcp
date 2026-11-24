//! Business logic shared by the tools (upstream `core/handler.py`).

use crate::core::calc::compute_rating;
use crate::core::clients::divingfish::{DfRecord, DfUser, DivingFish, RecordsQuery};
use crate::core::clients::lxns::{self, LxBest50, LxScore, Lxns};
use crate::core::constants::{ACHIEVEMENT_LIST, all_versions, new_version_name, new_version_plate};
use crate::core::models::{Best50, NotPlayed, Plate, Played, Player, Rise, Service, Slot, Song, Theme, Trophy, User};
use crate::core::music::{Filter, MaiData, data};
use crate::core::store;
use crate::error::{AppError, Result};
use crate::util::Rng;

/// Loads a user and checks LXNS authorization (upstream `GetUserAndAuth`).
pub fn auth_user(qq: &str) -> Result<User> {
    let user = store::get_user(qq)?;
    if user.service == Service::Lxns && user.lxns_access_token.is_none() {
        return Err(AppError::LxNotBound);
    }
    Ok(user)
}

fn lx_played(s: &LxScore, d: &MaiData) -> Played {
    let song_id = if s.kind == "dx" { s.id + 10000 } else { s.id };
    Played {
        song_id,
        song_name: s.song_name.clone(),
        level: s.level.clone(),
        level_index: s.level_index,
        level_value: d.level_value(song_id, s.level_index),
        kind: match s.kind.as_str() {
            "standard" => "SD".into(),
            "utage" => "DX".into(),
            k => k.to_uppercase(),
        },
        rating: s.dx_rating.unwrap_or(0.0) as u32,
        achievements: s.achievements,
        rate: s.rate.clone().filter(|r| !r.is_empty()),
        fc: s.fc.clone().filter(|r| !r.is_empty()),
        fs: s.fs.clone().filter(|r| !r.is_empty()),
        dx_score: s.dx_score,
    }
}

fn played_list(records: &[DfRecord]) -> (u32, Vec<Played>) {
    (records.iter().map(|r| r.ra).sum(), records.iter().map(DfRecord::to_played).collect())
}

fn df_best50(user: &DfUser, all_records: bool, d: &MaiData) -> Best50 {
    if !all_records {
        let (sd_total, sd) = played_list(&user.charts.sd);
        let (dx_total, dx) = played_list(&user.charts.dx);
        return Best50 { sd_total, dx_total, sd, dx };
    }
    let mut records = user.records.clone();
    records.sort_by(|a, b| {
        (b.ra, b.achievements).partial_cmp(&(a.ra, a.achievements)).unwrap_or(std::cmp::Ordering::Equal)
    });
    let new_ids = d.plate(new_version_plate());
    let (new, old): (Vec<DfRecord>, Vec<DfRecord>) = records.into_iter().partition(|r| new_ids.contains(&r.song_id));
    let (sd_total, sd) = played_list(&old[..old.len().min(35)]);
    let (dx_total, dx) = played_list(&new[..new.len().min(15)]);
    Best50 { sd_total, dx_total, sd, dx }
}

fn lx_best50(b: &LxBest50, d: &MaiData) -> Best50 {
    Best50 {
        sd_total: b.standard_total,
        dx_total: b.dx_total,
        sd: b.standard.iter().map(|s| lx_played(s, d)).collect(),
        dx: b.dx.iter().map(|s| lx_played(s, d)).collect(),
    }
}

fn df_player(u: &DfUser) -> Player {
    Player {
        name: u.nickname.clone().unwrap_or_default(),
        rating: u.rating.unwrap_or(0),
        course_rank: u.additional_rating.unwrap_or(0),
        plate: u.plate.clone().filter(|p| !p.is_empty()).map_or(Plate::None, Plate::Name),
        ..Default::default()
    }
}

/// Player info + Best 50 (`get_best50`). `username` forces DivingFish.
pub async fn best50(user: &User, username: Option<&str>, all_perfect: bool) -> Result<(Player, Best50)> {
    let d = data();
    if username.is_some() || user.service == Service::DivingFish {
        let qq = (!user.qq.is_empty()).then_some(user.qq.as_str());
        let api = DivingFish::new(qq, username);
        let info = if all_perfect {
            let q = RecordsQuery { fc: vec!["ap".into(), "app".into()], ..Default::default() };
            api.records(&q).await?
        } else {
            api.query_b50().await?
        };
        return Ok((df_player(&info), df_best50(&info, all_perfect, &d)));
    }
    let mut api = Lxns::for_user(user)?;
    let p = api.player().await?;
    let b = if all_perfect { lxns::ap50(p.friend_code).await? } else { api.best50().await? };
    let player = Player {
        name: p.name,
        rating: p.rating,
        course_rank: p.course_rank,
        class_rank: p.class_rank,
        friend_code: p.friend_code,
        plate: p.name_plate.map_or(Plate::None, |c| Plate::Id(c.id)),
        icon: p.icon.map(|c| c.id),
        trophy: p.trophy.map(|t| Trophy { name: t.name, color: t.color.unwrap_or_else(|| "Normal".into()) }),
    };
    Ok((player, lx_best50(&b, &d)))
}

/// All played charts, optionally filtered server-side (`get_player_result`).
pub async fn results(user: &User, query: &RecordsQuery) -> Result<Vec<Played>> {
    match user.service {
        Service::DivingFish => {
            let info = DivingFish::new(Some(&user.qq), None).records(query).await?;
            Ok(info.records.iter().map(DfRecord::to_played).collect())
        }
        Service::Lxns => {
            let d = data();
            Ok(Lxns::for_user(user)?.all_best().await?.iter().map(|s| lx_played(s, &d)).collect())
        }
    }
}

/// Played / not-played slot per difficulty of one song (`draw_play_data`).
pub async fn song_slots(user: &User, song: &Song) -> Result<Vec<Slot>> {
    let played: Vec<Played> = match user.service {
        Service::DivingFish => {
            let q = RecordsQuery { song_id: Some(song.song_id), ..Default::default() };
            let info = DivingFish::new(Some(&user.qq), None).records(&q).await?;
            info.records.iter().map(DfRecord::to_played).collect()
        }
        Service::Lxns => {
            let kind = match song.song_id {
                0..10000 => "standard",
                10000..100_000 => "dx",
                _ => "utage",
            };
            let d = data();
            let list = Lxns::for_user(user)?.song_bests(song.song_id % 10000, kind).await?;
            list.iter().map(|s| lx_played(s, &d)).collect()
        }
    };
    if played.is_empty() {
        return Err(AppError::MusicNotPlay);
    }
    let mut slots: Vec<Slot> = song
        .difficulties
        .iter()
        .map(|d| {
            Slot::NotPlayed(NotPlayed { song_id: song.song_id, level_index: d.level_index, level_value: d.level_value })
        })
        .collect();
    for mut p in played {
        if let Some(slot) = slots.get_mut(p.level_index as usize) {
            if let Slot::NotPlayed(np) = slot {
                p.level_value = np.level_value;
            }
            *slot = Slot::Played(p);
        }
    }
    Ok(slots)
}

/// Rise candidates for one half of B50 (`get_rise_score_list`).
fn rise_list(
    d: &MaiData,
    all: &[Played],
    dx: bool,
    best: &[Played],
    level: Option<&str>,
    score: Option<u32>,
    rng: &mut Rng,
) -> (Vec<Rise>, u32) {
    let capacity = if dx { 15 } else { 35 };
    let lowest = if best.len() >= capacity { best.iter().map(|p| p.rating).min().unwrap_or(0) } else { 0 };
    let target = score.unwrap_or(1).max(1);
    let coef = ACHIEVEMENT_LIST[ACHIEVEMENT_LIST.len() - 1] / 100.0 * 22.4;
    let min_ds = (f64::from(lowest + target) / coef * 10.0).ceil() / 10.0;
    let ds = (level.is_none() && lowest != 0).then_some((min_ds, min_ds + 1.0));
    let newest = new_version_name();
    let versions: Vec<&str> = if dx {
        vec![newest]
    } else {
        let all_v = all_versions();
        all_v[..all_v.iter().position(|v| *v == newest).unwrap_or(all_v.len())].to_vec()
    };
    let levels: Vec<&str> = level.into_iter().collect();
    let f = Filter {
        level: level.map(|_| levels.as_slice()),
        level_value: ds,
        version_str: Some(&versions),
        matched_diffs_only: true,
        ..Default::default()
    };
    let find = |list: &[Played], id: u32, li: u8| list.iter().find(|p| p.song_id == id && p.level_index == li).cloned();
    let mut out = Vec::new();
    for song in d.filter(&f) {
        if song.song_id >= 100_000 {
            continue;
        }
        for diff in &song.difficulties {
            let in_best = find(best, song.song_id, diff.level_index);
            let old = in_best.clone().or_else(|| find(all, song.song_id, diff.level_index));
            if old.as_ref().is_some_and(|o| o.achievements >= 100.5) {
                continue;
            }
            let old_ra = in_best.as_ref().map_or(lowest, |p| p.rating);
            for &ach in &ACHIEVEMENT_LIST[ACHIEVEMENT_LIST.len() - 4..] {
                let new_ra = compute_rating(diff.level_value, ach);
                if new_ra < old_ra + target || old.as_ref().is_some_and(|o| new_ra <= o.rating) {
                    continue;
                }
                out.push(Rise {
                    song_id: song.song_id,
                    song_name: song.song_name.clone(),
                    level_index: diff.level_index,
                    kind: song.kind.clone(),
                    rating: new_ra,
                    achievements: ach,
                    rate: crate::core::calc::compute_rate(ach).to_lowercase(),
                    level_value: diff.level_value,
                    old_rating: old.as_ref().map_or(0, |o| o.rating),
                    old_achievements: old.as_ref().map_or(0.0, |o| o.achievements),
                    old_rate: old.as_ref().and_then(|o| o.rate.clone()).unwrap_or_else(|| "d".into()),
                });
                break;
            }
        }
    }
    let mut sampled = rng.sample(&out, 5);
    sampled.sort_by(|a, b| b.level_value.partial_cmp(&a.level_value).unwrap_or(std::cmp::Ordering::Equal));
    (sampled, lowest)
}

pub type RiseResult = (Vec<Rise>, u32, Vec<Rise>, u32);

/// 我要上分 (`draw_rise_score_list`).
pub async fn rise(user: &User, level: Option<&str>, score: Option<u32>) -> Result<RiseResult> {
    let (_, b50) = best50(user, None, false).await?;
    let all = results(user, &RecordsQuery::default()).await?;
    let d = data();
    let mut rng = Rng::from_time();
    let (sd, sd_low) = rise_list(&d, &all, false, &b50.sd, level, score, &mut rng);
    let (dx, dx_low) = rise_list(&d, &all, true, &b50.dx, level, score, &mut rng);
    if sd.is_empty() && dx.is_empty() {
        return Err(AppError::NoRecommendation);
    }
    Ok((sd, sd_low, dx, dx_low))
}

/// Song that can raise the B50 floor (`get_mai_what`).
pub async fn mai_what_rise(user: &User) -> Option<Song> {
    let (_, b50) = best50(user, None, false).await.ok()?;
    let mut rng = Rng::from_time();
    let half = if rng.below(2) == 0 { &b50.sd } else { &b50.dx };
    let last = half.last().filter(|l| l.rating != 0)?;
    let ignore: Vec<u32> = half.iter().filter(|m| m.achievements < 100.5).map(|m| m.song_id).collect();
    let ds = crate::util::round_n(f64::from(last.rating) / 22.4, 1);
    let list: Vec<Song> = data()
        .filter(&Filter { level_value: Some((ds, ds + 1.0)), ..Default::default() })
        .into_iter()
        .filter(|s| !ignore.contains(&s.song_id))
        .collect();
    rng.choice(&list).cloned()
}

/// Player context for chart cards: (theme, is_full, best list of the song's half).
pub async fn chart_context(user: Option<&User>, song: &Song) -> (Theme, Option<(bool, Vec<Played>)>) {
    let Some(user) = user else {
        return (Theme::PrismPlus, None);
    };
    if song.song_id >= 100_000 {
        return (user.theme, None);
    }
    match best50(user, None, false).await {
        Ok((_, b)) => {
            let (list, cap) = if song.isnew { (b.dx, 15) } else { (b.sd, 35) };
            (user.theme, Some((list.len() == cap, list)))
        }
        Err(e) => {
            log::debug!("谱面信息未计算个人加分: {e}");
            (user.theme, None)
        }
    }
}

/// Fetches LXNS collection images / QQ avatar for the B50 header.
pub async fn b50_extras(player: &Player, qq: Option<&str>) -> crate::draw::best50::Extras {
    use crate::core::clients::{online_asset, qq_avatar};
    let decode = |b: Option<Vec<u8>>| b.and_then(|b| image::load_from_memory(&b).ok()).map(|i| i.to_rgba8());
    let online = crate::config::CONFIG.assets_online;
    let plate = match (&player.plate, online) {
        (Plate::Id(id), true) => decode(online_asset("plate", *id).await),
        _ => None,
    };
    let icon = match (player.icon, qq) {
        (Some(id), _) if online => decode(online_asset("icon", id).await),
        (Some(_), _) => None,
        (None, Some(qq)) => decode(qq_avatar(qq).await),
        _ => None,
    };
    crate::draw::best50::Extras { plate, icon }
}
