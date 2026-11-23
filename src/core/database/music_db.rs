use crate::config::CONFIG;
use crate::core::models::music::Song;
use crate::error::AppError;
use std::collections::HashMap;
use std::fs;
use std::sync::{LazyLock, RwLock};

pub struct MusicDatabase {
    pub songs: Vec<Song>,
    pub id_map: HashMap<String, usize>,
    pub title_map: HashMap<String, Vec<usize>>,
}

impl MusicDatabase {
    pub fn load() -> Result<Self, AppError> {
        let path = CONFIG.static_dir.join("data/music_data.json");
        let fallback_path = CONFIG.static_dir.join("data/merge_music_data.json");

        let content = if path.exists() {
            fs::read_to_string(&path)?
        } else if fallback_path.exists() {
            fs::read_to_string(&fallback_path)?
        } else {
            log::warn!("Music data file not found at {:?}, using empty DB", path);
            "[]".to_string()
        };

        let songs: Vec<Song> = serde_json::from_str(&content).unwrap_or_default();
        let mut id_map = HashMap::with_capacity(songs.len());
        let mut title_map: HashMap<String, Vec<usize>> = HashMap::with_capacity(songs.len());

        for (idx, song) in songs.iter().enumerate() {
            id_map.insert(song.id.clone(), idx);
            // Also index % 10000 normalized id
            if let Ok(num_id) = song.id.parse::<u32>() {
                id_map.insert((num_id % 10000).to_string(), idx);
                id_map.insert(num_id.to_string(), idx);
            }
            title_map
                .entry(song.title.to_lowercase())
                .or_default()
                .push(idx);
        }

        log::info!("Loaded {} songs into MusicDatabase", songs.len());
        Ok(Self {
            songs,
            id_map,
            title_map,
        })
    }

    pub fn get_by_id(&self, id: &str) -> Option<&Song> {
        self.id_map.get(id).map(|&idx| &self.songs[idx])
    }

    pub fn get_songs_by_plate(&self, plate_ver: &str) -> Vec<u32> {
        let versions: Vec<&str> = match plate_ver {
            "初" => vec!["maimai"],
            "真" => vec!["maimai", "maimai PLUS"],
            "超" => vec!["maimai GreeN"],
            "檄" => vec!["maimai GreeN PLUS"],
            "橙" => vec!["maimai ORANGE"],
            "晓" | "暁" => vec!["maimai ORANGE PLUS"],
            "桃" => vec!["maimai PiNK"],
            "樱" | "櫻" => vec!["maimai PiNK PLUS"],
            "紫" => vec!["maimai MURASAKi"],
            "堇" | "菫" => vec!["maimai MURASAKi PLUS"],
            "白" => vec!["maimai MiLK"],
            "雪" => vec!["MiLK PLUS"],
            "辉" | "輝" => vec!["maimai FiNALE"],
            "舞" | "霸" | "霸者" => vec![
                "maimai", "maimai PLUS", "maimai GreeN", "maimai GreeN PLUS",
                "maimai ORANGE", "maimai ORANGE PLUS", "maimai PiNK", "maimai PiNK PLUS",
                "maimai MURASAKi", "maimai MURASAKi PLUS", "maimai MiLK", "MiLK PLUS",
                "maimai FiNALE",
            ],
            "熊" => vec!["maimai でらっくす"],
            "华" | "華" => vec!["maimai でらっくす PLUS"],
            "熊&华" => vec!["maimai でらっくす", "maimai でらっくす PLUS"],
            "爽" => vec!["maimai でらっくす Splash"],
            "煌" => vec!["maimai でらっくす Splash PLUS"],
            "爽&煌" => vec!["maimai でらっくす Splash", "maimai でらっくす Splash PLUS"],
            "宙" => vec!["maimai でらっくす UNiVERSE"],
            "星" => vec!["maimai でらっくす UNiVERSE PLUS"],
            "宙&星" => vec!["maimai でらっくす UNiVERSE", "maimai でらっくす UNiVERSE PLUS"],
            "祭" => vec!["maimai でらっくす FESTiVAL"],
            "祝" => vec!["maimai でらっくす FESTiVAL PLUS"],
            "祭&祝" => vec!["maimai でらっくす FESTiVAL", "maimai でらっくす FESTiVAL PLUS"],
            "双" => vec!["maimai でらっくす BUDDiES"],
            "宴" => vec!["maimai でらっくす BUDDiES PLUS"],
            "双&宴" => vec!["maimai でらっくす BUDDiES", "maimai でらっくす BUDDiES PLUS"],
            "镜" => vec!["maimai でらっくす PRiSM"],
            "彩" => vec!["maimai でらっくす PRiSM PLUS"],
            _ => vec![],
        };

        let mut ids = Vec::new();
        for song in &self.songs {
            if song.song_type == crate::core::models::music::SongType::Utage {
                continue;
            }
            if let Ok(id_num) = song.id.parse::<u32>() {
                if id_num >= 100000 {
                    continue;
                }
                if versions.contains(&song.basic_info.version.as_str()) {
                    ids.push(id_num);
                }
            }
        }
        ids.sort();
        ids.dedup();
        ids
    }

    pub fn get_max_dx_score(&self, song_id: u32, level_index: usize) -> u32 {
        let id_str = song_id.to_string();
        let norm_id = (song_id % 10000).to_string();
        if let Some(song) = self.get_by_id(&id_str).or_else(|| self.get_by_id(&norm_id)) {
            song.max_dx_score(level_index)
        } else {
            0
        }
    }

    pub fn search_by_title(&self, query: &str) -> Vec<&Song> {
        let q = query.trim().to_lowercase();
        let mut results = Vec::new();

        // Exact match
        if let Some(indices) = self.title_map.get(&q) {
            for &idx in indices {
                results.push(&self.songs[idx]);
            }
            return results;
        }

        // Substring match
        for song in &self.songs {
            if song.title.to_lowercase().contains(&q) || song.basic_info.artist.to_lowercase().contains(&q) {
                results.push(song);
            }
        }
        results
    }

    /// Finds a single song by query (ID, title, artist, or alias fallback).
    /// If original name / ID is not found, automatically searches alias database.
    pub fn find_song(&self, query: &str) -> Option<Song> {
        self.find_songs(query).into_iter().next()
    }

    /// Finds all matching songs by query (ID, title, artist, or alias fallback).
    /// If original name / ID yields no matches, automatically falls back to alias database.
    pub fn find_songs(&self, query: &str) -> Vec<Song> {
        let q = query.trim();
        if q.is_empty() {
            return Vec::new();
        }

        // 1. Direct ID match
        if let Some(song) = self.get_by_id(q) {
            return vec![song.clone()];
        }

        // 2. Exact Title match
        let q_lower = q.to_lowercase();
        if let Some(indices) = self.title_map.get(&q_lower) {
            let mut exact_matches = Vec::new();
            for &idx in indices {
                exact_matches.push(self.songs[idx].clone());
            }
            if !exact_matches.is_empty() {
                return exact_matches;
            }
        }

        // 3. Substring match on Title or Artist
        let mut sub_matches = Vec::new();
        for song in &self.songs {
            if song.song_type == crate::core::models::music::SongType::Utage {
                continue;
            }
            let t_lower = song.title.to_lowercase();
            let a_lower = song.basic_info.artist.to_lowercase();
            if t_lower.contains(&q_lower) || a_lower.contains(&q_lower) {
                sub_matches.push(song.clone());
            }
        }

        if !sub_matches.is_empty() {
            return sub_matches;
        }

        // 4. Automatic Alias Fallback: search AliasDatabase
        let alias_db = crate::core::database::alias_db::ALIAS_DB.read().unwrap();
        let alias_ids = alias_db.find_song_ids_by_alias(q);
        let mut alias_matches = Vec::new();
        for sid in alias_ids {
            if let Some(song) = self.get_by_id(&sid.to_string()) {
                if !alias_matches.iter().any(|s: &Song| s.id == song.id) {
                    alias_matches.push(song.clone());
                }
            }
        }

        if !alias_matches.is_empty() {
            return alias_matches;
        }

        // 5. Fuzzy match on title as last resort
        let mut fuzzy_matches: Vec<(f64, Song)> = Vec::new();
        for song in &self.songs {
            if song.song_type == crate::core::models::music::SongType::Utage {
                continue;
            }
            let sim = strsim::jaro_winkler(&song.title.to_lowercase(), &q_lower);
            if sim > 0.82 {
                fuzzy_matches.push((sim, song.clone()));
            }
        }
        fuzzy_matches.sort_by(|a, b| b.0.partial_cmp(&a.0).unwrap_or(std::cmp::Ordering::Equal));
        fuzzy_matches.into_iter().map(|(_, s)| s).collect()
    }
}

pub static MUSIC_DB: LazyLock<RwLock<MusicDatabase>> = LazyLock::new(|| {
    RwLock::new(MusicDatabase::load().unwrap_or_else(|_| MusicDatabase {
        songs: Vec::new(),
        id_map: HashMap::new(),
        title_map: HashMap::new(),
    }))
});
