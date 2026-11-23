use crate::core::clients::divingfish::DivingFishClient;
use crate::core::database::user_store::UserStore;
use rmcp::model::{CallToolResult, ContentBlock};
use schemars::JsonSchema;
use serde::Deserialize;

#[derive(Debug, Deserialize, JsonSchema)]
pub struct MaiGroupRankParams {
    /// List of QQ numbers of group members to rank (optional, defaults to all known local users)
    pub qq_list: Option<Vec<String>>,
}

pub struct GroupRankTool;

impl GroupRankTool {
    pub async fn execute(params: MaiGroupRankParams) -> Result<CallToolResult, String> {
        let qq_candidates = params.qq_list.unwrap_or_default();
        if qq_candidates.is_empty() {
            return Ok(CallToolResult::success(vec![ContentBlock::text(
                "请提供群成员 QQ 列表以生成群内 DX Rating 排行榜。",
            )]));
        }

        let df = DivingFishClient::new();
        let mut ranks: Vec<(String, String, u32)> = Vec::new();

        for qq in qq_candidates {
            let ucfg = UserStore::get_user(&qq);
            if let Ok(b50) = if let Some(ref uname) = ucfg.divingfish_username {
                df.query_b50_by_username(uname).await
            } else {
                df.query_b50_by_qq(&qq).await
            } {
                ranks.push((qq, b50.nickname, b50.rating));
            }
        }

        if ranks.is_empty() {
            return Ok(CallToolResult::success(vec![ContentBlock::text(
                "未在水鱼查分网查找到指定成员的成绩数据。",
            )]));
        }

        // Sort descending by rating
        ranks.sort_by(|a, b| b.2.cmp(&a.2));

        let mut out = format!("🏆 【舞萌 DX 群内 Rating 排行榜】\n共统计 {} 位玩家：\n\n", ranks.len());
        for (i, (qq, nick, rating)) in ranks.iter().enumerate() {
            let medal = match i {
                0 => "🥇",
                1 => "🥈",
                2 => "🥉",
                _ => "▫️",
            };
            out.push_str(&format!("{} 第 {} 名: {} (QQ: {})\n   Rating: {}\n\n", medal, i + 1, nick, qq, rating));
        }

        Ok(CallToolResult::success(vec![ContentBlock::text(out)]))
    }
}
