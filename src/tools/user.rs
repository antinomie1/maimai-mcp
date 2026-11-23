use crate::core::clients::divingfish::DivingFishClient;
use crate::core::clients::lxns::LxnsClient;
use crate::core::database::user_store::UserStore;
use crate::core::models::user::ScoreSource;
use rmcp::model::{CallToolResult, ContentBlock};
use schemars::JsonSchema;
use serde::Deserialize;
use std::time::{SystemTime, UNIX_EPOCH};

#[derive(Debug, Deserialize, JsonSchema)]
pub struct MaiBindPlayerParams {
    /// User QQ number to bind
    pub qq: String,
    /// DivingFish username (optional)
    pub divingfish_username: Option<String>,
    /// LXNS developer/user personal token (optional)
    pub lxns_token: Option<String>,
    /// LXNS Friend Code / 好友码 (optional)
    pub lxns_friend_code: Option<u64>,
    /// LXNS OAuth authorization code / 授权码 (e.g. "JVJ6-VPTM-MGHZ") (optional)
    pub lxns_code: Option<String>,
    /// Default查分源: "divingfish" or "lxns"
    pub default_source: Option<String>,
    /// Default UI theme: "circle" or "prism_plus"
    pub theme: Option<String>,
    /// Request DivingFish OAuth device authorization link (optional)
    pub request_oauth_link: Option<bool>,
}

#[derive(Debug, Deserialize, JsonSchema)]
pub struct MaiUserSettingsParams {
    /// User QQ number to view settings
    pub qq: String,
}

pub struct UserTools;

impl UserTools {
    pub async fn bind_player(params: MaiBindPlayerParams) -> Result<CallToolResult, String> {
        let qq = params.qq.trim();
        if qq.is_empty() {
            return Err("请提供有效的 QQ 号进行绑定。".into());
        }

        let mut config = UserStore::get_user(qq);
        let mut notes = Vec::new();

        if let Some(uname) = params.divingfish_username {
            if !uname.trim().is_empty() {
                config.divingfish_username = Some(uname.trim().to_string());
            }
        }

        if let Some(tok) = params.lxns_token {
            if !tok.trim().is_empty() {
                config.lxns_token = Some(tok.trim().to_string());
            }
        }

        if let Some(fc) = params.lxns_friend_code {
            if fc > 0 {
                config.lxns_friend_code = Some(fc);
            }
        }

        // Handle LXNS OAuth code exchange
        if let Some(code) = params.lxns_code {
            let code_clean = code.trim();
            if !code_clean.is_empty() {
                let lx = LxnsClient::new();
                match lx.exchange_code(code_clean).await {
                    Ok(tok_resp) => {
                        let now_sec = SystemTime::now()
                            .duration_since(UNIX_EPOCH)
                            .unwrap_or_default()
                            .as_secs();
                        config.lxns_access_token = Some(tok_resp.access_token.clone());
                        config.lxns_refresh_token = Some(tok_resp.refresh_token.clone());
                        config.lxns_token_expires_at = Some(now_sec + tok_resp.expires_in);

                        // Try fetching player info to get friend code & name
                        if let Ok(player) = lx.query_player_oauth(&tok_resp.access_token).await {
                            if let Some(fc) = player.friend_code {
                                config.lxns_friend_code = Some(fc);
                            }
                            notes.push(format!("✨ 落雪 OAuth 授权成功！玩家名: {}, 好友码: {:?}", player.name, player.friend_code));
                        } else {
                            notes.push("✨ 落雪 OAuth 授权成功！".to_string());
                        }
                    }
                    Err(e) => {
                        return Err(format!("落雪授权码兑换失败: {e}"));
                    }
                }
            }
        }

        if let Some(src) = params.default_source {
            config.default_source = ScoreSource::parse(&src);
        }

        if let Some(th) = params.theme {
            let th_clean = th.trim().to_lowercase();
            if th_clean == "prism_plus" || th_clean == "circle" {
                config.theme = th_clean;
            }
        }

        UserStore::save_user(&config).map_err(|e| format!("保存用户配置失败: {e}"))?;

        let mut out = format!(
            "✅ 用户 QQ: {} 绑定配置已更新！\n• 默认查分源: {:?}\n• 水鱼用户名: {}\n• 落雪好友码: {}\n• 落雪 OAuth 状态: {}\n• 界面主题: {}",
            config.qq,
            config.default_source,
            config.divingfish_username.as_deref().unwrap_or("未设置 (将使用QQ直查/OAuth)"),
            config.lxns_friend_code.map(|f| f.to_string()).unwrap_or_else(|| "未设置".into()),
            if config.lxns_access_token.is_some() { "已授权已绑定" } else { "未授权" },
            config.theme
        );

        if !notes.is_empty() {
            out.push_str("\n\n");
            out.push_str(&notes.join("\n"));
        }

        // If DivingFish OAuth is configured and user requested, provide authorization link
        if DivingFishClient::is_oauth_configured()
            && (params.request_oauth_link.unwrap_or(false) || config.divingfish_username.is_none())
        {
            let df = DivingFishClient::new();
            if let Ok(dev) = df.start_device_binding(qq, None).await {
                out.push_str(&format!(
                    "\n\n🔗 【水鱼账号 OAuth 授权】\n请在浏览器中打开以下链接完成一次水鱼账号绑定（10分钟内有效）：\n👉 {}\n绑定完成后直接查分即可生效。",
                    dev.verification_uri_complete
                ));
            }
        }

        // If LXNS OAuth is configured and user hasn't authorized, provide authorization link
        if LxnsClient::is_oauth_configured() && config.lxns_access_token.is_none() {
            if let Ok(auth_url) = LxnsClient::get_authorize_url(Some(qq)) {
                out.push_str(&format!(
                    "\n\n🔗 【落雪账号 OAuth 授权】\n若需使用落雪查分器，请在浏览器中打开链接完成授权：\n👉 {}\n完成授权后获取授权码（格式 XXXX-XXXX-XXXX），再次发送包含 `lxns_code` 的绑定指令即可完成授权绑定。",
                    auth_url
                ));
            }
        }

        Ok(CallToolResult::success(vec![ContentBlock::text(out)]))
    }

    pub async fn user_settings(params: MaiUserSettingsParams) -> Result<CallToolResult, String> {
        let qq = params.qq.trim();
        if qq.is_empty() {
            return Err("请提供有效的 QQ 号。".into());
        }

        let config = UserStore::get_user(qq);
        let out = format!(
            "👤 【用户偏好设置】\n• QQ 号: {}\n• 默认查分源: {:?}\n• 水鱼用户名: {}\n• 落雪好友码: {}\n• 落雪 OAuth: {}\n• 当前主题: {}",
            config.qq,
            config.default_source,
            config.divingfish_username.as_deref().unwrap_or("未设置"),
            config.lxns_friend_code.map(|f| f.to_string()).unwrap_or_else(|| "未设置".into()),
            if config.lxns_access_token.is_some() { "已授权" } else { "未授权" },
            config.theme
        );

        Ok(CallToolResult::success(vec![ContentBlock::text(out)]))
    }
}
