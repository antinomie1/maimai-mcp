use crate::config::CONFIG;
use crate::core::clients::divingfish::DivingFishClient;
use crate::core::clients::lxns::LxnsClient;
use crate::core::models::user::ScoreSource;
use crate::draw::plate_table::PlateTableRenderer;
use crate::draw::rating_table::RatingTableRenderer;
use crate::tools::ToolHelper;
use rmcp::model::CallToolResult;
use schemars::JsonSchema;
use serde::Deserialize;
use std::collections::HashMap;
use std::fs;

#[derive(Debug, Deserialize, JsonSchema)]
pub struct MaiLevelProgressParams {
    /// Level rating category to query (e.g. "13", "13+", "14", "14+", "15")
    pub level: String,
    /// User QQ number (optional override)
    pub qq: Option<String>,
    /// DivingFish username or LXNS user identifier
    pub username: Option<String>,
    /// Score source: "divingfish" or "lxns"
    pub source: Option<String>,
    /// Set true to check FC / AP plan completion instead of achievement ranks
    pub plan: Option<bool>,
    /// Output format: "image" (default), "text", or "both"
    pub output_format: Option<String>,
}

#[derive(Debug, Deserialize, JsonSchema)]
pub struct MaiPlateTableParams {
    /// Game version plate name (e.g. "真", "超", "檄", "橙", "晓", "桃", "樱", "紫", "堇", "白", "雪", "辉", "熊", "华", "爽", "煌", "宙", "星", "祭", "祝", "双", "宴", "镜", "彩", "舞", "霸")
    pub version: String,
    /// Target plate type (e.g. "将", "极", "神", "舞舞", "者")
    pub plan: String,
    /// User QQ number (optional override)
    pub qq: Option<String>,
    /// DivingFish username or LXNS user identifier
    pub username: Option<String>,
    /// Score source: "divingfish" or "lxns"
    pub source: Option<String>,
    /// Output format: "image" (default), "text", or "both"
    pub output_format: Option<String>,
}

pub struct ProgressTools;

impl ProgressTools {
    pub async fn level_progress(params: MaiLevelProgressParams) -> Result<CallToolResult, String> {
        let (user_cfg, resolved_src, _) = ToolHelper::resolve_user_config(
            params.qq.as_deref(),
            params.username.as_deref(),
            params.source.as_deref(),
        );

        let is_fc = params.plan.unwrap_or(false);
        let level_clean = params.level.trim();

        // Fetch scores
        let qq_opt = if user_cfg.qq.is_empty() { None } else { Some(user_cfg.qq.as_str()) };
        let scores = match resolved_src {
            ScoreSource::DivingFish => {
                let df = DivingFishClient::new();
                df.query_all_records(qq_opt, user_cfg.divingfish_username.as_deref())
                    .await
                    .unwrap_or_default()
            }
            ScoreSource::Lxns => {
                let lxns = LxnsClient::new();
                lxns.query_all_scores_smart(qq_opt, user_cfg.lxns_friend_code)
                    .await
                    .unwrap_or_default()
            }
        };

        let summary = format!(
            "【Level.{} 定数完成度总览】\n展示该难度全部谱面的完成/评级/FC进度图。",
            level_clean
        );

        let img = if params.output_format.as_deref() == Some("text") {
            None
        } else {
            match RatingTableRenderer::render(level_clean, &scores, is_fc) {
                Ok(img) => Some(img),
                Err(e) => {
                    log::warn!("Failed to render level_progress image: {e}");
                    None
                }
            }
        };

        Ok(ToolHelper::format_result(
            img.as_ref(),
            "level_progress",
            &summary,
            params.output_format.as_deref(),
        ))
    }

    pub async fn plate_table(params: MaiPlateTableParams) -> Result<CallToolResult, String> {
        let (user_cfg, resolved_src, _) = ToolHelper::resolve_user_config(
            params.qq.as_deref(),
            params.username.as_deref(),
            params.source.as_deref(),
        );

        let ver = params.version.trim();
        let plan = params.plan.trim();

        // Load plate data from static/data/plate_data.json or fallback to MUSIC_DB
        let plate_file = CONFIG.static_dir.join("data/plate_data.json");
        let mut song_ids: Vec<u32> = Vec::new();
        if plate_file.exists() {
            if let Ok(content) = fs::read_to_string(&plate_file) {
                if let Ok(map) = serde_json::from_str::<HashMap<String, Vec<u32>>>(&content) {
                    if let Some(list) = map.get(ver) {
                        song_ids = list.clone();
                    }
                }
            }
        }
        if song_ids.is_empty() {
            let db = crate::core::database::music_db::MUSIC_DB.read().unwrap();
            song_ids = db.get_songs_by_plate(ver);
        }

        // Fetch scores
        let qq_opt = if user_cfg.qq.is_empty() { None } else { Some(user_cfg.qq.as_str()) };
        let scores = match resolved_src {
            ScoreSource::DivingFish => {
                let df = DivingFishClient::new();
                df.query_all_records(qq_opt, user_cfg.divingfish_username.as_deref())
                    .await
                    .unwrap_or_default()
            }
            ScoreSource::Lxns => {
                let lxns = LxnsClient::new();
                lxns.query_all_scores_smart(qq_opt, user_cfg.lxns_friend_code)
                    .await
                    .unwrap_or_default()
            }
        };

        let summary = format!(
            "【「{ver}{plan}」牌子达成进度表】\n共收录 {} 首关联曲目，正在展示达成进度。",
            song_ids.len()
        );

        let img = if params.output_format.as_deref() == Some("text") {
            None
        } else {
            match PlateTableRenderer::render_plate_table(ver, plan, &song_ids, &scores) {
                Ok(img) => Some(img),
                Err(e) => {
                    log::warn!("Failed to render plate_table image: {e}");
                    None
                }
            }
        };

        Ok(ToolHelper::format_result(
            img.as_ref(),
            "plate_table",
            &summary,
            params.output_format.as_deref(),
        ))
    }
}
