use crate::core::calc::rating::{compute_single_rating, get_rate_string};
use crate::core::clients::divingfish::DivingFishClient;
use crate::core::clients::lxns::LxnsClient;
use crate::core::database::music_db::MUSIC_DB;
use crate::core::models::score::{PlayerB50, ScoreRecord};
use crate::core::models::user::ScoreSource;
use crate::draw::b50::B50Theme;
use crate::draw::level_progress::{LevelProgressRenderer, RiseRecommendation};
use crate::draw::play_score::PlayScoreRenderer;
use crate::tools::ToolHelper;
use rmcp::model::CallToolResult;
use schemars::JsonSchema;
use serde::Deserialize;

#[derive(Debug, Deserialize, JsonSchema)]
pub struct MaiPlayScoreParams {
    /// Song name, alias, or song ID to query
    pub song_name: String,
    /// User QQ number (optional override)
    pub qq: Option<String>,
    /// DivingFish username or LXNS user identifier
    pub username: Option<String>,
    /// Score source: "divingfish" or "lxns"
    pub source: Option<String>,
    /// Theme: "circle" or "prism_plus"
    pub theme: Option<String>,
    /// Output format: "image" (default), "text", or "both"
    pub output_format: Option<String>,
}

#[derive(Debug, Deserialize, JsonSchema)]
pub struct MaiRiseScoreParams {
    /// User QQ number (optional override)
    pub qq: Option<String>,
    /// DivingFish username or LXNS user identifier
    pub username: Option<String>,
    /// Score source: "divingfish" or "lxns"
    pub source: Option<String>,
    /// Output format: "image" (default), "text", or "both"
    pub output_format: Option<String>,
}

pub struct ScoreTools;

impl ScoreTools {
    pub async fn play_score(params: MaiPlayScoreParams) -> Result<CallToolResult, String> {
        let (user_cfg, resolved_src, default_theme) = ToolHelper::resolve_user_config(
            params.qq.as_deref(),
            params.username.as_deref(),
            params.source.as_deref(),
        );

        let selected_theme = match params.theme.as_deref() {
            Some("prism_plus") => B50Theme::PrismPlus,
            Some("circle") => B50Theme::Circle,
            _ => default_theme,
        };

        // 1. Locate song
        let query = params.song_name.trim();
        let target_song = MUSIC_DB.read().unwrap().find_song(query);
        let song = target_song.ok_or_else(|| format!("未找到曲目: `{query}`，请检查曲名、别名或 ID。"))?;
        let song_id_num: u32 = song.id.parse().unwrap_or(0);

        // 2. Fetch player scores
        let mut play_results: Vec<Option<ScoreRecord>> = vec![None; song.ds.len().min(5)];

        let b50_opt: Option<PlayerB50> = match resolved_src {
            ScoreSource::DivingFish => {
                let df = DivingFishClient::new();
                if let Some(ref uname) = user_cfg.divingfish_username {
                    df.query_b50_by_username(uname).await.ok()
                } else if !user_cfg.qq.is_empty() {
                    df.query_b50_by_qq(&user_cfg.qq).await.ok()
                } else {
                    None
                }
            }
            ScoreSource::Lxns => {
                let lxns = LxnsClient::new();
                let qq_opt = if user_cfg.qq.is_empty() { None } else { Some(user_cfg.qq.as_str()) };
                lxns.query_b50_smart(qq_opt, user_cfg.lxns_friend_code, user_cfg.lxns_token.as_deref()).await.ok()
            }
        };

        if let Some(b50) = b50_opt {
            for rec in b50.b35.into_iter().chain(b50.b15.into_iter()) {
                if rec.song_id == song_id_num || rec.song_id % 10000 == song_id_num % 10000 {
                    let idx = rec.level_index as usize;
                    if idx < play_results.len() {
                        play_results[idx] = Some(rec);
                    }
                }
            }
        }

        // Summary text
        let mut summary = format!("【{} (ID:{}) 游玩成绩】\n", song.title, song.id);
        let diff_names = ["Basic", "Advanced", "Expert", "Master", "Re:Master"];
        for (i, opt) in play_results.iter().enumerate() {
            let diff_label = diff_names.get(i).copied().unwrap_or("Diff");
            let ds = song.ds.get(i).copied().unwrap_or(0.0);
            if let Some(rec) = opt {
                summary.push_str(&format!(
                    "{} (Lv.{:.1}): {:.4}% [{}] -> Ra:{}\n",
                    diff_label, ds, rec.achievements, rec.rate, rec.dx_rating
                ));
            } else {
                summary.push_str(&format!("{} (Lv.{:.1}): 未游玩 / 未在B50中\n", diff_label, ds));
            }
        }

        let img = if params.output_format.as_deref() == Some("text") {
            None
        } else {
            match PlayScoreRenderer::render_song_play_data(&song, &play_results, selected_theme) {
                Ok(img) => Some(img),
                Err(e) => {
                    log::warn!("Failed to render play_score image: {e}");
                    None
                }
            }
        };

        Ok(ToolHelper::format_result(
            img.as_ref(),
            "play_score",
            &summary,
            params.output_format.as_deref(),
        ))
    }

    pub async fn rise_score(params: MaiRiseScoreParams) -> Result<CallToolResult, String> {
        let (user_cfg, resolved_src, _) = ToolHelper::resolve_user_config(
            params.qq.as_deref(),
            params.username.as_deref(),
            params.source.as_deref(),
        );

        let df = DivingFishClient::new();
        let b50 = match resolved_src {
            ScoreSource::DivingFish => {
                if let Some(ref uname) = user_cfg.divingfish_username {
                    df.query_b50_by_username(uname).await
                } else if !user_cfg.qq.is_empty() {
                    df.query_b50_by_qq(&user_cfg.qq).await
                } else {
                    return Err("未指定 QQ 或用户名，无法分析当前成绩。".into());
                }
            }
            ScoreSource::Lxns => {
                let lxns = LxnsClient::new();
                let qq_opt = if user_cfg.qq.is_empty() { None } else { Some(user_cfg.qq.as_str()) };
                lxns.query_b50_smart(qq_opt, user_cfg.lxns_friend_code, user_cfg.lxns_token.as_deref()).await
            }
        }.map_err(|e| format!("获取成绩失败: {e}"))?;

        let sd_floor = b50.b35.last().map(|s| s.dx_rating).unwrap_or(200);
        let dx_floor = b50.b15.last().map(|s| s.dx_rating).unwrap_or(200);

        // Generate recommendations
        let mut sd_rises = Vec::new();
        for s in b50.b35.iter().rev().take(5) {
            let target_ach = (s.achievements + 0.5).min(100.5);
            let target_ra = compute_single_rating(s.ds, target_ach);
            let rate_str = get_rate_string(target_ach).to_uppercase();
            sd_rises.push(RiseRecommendation {
                song_id: s.song_id,
                song_name: s.title.clone(),
                level_index: s.level_index as usize,
                level_value: s.ds,
                song_type: s.song_type,
                old_achievements: s.achievements,
                old_rating: s.dx_rating,
                target_achievements: target_ach,
                target_rating: target_ra,
                rate_str,
            });
        }

        let mut dx_rises = Vec::new();
        for s in b50.b15.iter().rev().take(5) {
            let target_ach = (s.achievements + 0.5).min(100.5);
            let target_ra = compute_single_rating(s.ds, target_ach);
            let rate_str = get_rate_string(target_ach).to_uppercase();
            dx_rises.push(RiseRecommendation {
                song_id: s.song_id,
                song_name: s.title.clone(),
                level_index: s.level_index as usize,
                level_value: s.ds,
                song_type: s.song_type,
                old_achievements: s.achievements,
                old_rating: s.dx_rating,
                target_achievements: target_ach,
                target_rating: target_ra,
                rate_str,
            });
        }

        let summary = format!(
            "【上分建议分析】\n当前 DX Rating: {}\n旧曲底分 (B35 floor): {} | 新曲底分 (B15 floor): {}\n建议提升低达成率曲目至 SSS+ (100.5%) 获取显著提分。",
            b50.rating, sd_floor, dx_floor
        );

        let img = if params.output_format.as_deref() == Some("text") {
            None
        } else {
            match LevelProgressRenderer::render_rise_score(&sd_rises, &dx_rises) {
                Ok(img) => Some(img),
                Err(e) => {
                    log::warn!("Failed to render rise_score image: {e}");
                    None
                }
            }
        };

        Ok(ToolHelper::format_result(
            img.as_ref(),
            "rise_score",
            &summary,
            params.output_format.as_deref(),
        ))
    }
}
