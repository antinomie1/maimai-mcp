use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum SongType {
    #[serde(rename = "SD")]
    Standard,
    #[serde(rename = "DX")]
    Deluxe,
    #[serde(rename = "UTAGE")]
    Utage,
}

impl Default for SongType {
    fn default() -> Self {
        Self::Standard
    }
}

/// Chart level and internal constant (定数) information for a difficulty
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct ChartInfo {
    #[serde(default)]
    pub notes: Vec<u32>,
    #[serde(default)]
    pub charter: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct BasicInfo {
    pub title: String,
    #[serde(default)]
    pub artist: String,
    #[serde(default)]
    pub genre: String,
    #[serde(default)]
    pub bpm: u32,
    #[serde(default, rename = "from")]
    pub version: String,
    #[serde(default)]
    pub is_new: bool,
}

/// Song definition loaded from static music database
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Song {
    pub id: String,
    pub title: String,
    #[serde(rename = "type")]
    pub song_type: SongType,
    #[serde(default)]
    pub ds: Vec<f64>,
    #[serde(default)]
    pub level: Vec<String>,
    #[serde(default)]
    pub cids: Vec<u32>,
    #[serde(default)]
    pub charts: Vec<ChartInfo>,
    #[serde(default)]
    pub basic_info: BasicInfo,
}

impl Song {
    /// Calculate the theoretical max DX score for a difficulty index.
    /// Notes format in DivingFish music_data: [tap, hold, slide, touch, break] or [tap, hold, slide, break]
    pub fn max_dx_score(&self, level_index: usize) -> u32 {
        if level_index >= self.charts.len() {
            return 0;
        }
        let notes = &self.charts[level_index].notes;
        let total_notes = if notes.len() == 4 {
            notes[0] + notes[1] + notes[2] + notes[3]
        } else if notes.len() >= 5 {
            notes[0] + notes[1] + notes[2] + notes[3] + notes[4]
        } else {
            0
        };

        // Each note max Critical Perfect is 3 points.
        total_notes * 3
    }
}
