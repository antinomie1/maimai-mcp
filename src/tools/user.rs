use super::{Reply, require_qq};
use crate::config::CONFIG;
use crate::core::clients::divingfish::{self, REVOKE_URL, binding_label, extract_confirmation_code};
use crate::core::clients::lxns::{self, Lxns, extract_authorization_code};
use crate::core::models::{Service, Theme};
use crate::core::store;
use crate::core::update::{self, Target};
use crate::error::{AppError, Result};
use schemars::JsonSchema;
use serde::Deserialize;
use std::time::Duration;

const DF_SESSION_TTL: Duration = Duration::from_secs(20 * 60);
const LXNS_ERROR: &str = "BOT管理员尚未配置落雪查分器相关信息";

#[derive(Debug, Deserialize, JsonSchema)]
pub struct BindDfParams {
    /// User QQ number
    pub qq: Option<String>,
    /// Confirmation code shown after authorizing (e.g. BCDF-GHJK-LMNP). Omit to get the authorization link.
    pub code: Option<String>,
}

/// 绑定水鱼 / 水鱼确认码 (OAuth, `handoff=code`).
pub async fn bind_divingfish(p: BindDfParams) -> Result<Reply> {
    let qq = require_qq(p.qq.as_deref())?;
    store::get_user(qq)?;
    if !CONFIG.df_oauth_enabled() {
        return Ok(Reply::Text("BOT管理员尚未配置水鱼查分器 OAuth 应用，无法进行绑定授权。".into()));
    }
    if let Some(text) = p.code.as_deref().map(str::trim).filter(|t| !t.is_empty()) {
        if !store::pending_is_active(qq, Service::DivingFish) {
            return Ok(Reply::Text("请先发送「绑定水鱼」获取授权链接，完成授权后再发送确认码。".into()));
        }
        let Some(code) = extract_confirmation_code(text) else {
            return Ok(Reply::Text(
                "未识别到有效的水鱼确认码。\n请发送授权完成页面显示的完整确认码，形如 BCDF-GHJK-LMNP。".into(),
            ));
        };
        return Ok(Reply::Text(match divingfish::complete_binding(qq, &code).await {
            Ok(()) => {
                store::pending_clear(qq);
                "水鱼查分器授权完成，现在可以直接使用查询指令了。".into()
            }
            Err(AppError::DfBindingMismatch) => "水鱼绑定失败：这串确认码对应的授权不属于您的账号。\n确认码只能由发起绑定的本人使用，请勿使用他人转发给您的确认码。\n如需绑定自己的账号，请发送「绑定水鱼」重新走一遍授权。".into(),
            Err(AppError::DfConfirmationCode) => "水鱼绑定失败：确认码可能已使用、已过期，或不是本次绑定的确认码。\n当前绑定会话仍有效，您可以发送新的确认码；如需重新授权，请再次发送「绑定水鱼」。".into(),
            Err(e) => {
                log::warn!("水鱼绑定暂时失败：{e}");
                "水鱼绑定暂时失败：水鱼账号服务或网络出现异常。\n当前绑定会话仍有效，您可以稍后重新发送确认码；如果确认码已经使用，请再次发送「绑定水鱼」重新授权。".into()
            }
        }));
    }
    let auth = match divingfish::device_authorization(qq).await {
        Ok(a) => a,
        Err(e) => {
            log::warn!("水鱼授权发起失败：{e}");
            return Ok(Reply::Text("发起水鱼授权失败：水鱼账号服务可能暂时不可用，请稍后再试。".into()));
        }
    };
    store::pending_start(qq, Service::DivingFish, DF_SESSION_TTL);
    Ok(Reply::Text(format!(
        "请完成水鱼查分器授权：\n\n1. 打开以下链接并登录水鱼账号，授权「{bot} BOT」访问您的水鱼查分器数据\n=======================\n{url}\n=======================\n2. 确认页面显示的绑定身份为「{label}」后点击「同意授权」\n3. 复制页面给出的确认码，回到 QQ 发送给 BOT\n\n本次绑定 {minutes} 分钟内有效，确认码只能使用一次；\n超时或失效后请重新发送「绑定水鱼」。\n=======================\n请注意！！链接与确认码都仅供您本人使用，请勿转发他人。\n确认码建议在与 BOT 的私聊中发送，避免被他人看到。\n如需取消授权，请前往 {revoke}",
        bot = CONFIG.bot_name,
        url = auth.verification_uri_complete,
        label = binding_label(qq),
        minutes = (auth.expires_in / 60).max(1),
        revoke = CONFIG.df_auth_url.clone() + REVOKE_URL,
    )))
}

#[derive(Debug, Deserialize, JsonSchema)]
pub struct BindLxParams {
    /// User QQ number
    pub qq: Option<String>,
    /// Authorization code or full callback URL. Omit to get the authorization link.
    pub code: Option<String>,
}

/// lxbind / 落雪授权码 (OAuth authorization code).
pub async fn bind_lxns(p: BindLxParams) -> Result<Reply> {
    let qq = require_qq(p.qq.as_deref())?;
    store::get_user(qq)?;
    if !CONFIG.lxns_oauth_enabled() {
        return Ok(Reply::Text(format!("{LXNS_ERROR}，无法进行绑定授权。")));
    }
    let Some(text) = p.code.as_deref().map(str::trim).filter(|t| !t.is_empty()) else {
        store::pending_start(qq, Service::Lxns, Duration::from_secs(600));
        return Ok(Reply::Text(format!(
            "请完成落雪查分器授权：\n\n1. 打开以下链接并允许「{} BOT」访问您的落雪查分器数据\n=======================\n{}\n=======================\n2. 授权完成后，复制页面显示的授权码\n3. 回到 QQ，直接发送授权码或完整回调链接\n\n本次绑定有效期为 10 分钟，授权码只能使用一次；\n超时或失效后请重新发送「lxbind」获取授权链接\n=======================\n请注意！！您必须在落雪查分器的\n「账号设置 -> 常规设置」中的\n「隐私设置」开启允许读取成绩，否\n则BOT将无法查询您的成绩",
            CONFIG.bot_name,
            lxns::authorize_url()
        )));
    };
    let Some(code) = extract_authorization_code(text) else {
        return Ok(Reply::Text(
            "未识别到有效的落雪授权码。\n请发送授权页面显示的完整授权码，或直接粘贴完整回调链接。".into(),
        ));
    };
    let result = async {
        let token = lxns::exchange_code(&code).await?;
        let mut user = store::get_user(qq)?;
        user.lxns_access_token = Some(token.access_token);
        user.lxns_refresh_token = Some(token.refresh_token);
        let player = Lxns::for_user(&user)?.player().await?;
        store::update_user(qq, |u| {
            u.lxns_friend_code = Some(player.friend_code);
            u.lxns_access_token = user.lxns_access_token.clone();
            u.lxns_refresh_token = user.lxns_refresh_token.clone();
        })
    }
    .await;
    Ok(Reply::Text(match result {
        Ok(_) => {
            store::pending_clear(qq);
            "授权完成。".into()
        }
        Err(AppError::Network(e)) | Err(AppError::Other(e)) => {
            log::warn!("落雪 OAuth 绑定暂时失败：{e}");
            "落雪绑定暂时失败：网络、响应数据或本地数据库出现异常。\n当前绑定会话仍有效，您可以稍后重新发送授权码；如果授权码已经使用，请再次发送「lxbind」重新授权。".into()
        }
        Err(_) => "落雪绑定失败：授权码可能已使用、已过期，或授权未成功。\n当前绑定会话仍有效，您可以发送新的授权码；如需重新授权，请再次发送「lxbind」。".into(),
    }))
}

#[derive(Debug, Deserialize, JsonSchema)]
pub struct SettingsParams {
    /// User QQ number
    pub qq: Option<String>,
    /// Switch score source (数据源): "divingfish" (0) or "lxns" (1)
    pub source: Option<String>,
    /// Switch theme (主题): "prism_plus" (0) or "circle" (1)
    pub theme: Option<String>,
}

/// 数据源 / 主题, or view current settings.
pub async fn user_settings(p: SettingsParams) -> Result<Reply> {
    let qq = require_qq(p.qq.as_deref())?;
    let mut msgs = Vec::new();
    if let Some(s) = p.source.as_deref().filter(|s| !s.trim().is_empty()) {
        let Some(mut source) = Service::parse(s) else {
            return Ok(Reply::Text(
                "未找到该数据源，请输入指定数字切换：\n「0」：Diving-Fish\n「1」：Lxns-Network".into(),
            ));
        };
        if source == Service::Lxns && CONFIG.lxns_dev_token.is_none() && !CONFIG.lxns_oauth_enabled() {
            source = Service::DivingFish;
            msgs.push(format!("{LXNS_ERROR}。为防止无法查询成绩，已强制将数据源切换为水鱼查分器。"));
        } else {
            msgs.push(format!("已切换数据源为：「{}」", source.value()));
        }
        store::update_user(qq, |u| u.service = source)?;
    }
    if let Some(t) = p.theme.as_deref().filter(|s| !s.trim().is_empty()) {
        let Some(theme) = Theme::parse(t) else {
            return Ok(Reply::Text("未找到该主题，请输入指定数字切换：\n「0」：prism_plus\n「1」：circle".into()));
        };
        store::update_user(qq, |u| u.theme = theme)?;
        msgs.push(format!("已切换主题为：「{}」", theme.dir()));
    }
    let u = store::get_user(qq)?;
    msgs.push(format!(
        "QQ：{}\n数据源：{}\n主题：{}\n落雪授权：{}",
        u.qq,
        u.service.value(),
        u.theme.dir(),
        if u.lxns_access_token.is_some() { "已绑定" } else { "未绑定" }
    ));
    Ok(Reply::Text(msgs.join("\n")))
}

#[derive(Debug, Deserialize, JsonSchema)]
pub struct UpdateParams {
    /// "all" (default), "music", "alias", "plate" or "tables" (re-render rating/plate tables)
    pub target: Option<String>,
}

/// 更新maimai数据 / 更新别名库 / 更新定数表 / 更新完成表.
pub async fn update_data(p: UpdateParams) -> Result<Reply> {
    let target = match p.target.as_deref().map(str::to_lowercase).as_deref() {
        None | Some("" | "all") => Target::All,
        Some("music") => Target::Music,
        Some("alias") => Target::Alias,
        Some("plate") => Target::Plate,
        Some("tables" | "table") => Target::Tables,
        Some(t) => return Err(AppError::Msg(format!("未知的更新目标「{t}」"))),
    };
    Ok(Reply::Text(update::update(target).await))
}
