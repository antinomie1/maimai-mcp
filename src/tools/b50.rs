use crate::core::clients::divingfish::DivingFishClient;
use crate::core::clients::lxns::LxnsClient;
use crate::core::models::user::ScoreSource;
use crate::draw::b50::{B50Renderer, B50Theme};
use crate::tools::ToolHelper;
use rmcp::model::CallToolResult;
use schemars::JsonSchema;
use serde::Deserialize;

#[derive(Debug, Deserialize, JsonSchema)]
pub struct MaiB50Params {
    /// User QQ number (auto-extracted from context if available)
    pub qq: Option<String>,
    /// DivingFish username or LXNS user identifier (optional override)
    pub username: Option<String>,
    /// Score source: "divingfish" or "lxns" (defaults to user preference or divingfish)
    pub source: Option<String>,
    /// B50 theme: "circle" or "prism_plus" (defaults to user preference or circle)
    pub theme: Option<String>,
    /// Output format: "image" (default), "text", or "both"
    pub output_format: Option<String>,
}

pub struct B50Tool;

impl B50Tool {
    pub async fn execute(params: MaiB50Params) -> Result<CallToolResult, String> {
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

        let b50_result = match resolved_src {
            ScoreSource::DivingFish => {
                let df_client = DivingFishClient::new();
                if let Some(ref uname) = user_cfg.divingfish_username {
                    df_client.query_b50_by_username(uname).await
                } else if !user_cfg.qq.is_empty() {
                    df_client.query_b50_by_qq(&user_cfg.qq).await
                } else {
                    return Err("未指定 QQ 号或水鱼用户名，请提供 `qq` 参数或先绑定账号。".to_string());
                }
            }
            ScoreSource::Lxns => {
                let lxns_client = LxnsClient::new();
                let qq_opt = if user_cfg.qq.is_empty() { None } else { Some(user_cfg.qq.as_str()) };
                lxns_client.query_b50_smart(qq_opt, user_cfg.lxns_friend_code, user_cfg.lxns_token.as_deref()).await
            }
        };

        let player_b50 = b50_result.map_err(|e| format!("查询 B50 失败: {e}"))?;

        // Generate text summary
        let mut summary = format!(
            "【{} 的舞萌 DX B50 成绩】\nDX Rating: {} (旧曲 B35: {} + 新曲 B15: {})\n",
            player_b50.nickname, player_b50.rating, player_b50.b35_rating, player_b50.b15_rating
        );
        if let Some(ref plate) = player_b50.plate {
            summary.push_str(&format!("佩戴名牌: {plate}\n"));
        }
        summary.push_str("--- Top 3 单曲 ---\n");
        for (i, song) in player_b50.b35.iter().take(3).enumerate() {
            summary.push_str(&format!(
                "旧曲 {}. {} [{:.4}% -> Ra:{}]\n",
                i + 1, song.title, song.achievements, song.dx_rating
            ));
        }
        for (i, song) in player_b50.b15.iter().take(3).enumerate() {
            summary.push_str(&format!(
                "新曲 {}. {} [{:.4}% -> Ra:{}]\n",
                i + 1, song.title, song.achievements, song.dx_rating
            ));
        }

        // Render Image
        let b50_img = if params.output_format.as_deref() == Some("text") {
            None
        } else {
            match B50Renderer::render_b50_with_theme(&player_b50, selected_theme) {
                Ok(img) => Some(img),
                Err(e) => {
                    log::warn!("Failed to render B50 image: {e}");
                    None
                }
            }
        };

        Ok(ToolHelper::format_result(
            b50_img.as_ref(),
            "b50",
            &summary,
            params.output_format.as_deref(),
        ))
    }
}
