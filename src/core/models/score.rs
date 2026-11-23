use super::music::SongType;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize, Default)]
pub enum ComboStatus {
    #[default]
    None,
    FC,
    FCP,
    AP,
    APP,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize, Default)]
pub enum SyncStatus {
    #[default]
    None,
    Sync,
    FS,
    FSP,
    FDX,
    FDXP,
}

/// Unified Single Play Score Record
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ScoreRecord {
    pub song_id: u32,
    pub title: String,
    pub level_index: u8, // 0: Basic, 1: Advanced, 2: Expert, 3: Master, 4: Re:Master
    pub song_type: SongType,
    pub ds: f64,
    pub achievements: f64,
    pub dx_score: u32,
    pub dx_rating: u32,
    pub rate: String, // "sssp", "sss", "ssp", etc.
    #[serde(default)]
    pub fc: ComboStatus,
    #[serde(default)]
    pub fs: SyncStatus,
    #[serde(default)]
    pub is_new: bool,
}

/// Player Profile and B50 Result
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PlayerB50 {
    pub nickname: String,
    pub rating: u32,
    pub b35: Vec<ScoreRecord>,
    pub b15: Vec<ScoreRecord>,
    pub b35_rating: u32,
    pub b15_rating: u32,
    #[serde(default)]
    pub plate: Option<String>,
    #[serde(default)]
    pub course_rank: u32, // Dani / additional_rating
    #[serde(default)]
    pub class_rank: u32,
}
