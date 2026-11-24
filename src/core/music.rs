//! In-memory song, alias and plate data (upstream `MusicList`, `AliasList`,
//! `total_plate_id_list`, `total_level_data`). Swapped atomically on update.

use crate::config::CONFIG;
use crate::core::models::{Alias, Difficulty, Song};
use crate::util::Rng;
use std::collections::HashMap;
use std::sync::{Arc, LazyLock, RwLock};

pub const MERGE_MUSIC_FILE: &str = "merge_music_data.json";
pub const MERGE_ALIAS_FILE: &str = "merge_music_alias.json";
pub const PLATE_FILE: &str = "plate_data.json";

/// One chart of the level table (upstream `SimpleSong`).
#[derive(Debug, Clone)]
pub struct Chart {
    pub song_id: u32,
    pub version_str: String,
    pub kind: String,
    pub diff: Difficulty,
}

/// `level -> [(ds "13.7", charts)]`, both sorted descending.
pub type LevelData = Vec<(String, Vec<(String, Vec<Chart>)>)>;

#[derive(Debug, Default)]
pub struct MaiData {
    pub songs: Vec<Song>,
    index: HashMap<u32, usize>,
    pub aliases: Vec<Alias>,
    pub plates: HashMap<String, Vec<u32>>,
    pub level_data: LevelData,
}

/// `MusicList.filter` arguments. Ranges are inclusive; a scalar is `(v, v)`.
#[derive(Default)]
pub struct Filter<'a> {
    pub level: Option<&'a [&'a str]>,
    pub level_value: Option<(f64, f64)>,
    pub kind: Option<&'a [&'a str]>,
    pub title: Option<&'a str>,
    pub artist: Option<&'a str>,
    pub charter: Option<&'a str>,
    pub bpm: Option<(f64, f64)>,
    pub version_str: Option<&'a [&'a str]>,
    /// `all_diff=False`: keep only matching difficulties.
    pub matched_diffs_only: bool,
}

fn level_key(lv: &str) -> (f64, u8) {
    let plus = lv.ends_with('+');
    (lv.trim_end_matches('+').parse().unwrap_or(0.0), u8::from(plus))
}

impl MaiData {
    pub fn new(songs: Vec<Song>, aliases: Vec<Alias>, plates: HashMap<String, Vec<u32>>) -> Self {
        let index = songs.iter().enumerate().map(|(i, s)| (s.song_id, i)).collect();
        let mut data = Self { songs, index, aliases, plates, level_data: Vec::new() };
        data.level_data = data.build_level_data();
        data
    }

    /// Loads the merged files written by the updater (or upstream).
    pub fn load_local() -> Self {
        fn read<T: serde::de::DeserializeOwned + Default>(name: &str) -> T {
            std::fs::read(CONFIG.data_file(name))
                .ok()
                .and_then(|b| serde_json::from_slice(&b).map_err(|e| log::warn!("解析 {name} 失败: {e}")).ok())
                .unwrap_or_default()
        }
        Self::new(read(MERGE_MUSIC_FILE), read(MERGE_ALIAS_FILE), read(PLATE_FILE))
    }

    fn build_level_data(&self) -> LevelData {
        let mut map: HashMap<String, Vec<(String, Vec<Chart>)>> = HashMap::new();
        for song in &self.songs {
            for d in &song.difficulties {
                if d.level.contains('?') || d.level_value < 7.0 {
                    continue;
                }
                let key = format!("{:.1}", d.level_value);
                let groups = map.entry(d.level.clone()).or_default();
                let chart = Chart {
                    song_id: song.song_id,
                    version_str: song.version_str.clone(),
                    kind: song.kind.clone(),
                    diff: d.clone(),
                };
                match groups.iter_mut().find(|(k, _)| *k == key) {
                    Some((_, list)) => list.push(chart),
                    None => groups.push((key, vec![chart])),
                }
            }
        }
        let mut out: LevelData = map.into_iter().collect();
        out.sort_by(|a, b| level_key(&b.0).partial_cmp(&level_key(&a.0)).unwrap_or(std::cmp::Ordering::Equal));
        for (_, groups) in &mut out {
            groups.sort_by(|a, b| {
                let (x, y): (f64, f64) = (a.0.parse().unwrap_or(0.0), b.0.parse().unwrap_or(0.0));
                y.partial_cmp(&x).unwrap_or(std::cmp::Ordering::Equal)
            });
        }
        out
    }

    pub fn by_id(&self, id: u32) -> Option<&Song> {
        self.index.get(&id).map(|&i| &self.songs[i])
    }

    pub fn by_name(&self, name: &str) -> Option<&Song> {
        self.songs.iter().find(|s| s.song_name == name)
    }

    /// Songs whose id is in `ids`, in song-list order.
    pub fn by_ids(&self, ids: &[u32]) -> Vec<&Song> {
        let set: std::collections::HashSet<u32> = ids.iter().copied().collect();
        self.songs.iter().filter(|s| set.contains(&s.song_id)).collect()
    }

    /// Charts of a level grouped per song (upstream `by_plan`).
    pub fn by_level(&self, level: &str) -> Vec<(u32, Vec<&Difficulty>)> {
        self.songs
            .iter()
            .filter_map(|s| {
                let diffs: Vec<&Difficulty> = s.difficulties.iter().filter(|d| d.level == level).collect();
                (!diffs.is_empty()).then_some((s.song_id, diffs))
            })
            .collect()
    }

    pub fn level_charts(&self, level: &str) -> Option<&Vec<(String, Vec<Chart>)>> {
        self.level_data.iter().find(|(l, _)| l == level).map(|(_, g)| g)
    }

    pub fn level_value(&self, song_id: u32, level_index: u8) -> f64 {
        self.diff(song_id, level_index).map_or(0.0, |d| d.level_value)
    }

    pub fn diff(&self, song_id: u32, level_index: u8) -> Option<&Difficulty> {
        self.by_id(song_id)?.difficulties.get(level_index as usize)
    }

    pub fn random(&self, rng: &mut Rng) -> Option<&Song> {
        rng.choice(&self.songs)
    }

    pub fn plate(&self, key: &str) -> &[u32] {
        self.plates.get(key).map_or(&[], Vec::as_slice)
    }

    pub fn alias_by_id(&self, id: u32) -> Vec<&Alias> {
        self.aliases.iter().filter(|a| a.song_id == id).collect()
    }

    pub fn alias_by_alias(&self, name: &str) -> Vec<&Alias> {
        self.aliases.iter().filter(|a| a.alias.iter().any(|x| x == name)).collect()
    }

    pub fn filter(&self, f: &Filter) -> Vec<Song> {
        let lower = |s: Option<&str>| s.map(str::to_lowercase);
        let (title, artist, charter) = (lower(f.title), lower(f.artist), lower(f.charter));
        let in_range = |r: Option<(f64, f64)>, v: f64| r.is_none_or(|(a, b)| a <= v && v <= b);
        let mut out = Vec::new();
        for song in &self.songs {
            if title.as_ref().is_some_and(|t| !song.song_name.to_lowercase().contains(t.as_str())) {
                continue;
            }
            if artist
                .as_ref()
                .is_some_and(|a| song.artist.is_empty() || !song.artist.to_lowercase().contains(a.as_str()))
            {
                continue;
            }
            if f.kind.is_some_and(|k| !k.contains(&song.kind.as_str())) {
                continue;
            }
            if f.version_str.is_some_and(|v| !v.contains(&song.version_str.as_str())) || !in_range(f.bpm, song.bpm) {
                continue;
            }
            let diffs: Vec<Difficulty> = song
                .difficulties
                .iter()
                .filter(|d| {
                    f.level.is_none_or(|l| l.contains(&d.level.as_str()))
                        && in_range(f.level_value, d.level_value)
                        && charter.as_ref().is_none_or(|c| {
                            !d.note_designer.is_empty() && d.note_designer.to_lowercase().contains(c.as_str())
                        })
                })
                .cloned()
                .collect();
            if !diffs.is_empty() {
                let mut s = song.clone();
                if f.matched_diffs_only {
                    s.difficulties = diffs;
                }
                out.push(s);
            }
        }
        out
    }
}

static DATA: LazyLock<RwLock<Arc<MaiData>>> = LazyLock::new(|| RwLock::new(Arc::new(MaiData::load_local())));

/// Current data snapshot; cheap to clone, never blocks writers for long.
pub fn data() -> Arc<MaiData> {
    DATA.read().map(|d| d.clone()).unwrap_or_default()
}

pub fn set_data(new: MaiData) {
    if let Ok(mut d) = DATA.write() {
        *d = Arc::new(new);
    }
}
