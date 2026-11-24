//! Domain models. `Song`/`Alias` serialize exactly like upstream's merged
//! `merge_music_data.json` / `merge_music_alias.json`.

use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct Notes {
    #[serde(default)]
    pub total: u32,
    #[serde(default)]
    pub tap: u32,
    #[serde(default)]
    pub hold: u32,
    #[serde(default)]
    pub slide: u32,
    #[serde(default)]
    pub touch: u32,
    #[serde(default, alias = "break")]
    pub brk: u32,
}

impl Notes {
    /// `[total, tap, hold, slide, touch, brk]` (upstream `NOTE_FIELDS`).
    pub fn fields(&self) -> [u32; 6] {
        [self.total, self.tap, self.hold, self.slide, self.touch, self.brk]
    }
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(default)]
pub struct Stats {
    pub cnt: f64,
    pub diff: String,
    pub fit_diff: f64,
    pub avg: f64,
    pub avg_dx: f64,
    pub std_dev: f64,
    pub dist: Vec<f64>,
    pub fc_dist: Vec<f64>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Difficulty {
    pub level_index: u8,
    pub level: String,
    pub level_value: f64,
    #[serde(default)]
    pub note_designer: String,
    #[serde(default)]
    pub notes: Notes,
    #[serde(default)]
    pub dx_score: u32,
    #[serde(default)]
    pub stats: Option<Stats>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Song {
    pub song_id: u32,
    pub song_name: String,
    #[serde(default)]
    pub artist: String,
    #[serde(default)]
    pub genre: String,
    #[serde(default)]
    pub bpm: f64,
    #[serde(default)]
    pub version_str: String,
    #[serde(default)]
    pub version_int: u32,
    /// `"SD"` or `"DX"`.
    #[serde(rename = "type")]
    pub kind: String,
    #[serde(default)]
    pub isnew: bool,
    #[serde(default)]
    pub difficulties: Vec<Difficulty>,
    #[serde(default)]
    pub kanji: Option<String>,
    #[serde(default)]
    pub description: Option<String>,
    #[serde(default)]
    pub is_buddy: Option<bool>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Alias {
    pub song_id: u32,
    #[serde(default)]
    pub song_name: String,
    #[serde(default)]
    pub alias: Vec<String>,
}

/// A played chart (upstream `PlayedResult`). `rate`/`fc`/`fs` are lowercase API values.
#[derive(Debug, Clone, Default)]
pub struct Played {
    pub song_id: u32,
    pub song_name: String,
    pub level: String,
    pub level_index: u8,
    pub level_value: f64,
    pub kind: String,
    pub rating: u32,
    pub achievements: f64,
    pub rate: Option<String>,
    pub fc: Option<String>,
    pub fs: Option<String>,
    pub dx_score: u32,
}

/// A chart slot that may not have been played (upstream `NotPlayedResult`).
#[derive(Debug, Clone)]
pub struct NotPlayed {
    pub song_id: u32,
    pub level_index: u8,
    pub level_value: f64,
}

#[derive(Debug, Clone)]
pub enum Slot {
    Played(Played),
    NotPlayed(NotPlayed),
}

#[derive(Debug, Clone)]
pub struct Rise {
    pub song_id: u32,
    pub song_name: String,
    pub level_index: u8,
    pub kind: String,
    pub rating: u32,
    pub achievements: f64,
    pub rate: String,
    pub level_value: f64,
    pub old_rating: u32,
    pub old_achievements: f64,
    pub old_rate: String,
}

#[derive(Debug, Clone, Default)]
pub enum Plate {
    #[default]
    None,
    /// LXNS collection id, fetched from the asset server.
    Id(u32),
    /// DivingFish plate name, `plate_version/<name>.png`.
    Name(String),
}

#[derive(Debug, Clone)]
pub struct Trophy {
    pub name: String,
    pub color: String,
}

#[derive(Debug, Clone, Default)]
pub struct Player {
    pub name: String,
    pub rating: u32,
    pub course_rank: u32,
    pub class_rank: u32,
    pub friend_code: u64,
    pub plate: Plate,
    pub icon: Option<u32>,
    pub trophy: Option<Trophy>,
}

#[derive(Debug, Clone, Default)]
pub struct Best50 {
    pub sd_total: u32,
    pub dx_total: u32,
    pub sd: Vec<Played>,
    pub dx: Vec<Played>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
pub enum Service {
    #[default]
    #[serde(rename = "divingfish", alias = "Diving-Fish")]
    DivingFish,
    #[serde(rename = "lxns", alias = "Lxns-Network")]
    Lxns,
}

impl Service {
    pub fn value(self) -> &'static str {
        match self {
            Self::DivingFish => "Diving-Fish",
            Self::Lxns => "Lxns-Network",
        }
    }

    pub fn parse(s: &str) -> Option<Self> {
        match s.trim().to_lowercase().as_str() {
            "0" | "divingfish" | "diving-fish" | "df" | "水鱼" => Some(Self::DivingFish),
            "1" | "lxns" | "lxns-network" | "lx" | "落雪" => Some(Self::Lxns),
            _ => None,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default, Serialize, Deserialize)]
pub enum Theme {
    #[default]
    #[serde(rename = "prism_plus")]
    PrismPlus,
    #[serde(rename = "circle")]
    Circle,
}

impl Theme {
    pub fn dir(self) -> &'static str {
        match self {
            Self::PrismPlus => "prism_plus",
            Self::Circle => "circle",
        }
    }

    pub fn parse(s: &str) -> Option<Self> {
        match s.trim().to_lowercase().as_str() {
            "0" | "prism_plus" | "prism" | "prismplus" => Some(Self::PrismPlus),
            "1" | "circle" => Some(Self::Circle),
            _ => None,
        }
    }

    /// Theme accent text colour.
    pub fn color(self) -> [u8; 4] {
        match self {
            Self::Circle => [249, 62, 172, 255],
            Self::PrismPlus => [124, 129, 255, 255],
        }
    }
}

/// Persisted per-QQ user configuration.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct User {
    pub qq: String,
    #[serde(default, alias = "default_source")]
    pub service: Service,
    #[serde(default)]
    pub theme: Theme,
    #[serde(default)]
    pub lxns_friend_code: Option<u64>,
    #[serde(default)]
    pub lxns_access_token: Option<String>,
    #[serde(default)]
    pub lxns_refresh_token: Option<String>,
}

impl User {
    pub fn qq_num(&self) -> u64 {
        self.qq.parse().unwrap_or(0)
    }
}
