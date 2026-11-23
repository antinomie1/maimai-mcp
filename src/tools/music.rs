use crate::core::database::alias_db::ALIAS_DB;
use crate::core::database::music_db::MUSIC_DB;
use crate::draw::b50::B50Theme;
use crate::draw::chart_info::ChartInfoRenderer;
use crate::tools::ToolHelper;
use rmcp::model::{CallToolResult, ContentBlock};
use schemars::JsonSchema;
use serde::Deserialize;

#[derive(Debug, Deserialize, JsonSchema)]
pub struct MaiChartInfoParams {
    /// Song name, alias, or ID to query chart details
    pub song_name: String,
    /// Theme: "circle" or "prism_plus" (default: "circle")
    pub theme: Option<String>,
    /// Output format: "image" (default), "text", or "both"
    pub output_format: Option<String>,
}

#[derive(Debug, Deserialize, JsonSchema)]
pub struct MaiSearchSongParams {
    /// Search query (song name, artist, genre, version, or alias)
    pub keyword: String,
    /// Maximum number of search results to return (default: 10)
    pub limit: Option<usize>,
}

#[derive(Debug, Deserialize, JsonSchema)]
pub struct MaiAliasQueryParams {
    /// Song ID, exact title, or alias to look up
    pub song_name: String,
}

pub struct MusicTools;

impl MusicTools {
    pub async fn chart_info(params: MaiChartInfoParams) -> Result<CallToolResult, String> {
        let query = params.song_name.trim();
        let target_song = MUSIC_DB.read().unwrap().find_song(query);
        let song = target_song.ok_or_else(|| format!("未找到曲目: `{query}`（已尝试原名、曲师及别名自动匹配）"))?;

        let theme = match params.theme.as_deref() {
            Some("prism_plus") => B50Theme::PrismPlus,
            _ => B50Theme::Circle,
        };

        // Text summary
        let mut summary = format!(
            "【{} (ID: {})】\n曲师: {}\nBPM: {} | 流派: {}\n版本: {}\n--- 各难度定数与物量 ---\n",
            song.title, song.id, song.basic_info.artist, song.basic_info.bpm, song.basic_info.genre, song.basic_info.version
        );

        let diff_names = ["Basic", "Advanced", "Expert", "Master", "Re:Master"];
        for (i, lv) in song.level.iter().enumerate() {
            let diff_label = diff_names.get(i).copied().unwrap_or("Diff");
            let ds = song.ds.get(i).copied().unwrap_or(0.0);
            let charter = song.charts.get(i).and_then(|c| c.charter.as_deref()).unwrap_or("-");
            summary.push_str(&format!("{diff_label}: {lv} (ds: {ds:.1}) | 谱师: {charter}\n"));
        }

        let img = if params.output_format.as_deref() == Some("text") {
            None
        } else {
            match ChartInfoRenderer::render_chart_info(&song, theme) {
                Ok(img) => Some(img),
                Err(e) => {
                    log::warn!("Failed to render chart_info image: {e}");
                    None
                }
            }
        };

        Ok(ToolHelper::format_result(
            img.as_ref(),
            "chart_info",
            &summary,
            params.output_format.as_deref(),
        ))
    }

    pub async fn search_song(params: MaiSearchSongParams) -> Result<CallToolResult, String> {
        let q = params.keyword.trim();
        let limit = params.limit.unwrap_or(10).clamp(1, 30);
        let results = MUSIC_DB.read().unwrap().find_songs(q);

        if results.is_empty() {
            return Ok(CallToolResult::success(vec![ContentBlock::text(format!(
                "未找到与 `{q}` 匹配的舞萌曲目（已尝试原名、曲师及别名匹配）。"
            ))]));
        }

        let mut out = format!("🔍 为您找到 {} 首匹配曲目（展示前 {} 条）：\n\n", results.len(), results.len().min(limit));
        for (i, song) in results.iter().take(limit).enumerate() {
            let levels = song.level.join(" / ");
            out.push_str(&format!(
                "{}. [{}] {} ({:?})\n   难度: {}\n   曲师: {}\n\n",
                i + 1, song.id, song.title, song.song_type, levels, song.basic_info.artist
            ));
        }

        Ok(CallToolResult::success(vec![ContentBlock::text(out)]))
    }

    pub async fn alias_query(params: MaiAliasQueryParams) -> Result<CallToolResult, String> {
        let q = params.song_name.trim();
        let alias_db = ALIAS_DB.read().unwrap();
        let db = MUSIC_DB.read().unwrap();

        // 1. If q is a number/ID
        if let Ok(num_id) = q.parse::<u32>() {
            let aliases = alias_db.get_aliases_for_song(num_id).cloned().unwrap_or_default();
            let song_title = db.get_by_id(q).map(|s| s.title.as_str()).unwrap_or("未知曲名");
            if aliases.is_empty() {
                return Ok(CallToolResult::success(vec![ContentBlock::text(format!(
                    "曲目 ID: {} ({}) 暂无收录的别名。", num_id, song_title
                ))]));
            }
            let out = format!(
                "【曲目别名库】\nID: {} - {}\n已知别名 ({}个):\n• {}",
                num_id, song_title, aliases.len(), aliases.join("\n• ")
            );
            return Ok(CallToolResult::success(vec![ContentBlock::text(out)]));
        }

        // 2. If q matches a song title directly, get that song's aliases
        if let Some(song) = db.search_by_title(q).first() {
            if let Ok(sid) = song.id.parse::<u32>() {
                if let Some(aliases) = alias_db.get_aliases_for_song(sid) {
                    let out = format!(
                        "【曲目别名库】\nID: {} - {}\n已知别名 ({}个):\n• {}",
                        sid, song.title, aliases.len(), aliases.join("\n• ")
                    );
                    return Ok(CallToolResult::success(vec![ContentBlock::text(out)]));
                }
            }
        }

        // 3. Query as an alias
        let song_ids = alias_db.find_song_ids_by_alias(q);
        if song_ids.is_empty() {
            return Ok(CallToolResult::success(vec![ContentBlock::text(format!(
                "未找到包含别名 `{q}` 的曲目。"
            ))]));
        }

        let mut out = format!("🔍 别名 `{q}` 对应的曲目：\n\n");
        for sid in song_ids {
            let song_title = db.get_by_id(&sid.to_string()).map(|s| s.title.as_str()).unwrap_or("未知");
            let all_aliases = alias_db.get_aliases_for_song(sid).cloned().unwrap_or_default();
            out.push_str(&format!(
                "• [ID:{sid}] {song_title}\n  全部别名: {}\n\n",
                all_aliases.join(", ")
            ));
        }

        Ok(CallToolResult::success(vec![ContentBlock::text(out)]))
    }
}
