//! Online data update (songs, aliases, plates) with local-file fallback,
//! upstream-compatible merging and a daily scheduler.

use crate::config::CONFIG;
use crate::core::clients::{divingfish::DivingFish, lxns, yuzuchan};
use crate::core::constants::DX_CN_VERSION;
use crate::core::models::{Alias, Difficulty, Notes, Song, Stats};
use crate::core::music::{self, MERGE_ALIAS_FILE, MERGE_MUSIC_FILE, MaiData, PLATE_FILE};
use crate::error::{AppError, Result};
use serde::Deserialize;
use serde_json::Value;
use std::collections::HashMap;
use std::future::Future;
use std::hash::{Hash, Hasher};
use tokio::sync::Mutex;

static UPDATING: Mutex<()> = Mutex::const_new(());

/// Downloads `fetch` into `file`, falling back to the cached file on failure.
async fn fetch_or_cache(file: &str, fetch: impl Future<Output = Result<Vec<u8>>>) -> Result<Vec<u8>> {
    let path = CONFIG.data_file(file);
    match fetch.await {
        Ok(bytes) if serde_json::from_slice::<Value>(&bytes).is_ok() => {
            let _ = std::fs::write(&path, &bytes);
            Ok(bytes)
        }
        res => {
            let reason = res.err().map_or_else(|| "invalid JSON".to_string(), |e| e.to_string());
            log::warn!("获取 {file} 失败（{reason}），使用本地暂存文件");
            std::fs::read(&path).map_err(|_| AppError::Msg(format!("{file} 获取失败且无本地暂存文件")))
        }
    }
}

// ------------------------------------------------------------ raw formats

#[derive(Deserialize)]
struct DfMusic {
    id: String,
    title: String,
    #[serde(rename = "type")]
    kind: String,
    ds: Vec<f64>,
    level: Vec<String>,
    charts: Vec<DfChart>,
    basic_info: DfBasic,
}

#[derive(Deserialize)]
struct DfChart {
    #[serde(default)]
    notes: Vec<u32>,
    charter: Option<String>,
}

#[derive(Deserialize)]
struct DfBasic {
    #[serde(default)]
    artist: String,
    #[serde(default)]
    genre: String,
    #[serde(default)]
    bpm: f64,
    #[serde(rename = "from", default)]
    version: String,
    #[serde(default)]
    is_new: bool,
}

#[derive(Deserialize)]
struct LxSongs {
    songs: Vec<LxSong>,
    versions: Vec<LxVersion>,
}

#[derive(Deserialize)]
struct LxVersion {
    title: String,
    version: u32,
}

#[derive(Deserialize)]
struct LxSong {
    id: u32,
    title: String,
    #[serde(default)]
    artist: String,
    #[serde(default)]
    genre: String,
    #[serde(default)]
    bpm: f64,
    difficulties: LxDiffs,
}

#[derive(Deserialize, Default)]
#[serde(default)]
struct LxDiffs {
    standard: Vec<LxDiff>,
    dx: Vec<LxDiff>,
    utage: Vec<LxDiff>,
}

#[derive(Deserialize, Clone)]
struct LxDiff {
    difficulty: u8,
    level: String,
    level_value: f64,
    #[serde(default)]
    note_designer: String,
    #[serde(default)]
    version: u32,
    #[serde(default)]
    notes: Value,
    kanji: Option<String>,
    description: Option<String>,
    is_buddy: Option<bool>,
}

fn df_notes(n: &[u32]) -> Notes {
    let g = |i: usize| n.get(i).copied().unwrap_or(0);
    let (tap, hold, slide, touch, brk) =
        if n.len() >= 5 { (g(0), g(1), g(2), g(3), g(4)) } else { (g(0), g(1), g(2), 0, g(3)) };
    Notes { total: tap + hold + slide + touch + brk, tap, hold, slide, touch, brk }
}

fn diff(level_index: u8, level: &str, level_value: f64, designer: &str, notes: Notes) -> Difficulty {
    Difficulty {
        level_index,
        level: level.to_string(),
        level_value,
        note_designer: designer.to_string(),
        dx_score: notes.total * 3,
        notes,
        stats: None,
    }
}

/// Port of upstream `merge_music_data`.
fn merge_music(df: Vec<DfMusic>, lx: Option<LxSongs>, stats: HashMap<String, Vec<Stats>>) -> Vec<Song> {
    let mut songs: Vec<Song> = Vec::new();
    let mut index: HashMap<u32, usize> = HashMap::new();
    for raw in df {
        let Ok(song_id) = raw.id.parse::<u32>() else {
            continue;
        };
        let difficulties = raw
            .ds
            .iter()
            .enumerate()
            .map(|(n, ds)| {
                let chart = raw.charts.get(n);
                let notes = df_notes(chart.map_or(&[][..], |c| &c.notes));
                let level = raw.level.get(n).map_or("", String::as_str);
                diff(n as u8, level, *ds, chart.and_then(|c| c.charter.as_deref()).unwrap_or(""), notes)
            })
            .collect();
        index.insert(song_id, songs.len());
        songs.push(Song {
            song_id,
            song_name: raw.title,
            artist: raw.basic_info.artist,
            genre: raw.basic_info.genre,
            bpm: raw.basic_info.bpm,
            version_str: raw.basic_info.version,
            version_int: 0,
            kind: raw.kind,
            isnew: raw.basic_info.is_new,
            difficulties,
            kanji: None,
            description: None,
            is_buddy: None,
        });
    }

    if let Some(lx) = lx {
        let ver_map: HashMap<u32, &str> = lx.versions.iter().map(|v| (v.version, v.title.as_str())).collect();
        let new_version = lx.versions.last().map_or(0, |v| v.version);
        for raw in &lx.songs {
            let d = &raw.difficulties;
            let mut targets: Vec<(&str, u32, &[LxDiff], bool)> = Vec::new();
            if raw.id < 10000 {
                targets.push(("SD", raw.id, &d.standard, false));
                targets.push(("DX", raw.id + 10000, &d.dx, false));
            } else if raw.id < 100_000 {
                targets.push(("DX", raw.id, &d.dx, false));
                targets.push(("SD", raw.id - 10000, &d.standard, false));
            } else {
                targets.push(("DX", raw.id, &d.utage, true));
            }
            for (kind, sid, diffs, utage) in targets {
                let Some(base) = diffs.first() else { continue };
                let lx_notes = |v: &Value| serde_json::from_value::<Notes>(v.clone()).unwrap_or_default();
                let apply_extra = |song: &mut Song| {
                    if utage {
                        song.kanji = base.kanji.clone();
                        song.description = base.description.clone();
                        song.is_buddy = base.is_buddy;
                    } else if song.difficulties.len() < diffs.len() {
                        let last = &diffs[diffs.len() - 1];
                        let level_index = last.difficulty;
                        song.difficulties.push(diff(
                            level_index,
                            &last.level,
                            last.level_value,
                            &last.note_designer,
                            lx_notes(&last.notes),
                        ));
                    }
                };
                if let Some(&i) = index.get(&sid) {
                    songs[i].version_int = base.version;
                    apply_extra(&mut songs[i]);
                    continue;
                }
                let difficulties = if sid > 100_000 {
                    let parts = match (base.notes.get("left"), base.notes.get("right")) {
                        (Some(l), Some(r)) => vec![lx_notes(l), lx_notes(r)],
                        _ => vec![lx_notes(&base.notes)],
                    };
                    parts
                        .into_iter()
                        .enumerate()
                        .map(|(n, notes)| diff(n as u8, &base.level, base.level_value, &base.note_designer, notes))
                        .collect()
                } else {
                    diffs
                        .iter()
                        .enumerate()
                        .map(|(n, x)| diff(n as u8, &x.level, x.level_value, &x.note_designer, lx_notes(&x.notes)))
                        .collect()
                };
                let title = ver_map.get(&(base.version - base.version % 100)).copied().unwrap_or_default();
                let version_str = DX_CN_VERSION.iter().find(|(t, _, _)| *t == title).map_or(title, |(_, _, n)| n);
                let mut song = Song {
                    song_id: sid,
                    song_name: raw.title.clone(),
                    artist: raw.artist.clone(),
                    genre: raw.genre.clone(),
                    bpm: raw.bpm,
                    version_str: version_str.to_string(),
                    version_int: base.version,
                    kind: kind.to_string(),
                    isnew: new_version == base.version,
                    difficulties,
                    kanji: None,
                    description: None,
                    is_buddy: None,
                };
                apply_extra(&mut song);
                index.insert(sid, songs.len());
                songs.push(song);
            }
        }
    }

    for (sid, list) in stats {
        let Some(&i) = sid.parse::<u32>().ok().and_then(|id| index.get(&id)) else {
            continue;
        };
        for (d, s) in songs[i].difficulties.iter_mut().zip(list) {
            d.stats = Some(s);
        }
    }
    songs
}

/// Port of upstream `merge_alias_data` (yuzu + lxns + local).
fn merge_aliases(yuzu: Vec<yuzuchan::YuzuAlias>, lx: Option<Value>, local: HashMap<String, Vec<String>>) -> Vec<Alias> {
    let mut map: HashMap<u32, Vec<String>> = HashMap::new();
    let mut names: HashMap<u32, String> = HashMap::new();
    let mut add = |id: u32, aliases: &[String]| {
        let list = map.entry(id).or_default();
        for a in aliases {
            if !list.contains(a) {
                list.push(a.clone());
            }
        }
    };
    let yuzu_ids: std::collections::HashSet<u32> = yuzu.iter().map(|a| a.song_id).collect();
    for item in &yuzu {
        add(item.song_id, &item.alias);
        if !item.name.is_empty() {
            names.entry(item.song_id).or_insert_with(|| item.name.clone());
        }
    }
    let lx_items = lx.as_ref().and_then(|v| v.get("aliases")).and_then(Value::as_array).cloned().unwrap_or_default();
    for item in lx_items {
        let id = item.get("song_id").and_then(Value::as_u64).unwrap_or(0) as u32;
        let aliases: Vec<String> =
            serde_json::from_value(item.get("aliases").cloned().unwrap_or_default()).unwrap_or_default();
        if yuzu_ids.contains(&id) {
            add(id, &aliases);
        }
        if id < 10000 && yuzu_ids.contains(&(id + 10000)) {
            add(id + 10000, &aliases);
        }
    }
    for (id, aliases) in local {
        if let Ok(id) = id.parse() {
            add(id, &aliases);
        }
    }
    let mut out: Vec<Alias> = map
        .into_iter()
        .filter(|(_, a)| !a.is_empty())
        .map(|(song_id, alias)| Alias { song_id, song_name: names.get(&song_id).cloned().unwrap_or_default(), alias })
        .collect();
    out.sort_by_key(|a| a.song_id);
    out
}

fn write_json<T: serde::Serialize>(file: &str, value: &T) {
    match serde_json::to_vec_pretty(value) {
        Ok(bytes) => {
            if let Err(e) = std::fs::write(CONFIG.data_file(file), bytes) {
                log::warn!("写入 {file} 失败: {e}");
            }
        }
        Err(e) => log::warn!("序列化 {file} 失败: {e}"),
    }
}

pub async fn fetch_music() -> Result<Vec<Song>> {
    let (music, chart) = tokio::join!(
        fetch_or_cache("music_data.json", DivingFish::download("music_data")),
        fetch_or_cache("music_chart.json", DivingFish::download("chart_stats")),
    );
    let df: Vec<DfMusic> = serde_json::from_slice(&music?)?;
    let stats: HashMap<String, Vec<Stats>> = chart
        .ok()
        .and_then(|b| serde_json::from_slice::<Value>(&b).ok())
        .and_then(|v| serde_json::from_value(v.get("charts")?.clone()).ok())
        .unwrap_or_default();
    let lx = if CONFIG.lxns_dev_token.is_some() {
        fetch_or_cache("lxns_music_data.json", lxns::download("/song/list?notes=true")).await.ok().and_then(|b| {
            serde_json::from_slice::<LxSongs>(&b).map_err(|e| log::warn!("落雪曲目数据解析失败: {e}")).ok()
        })
    } else {
        log::warn!("未配置落雪开发者Token，跳过获取「落雪」曲目数据源");
        None
    };
    let songs = merge_music(df, lx, stats);
    write_json(MERGE_MUSIC_FILE, &songs);
    Ok(songs)
}

pub async fn fetch_aliases() -> Result<Vec<Alias>> {
    let yuzu: Vec<yuzuchan::YuzuAlias> =
        serde_json::from_slice(&fetch_or_cache("music_alias.json", yuzuchan::download_aliases()).await?)?;
    let lx = if CONFIG.lxns_dev_token.is_some() {
        fetch_or_cache("lxns_music_alias.json", lxns::download("/alias/list"))
            .await
            .ok()
            .and_then(|b| serde_json::from_slice(&b).ok())
    } else {
        None
    };
    let local: HashMap<String, Vec<String>> = std::fs::read(CONFIG.data_file("local_music_alias.json"))
        .ok()
        .and_then(|b| serde_json::from_slice(&b).ok())
        .unwrap_or_default();
    let aliases = merge_aliases(yuzu, lx, local);
    write_json(MERGE_ALIAS_FILE, &aliases);
    Ok(aliases)
}

pub async fn fetch_plates() -> Result<HashMap<String, Vec<u32>>> {
    Ok(serde_json::from_slice(&fetch_or_cache(PLATE_FILE, yuzuchan::download_plates()).await?)?)
}

/// Fingerprint of everything the pre-rendered tables depend on.
fn fingerprint(d: &MaiData) -> String {
    let mut h = std::collections::hash_map::DefaultHasher::new();
    for s in &d.songs {
        s.song_id.hash(&mut h);
        s.version_str.hash(&mut h);
        for x in &s.difficulties {
            x.level.hash(&mut h);
            x.level_value.to_bits().hash(&mut h);
        }
    }
    let mut keys: Vec<_> = d.plates.iter().collect();
    keys.sort();
    keys.hash(&mut h);
    format!("{:x}-{}", h.finish(), CONFIG.bot_name)
}

/// Re-renders rating/plate tables when data changed or files are missing.
pub async fn refresh_tables(force: bool) -> Result<String> {
    let d = music::data();
    if d.songs.is_empty() {
        return Err(AppError::Msg("曲目数据为空，无法生成定数表与完成表。".into()));
    }
    let fp = fingerprint(&d);
    let fp_file = CONFIG.static_dir.join("mai/rating_table/.fingerprint");
    if !force && std::fs::read_to_string(&fp_file).is_ok_and(|s| s == fp) && crate::draw::update_table::tables_exist() {
        return Ok("定数表与完成表已是最新。".into());
    }
    let msg = tokio::task::spawn_blocking(move || crate::draw::update_table::update_all(&d))
        .await
        .map_err(|e| AppError::Other(e.to_string()))?;
    let _ = std::fs::write(fp_file, fp);
    Ok(msg)
}

/// Which data sets to refresh.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Target {
    All,
    Music,
    Alias,
    Plate,
    Tables,
}

fn take<T>(report: &mut Vec<String>, name: &str, r: Option<Result<T>>, old: impl FnOnce() -> T) -> T {
    match r {
        Some(Ok(v)) => {
            report.push(format!("{name}更新完成"));
            v
        }
        Some(Err(e)) => {
            report.push(format!("{name}更新失败：{e}"));
            old()
        }
        None => old(),
    }
}

pub async fn update(target: Target) -> String {
    let _guard = UPDATING.lock().await;
    let cur = music::data();
    let want = |t: Target| target == Target::All || target == t;
    let mut report = Vec::new();
    let (songs, aliases, plates) = tokio::join!(
        async { if want(Target::Music) { Some(fetch_music().await) } else { None } },
        async { if want(Target::Alias) { Some(fetch_aliases().await) } else { None } },
        async { if want(Target::Plate) { Some(fetch_plates().await) } else { None } },
    );
    let songs = take(&mut report, "曲目数据", songs, || cur.songs.clone());
    let aliases = take(&mut report, "别名数据", aliases, || cur.aliases.clone());
    let plates = take(&mut report, "牌子数据", plates, || cur.plates.clone());
    if target != Target::Tables {
        music::set_data(MaiData::new(songs, aliases, plates));
    }
    if target != Target::Alias {
        match refresh_tables(target == Target::Tables).await {
            Ok(m) => report.push(m),
            Err(e) => report.push(format!("表格生成失败：{e}")),
        }
    }
    let msg = report.join("\n");
    log::info!("maimaiDX数据更新：{}", msg.replace('\n', "；"));
    msg
}

/// Updates at startup, then daily at `MAIMAI_UPDATE_HOUR` (UTC+8).
pub fn spawn_scheduler() {
    tokio::spawn(async {
        update(Target::All).await;
        let Some(hour) = CONFIG.update_hour else {
            return;
        };
        loop {
            let now = crate::util::cn_now() % 86_400;
            let target = u64::from(hour) * 3600;
            let wait = if now < target { target - now } else { 86_400 - now + target };
            tokio::time::sleep(std::time::Duration::from_secs(wait.max(60))).await;
            update(Target::All).await;
        }
    });
}
