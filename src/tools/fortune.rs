use crate::core::database::music_db::MUSIC_DB;
use crate::draw::b50::B50Theme;
use crate::draw::chart_info::ChartInfoRenderer;
use crate::tools::ToolHelper;
use rmcp::model::CallToolResult;
use schemars::JsonSchema;
use serde::Deserialize;
use std::time::{SystemTime, UNIX_EPOCH};

#[derive(Debug, Deserialize, JsonSchema)]
pub struct MaiFortuneParams {
    /// User QQ number (optional, used for deterministic daily lucky hash)
    pub qq: Option<String>,
    /// Output format: "image" (default), "text", or "both"
    pub output_format: Option<String>,
}

pub struct FortuneTool;

impl FortuneTool {
    fn hash_daily(qq: &str) -> u64 {
        let days_since_epoch = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap_or_default()
            .as_secs()
            / 86400;

        let mut hash: u64 = 14695981039346656037;
        for byte in qq.as_bytes() {
            hash = (hash ^ (*byte as u64)).wrapping_mul(1099511628211);
        }
        hash = (hash ^ days_since_epoch).wrapping_mul(1099511628211);
        hash
    }

    pub async fn execute(params: MaiFortuneParams) -> Result<CallToolResult, String> {
        let qq = params.qq.as_deref().unwrap_or("0");
        let hash = Self::hash_daily(qq);
        let rp = (hash % 100) as u32;

        let fortune_items = [
            "拼机", "推分", "越级", "下埋", "夜勤",
            "练底力", "练手法", "打旧框", "干饭", "抓绝赞", "收歌",
        ];

        let mut h = hash;
        let mut yi_list = Vec::new();
        let mut ji_list = Vec::new();

        for item in &fortune_items {
            let val = h & 3;
            h >>= 2;
            if val == 3 {
                yi_list.push(*item);
            } else if val == 0 {
                ji_list.push(*item);
            }
        }

        let db = MUSIC_DB.read().unwrap();
        if db.songs.is_empty() {
            return Err("曲目库为空。".into());
        }

        let chosen_song = &db.songs[(hash as usize) % db.songs.len()];

        let mut summary = format!("🎋 【今日舞萌运势】\n今日人品值 (Lucky): {} / 100\n", rp);
        if !yi_list.is_empty() {
            summary.push_str(&format!("✨ 宜: {}\n", yi_list.join("、")));
        }
        if !ji_list.is_empty() {
            summary.push_str(&format!("⚠️ 忌: {}\n", ji_list.join("、")));
        }
        summary.push_str(&format!(
            "\n今日幸运曲目推荐：\n[{}] {} ({})\n难度: {}",
            chosen_song.id, chosen_song.title, chosen_song.basic_info.artist, chosen_song.level.join(" / ")
        ));

        let img = if params.output_format.as_deref() == Some("text") {
            None
        } else {
            match ChartInfoRenderer::render_chart_info(chosen_song, B50Theme::Circle) {
                Ok(img) => Some(img),
                Err(e) => {
                    log::warn!("Failed to render fortune image: {e}");
                    None
                }
            }
        };

        Ok(ToolHelper::format_result(
            img.as_ref(),
            "fortune",
            &summary,
            params.output_format.as_deref(),
        ))
    }
}
