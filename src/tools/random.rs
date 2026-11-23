use crate::core::database::music_db::MUSIC_DB;
use crate::core::models::music::{Song, SongType};
use crate::draw::b50::B50Theme;
use crate::draw::chart_info::ChartInfoRenderer;
use crate::tools::ToolHelper;
use rmcp::model::CallToolResult;
use schemars::JsonSchema;
use serde::Deserialize;
use std::time::{SystemTime, UNIX_EPOCH};

#[derive(Debug, Deserialize, JsonSchema)]
pub struct MaiRandomSongParams {
    /// Filter by difficulty level (e.g., "12+", "13", "13+", "14", "14+")
    pub level: Option<String>,
    /// Filter by song type: "SD" or "DX"
    pub song_type: Option<String>,
    /// Output format: "image" (default), "text", or "both"
    pub output_format: Option<String>,
}

#[derive(Debug, Deserialize, JsonSchema)]
pub struct MaiWhatParams {
    /// Output format: "image" (default), "text", or "both"
    pub output_format: Option<String>,
}

pub struct RandomTools;

impl RandomTools {
    fn get_random_seed() -> u64 {
        SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap_or_default()
            .as_nanos() as u64
    }

    pub async fn random_song(params: MaiRandomSongParams) -> Result<CallToolResult, String> {
        let db = MUSIC_DB.read().unwrap();
        let target_type = params.song_type.as_deref().map(|t| match t.to_uppercase().as_str() {
            "DX" => SongType::Deluxe,
            _ => SongType::Standard,
        });

        let mut filtered: Vec<&Song> = db.songs.iter().collect();

        if let Some(st) = target_type {
            filtered.retain(|s| s.song_type == st);
        }

        if let Some(ref lv) = params.level {
            let lv_clean = lv.trim();
            filtered.retain(|s| s.level.iter().any(|l| l == lv_clean));
        }

        if filtered.is_empty() {
            return Err("未找到符合条件的曲目，请放宽定数或类型筛选。".into());
        }

        let seed = Self::get_random_seed();
        let chosen = filtered[(seed as usize) % filtered.len()];

        let summary = format!(
            "🎲 为您随机抽取的曲目：\n【{} (ID: {})】\n类型: {:?} | 难度: {}\n曲师: {} | BPM: {}",
            chosen.title, chosen.id, chosen.song_type, chosen.level.join(" / "), chosen.basic_info.artist, chosen.basic_info.bpm
        );

        let img = if params.output_format.as_deref() == Some("text") {
            None
        } else {
            match ChartInfoRenderer::render_chart_info(chosen, B50Theme::Circle) {
                Ok(img) => Some(img),
                Err(e) => {
                    log::warn!("Failed to render random_song image: {e}");
                    None
                }
            }
        };

        Ok(ToolHelper::format_result(
            img.as_ref(),
            "random_song",
            &summary,
            params.output_format.as_deref(),
        ))
    }

    pub async fn mai_what(params: MaiWhatParams) -> Result<CallToolResult, String> {
        let db = MUSIC_DB.read().unwrap();
        if db.songs.is_empty() {
            return Err("曲目库为空。".into());
        }

        let seed = Self::get_random_seed();
        let chosen = &db.songs[(seed as usize) % db.songs.len()];

        let tips = [
            "今天来一首充满节奏感的挑战吧！",
            "打机时记得放松手腕，顺便收个 AP 怎么样？",
            "今日推荐：试试这张谱面的绝赞音符手感！",
            "劳逸结合，今天就决定打这首啦！",
            "干饭之前先来一局热身！",
        ];
        let tip = tips[(seed as usize / 17) % tips.len()];

        let summary = format!(
            "🍽️ 【今天舞萌吃什么 / 练什么】\n{}\n\n推荐曲目：{} (ID:{})\n难度：{}\n曲师：{}",
            tip, chosen.title, chosen.id, chosen.level.join(" / "), chosen.basic_info.artist
        );

        let img = if params.output_format.as_deref() == Some("text") {
            None
        } else {
            match ChartInfoRenderer::render_chart_info(chosen, B50Theme::Circle) {
                Ok(img) => Some(img),
                Err(e) => {
                    log::warn!("Failed to render mai_what image: {e}");
                    None
                }
            }
        };

        Ok(ToolHelper::format_result(
            img.as_ref(),
            "mai_what",
            &summary,
            params.output_format.as_deref(),
        ))
    }
}
