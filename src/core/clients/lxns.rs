use crate::config::CONFIG;
use crate::core::database::music_db::MUSIC_DB;
use crate::core::database::user_store::UserStore;
use crate::core::models::music::SongType;
use crate::core::models::score::{ComboStatus, PlayerB50, ScoreRecord, SyncStatus};
use crate::error::AppError;
use reqwest::Client;
use serde::{Deserialize, Serialize};
use std::time::{Duration, SystemTime, UNIX_EPOCH};

#[derive(Debug, Deserialize, Serialize, Clone)]
pub struct LxnsOAuthToken {
    pub access_token: String,
    pub token_type: String,
    pub expires_in: u64,
    pub refresh_token: String,
    pub scope: Option<String>,
    pub id_token: Option<String>,
}

#[derive(Debug, Deserialize, Clone)]
#[allow(dead_code)]
pub struct LxnsScore {
    pub id: u32,
    pub song_name: Option<String>,
    pub level: Option<String>,
    pub level_index: u8,
    pub achievements: f64,
    pub fc: Option<String>,
    pub fs: Option<String>,
    pub dx_score: Option<u32>,
    pub dx_rating: Option<u32>,
    pub rate: Option<String>,
    #[serde(rename = "type")]
    pub song_type: Option<String>,
    pub play_time: Option<String>,
}

#[derive(Debug, Deserialize, Clone)]
pub struct LxnsNamePlate {
    pub id: Option<u32>,
}

#[derive(Debug, Deserialize, Clone)]
pub struct LxnsPlayer {
    pub name: String,
    pub rating: u32,
    pub friend_code: Option<u64>,
    pub course_rank: Option<u32>,
    pub class_rank: Option<u32>,
    pub star: Option<u32>,
    pub name_plate: Option<LxnsNamePlate>,
}

#[derive(Debug, Deserialize)]
pub struct LxnsB50Data {
    pub standard_total: Option<u32>,
    pub dx_total: Option<u32>,
    pub standard: Option<Vec<LxnsScore>>,
    pub dx: Option<Vec<LxnsScore>>,
}

#[derive(Debug, Deserialize)]
pub struct LxnsResponse<T> {
    pub code: i32,
    pub data: Option<T>,
    pub message: Option<String>,
}

pub struct LxnsClient {
    client: Client,
}

impl LxnsClient {
    pub fn new() -> Self {
        let client = Client::builder()
            .timeout(Duration::from_secs(15))
            .build()
            .unwrap_or_default();
        Self { client }
    }

    /// Check if LXNS OAuth credentials are configured
    pub fn is_oauth_configured() -> bool {
        CONFIG.lxns_client_id.is_some() && CONFIG.lxns_client_secret.is_some()
    }

    /// Generate OAuth authorization link for users to visit
    pub fn get_authorize_url(state: Option<&str>) -> Result<String, AppError> {
        let client_id = CONFIG.lxns_client_id.as_deref().ok_or_else(|| {
            AppError::InvalidInput("落雪 OAuth client_id 未配置 (LXNS_CLIENT_ID)".into())
        })?;

        let redirect_uri = CONFIG
            .lxns_redirect_uri
            .as_deref()
            .unwrap_or("https://maimai.lxns.net/oauth/redirect");

        let scope = "read_player+read_user_profile+write_player";
        let mut url = format!(
            "https://maimai.lxns.net/oauth/authorize?response_type=code&client_id={client_id}&redirect_uri={redirect_uri}&scope={scope}"
        );

        if let Some(s) = state {
            url.push_str(&format!("&state={s}"));
        }

        Ok(url)
    }

    /// Exchange authorization code for Access and Refresh tokens
    pub async fn exchange_code(&self, code: &str) -> Result<LxnsOAuthToken, AppError> {
        let client_id = CONFIG.lxns_client_id.as_deref().ok_or_else(|| {
            AppError::InvalidInput("落雪 OAuth client_id 未配置".into())
        })?;
        let client_secret = CONFIG.lxns_client_secret.as_deref().ok_or_else(|| {
            AppError::InvalidInput("落雪 OAuth client_secret 未配置".into())
        })?;
        let redirect_uri = CONFIG
            .lxns_redirect_uri
            .as_deref()
            .unwrap_or("https://maimai.lxns.net/oauth/redirect");

        let url = "https://maimai.lxns.net/api/v0/oauth/token";
        let resp = self
            .client
            .post(url)
            .json(&serde_json::json!({
                "client_id": client_id,
                "client_secret": client_secret,
                "grant_type": "authorization_code",
                "code": code.trim(),
                "redirect_uri": redirect_uri,
            }))
            .send()
            .await?;

        if !resp.status().is_success() {
            let status = resp.status();
            let text = resp.text().await.unwrap_or_default();
            return Err(AppError::Network(format!(
                "落雪 OAuth 授权码兑换失败 (HTTP {status}): {text}"
            )));
        }

        let body: LxnsOAuthToken = resp.json().await?;
        Ok(body)
    }

    /// Refresh Access Token using Refresh Token
    pub async fn refresh_access_token(&self, refresh_token: &str) -> Result<LxnsOAuthToken, AppError> {
        let client_id = CONFIG.lxns_client_id.as_deref().ok_or_else(|| {
            AppError::InvalidInput("落雪 OAuth client_id 未配置".into())
        })?;
        let client_secret = CONFIG.lxns_client_secret.as_deref().ok_or_else(|| {
            AppError::InvalidInput("落雪 OAuth client_secret 未配置".into())
        })?;

        let url = "https://maimai.lxns.net/api/v0/oauth/token";
        let resp = self
            .client
            .post(url)
            .json(&serde_json::json!({
                "client_id": client_id,
                "client_secret": client_secret,
                "grant_type": "refresh_token",
                "refresh_token": refresh_token.trim(),
            }))
            .send()
            .await?;

        if !resp.status().is_success() {
            let status = resp.status();
            let text = resp.text().await.unwrap_or_default();
            return Err(AppError::Network(format!(
                "落雪 OAuth 令牌刷新失败 (HTTP {status}): {text}"
            )));
        }

        let body: LxnsOAuthToken = resp.json().await?;
        Ok(body)
    }

    /// Get valid access token for a bound QQ user, automatically refreshing if needed
    pub async fn get_valid_token_for_user(&self, qq: &str) -> Result<String, AppError> {
        let mut user = UserStore::get_user(qq);
        let now_sec = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap_or_default()
            .as_secs();

        if let Some(access_tok) = &user.lxns_access_token {
            let expires_at = user.lxns_token_expires_at.unwrap_or(0);
            if expires_at > now_sec + 60 {
                return Ok(access_tok.clone());
            }
        }

        // Try refreshing if refresh token exists
        if let Some(refresh_tok) = &user.lxns_refresh_token {
            match self.refresh_access_token(refresh_tok).await {
                Ok(new_token) => {
                    let new_expires_at = now_sec + new_token.expires_in;
                    user.lxns_access_token = Some(new_token.access_token.clone());
                    user.lxns_refresh_token = Some(new_token.refresh_token.clone());
                    user.lxns_token_expires_at = Some(new_expires_at);
                    let _ = UserStore::save_user(&user);
                    return Ok(new_token.access_token);
                }
                Err(e) => {
                    log::warn!("Failed to refresh LXNS token for QQ {qq}: {e}");
                }
            }
        }

        Err(AppError::NotFound(
            "该 QQ 用户尚未完成落雪 OAuth 授权绑定或令牌已失效".into(),
        ))
    }

    pub fn map_combo(fc: Option<&str>) -> ComboStatus {
        match fc {
            Some("fc") => ComboStatus::FC,
            Some("fcp") => ComboStatus::FCP,
            Some("ap") => ComboStatus::AP,
            Some("app") => ComboStatus::APP,
            _ => ComboStatus::None,
        }
    }

    pub fn map_sync(fs: Option<&str>) -> SyncStatus {
        match fs {
            Some("fs") => SyncStatus::FS,
            Some("fsp") => SyncStatus::FSP,
            Some("fsd") | Some("fdx") => SyncStatus::FDX,
            Some("fsdp") | Some("fdxp") => SyncStatus::FDXP,
            Some("sync") => SyncStatus::Sync,
            _ => SyncStatus::None,
        }
    }

    pub fn convert_score(s: LxnsScore, is_new: bool) -> ScoreRecord {
        let song_type = match s.song_type.as_deref() {
            Some("dx") | Some("DX") => SongType::Deluxe,
            Some("utage") | Some("UTAGE") => SongType::Utage,
            _ => SongType::Standard,
        };

        // Lookup ds from catalog if missing or zero
        let song_id_str = s.id.to_string();
        let ds = {
            let music_db = MUSIC_DB.read().unwrap();
            if let Some(song) = music_db.get_by_id(&song_id_str) {
                if let Some(ds_val) = song.ds.get(s.level_index as usize) {
                    *ds_val
                } else {
                    0.0
                }
            } else {
                0.0
            }
        };

        let rating = s.dx_rating.unwrap_or_else(|| {
            if ds > 0.0 {
                crate::core::calc::rating::compute_single_rating(ds, s.achievements)
            } else {
                0
            }
        });

        let rate = s.rate.unwrap_or_else(|| {
            crate::core::calc::rating::get_rate_string(s.achievements).to_string()
        });

        ScoreRecord {
            song_id: s.id,
            title: s.song_name.unwrap_or_default(),
            level_index: s.level_index,
            song_type,
            ds,
            achievements: s.achievements,
            dx_score: s.dx_score.unwrap_or(0),
            dx_rating: rating,
            rate,
            fc: Self::map_combo(s.fc.as_deref()),
            fs: Self::map_sync(s.fs.as_deref()),
            is_new,
        }
    }

    // ==========================================
    // OAuth User Endpoints (/api/v0/user/maimai/player)
    // ==========================================

    /// Query player profile via OAuth Bearer token
    pub async fn query_player_oauth(&self, access_token: &str) -> Result<LxnsPlayer, AppError> {
        let url = "https://maimai.lxns.net/api/v0/user/maimai/player";
        let resp = self
            .client
            .get(url)
            .header("Authorization", format!("Bearer {access_token}"))
            .send()
            .await?;

        if !resp.status().is_success() {
            let status = resp.status();
            let text = resp.text().await.unwrap_or_default();
            return Err(AppError::NotFound(format!(
                "落雪获取玩家信息失败 (HTTP {status}): {text}"
            )));
        }

        let body: LxnsResponse<LxnsPlayer> = resp.json().await?;
        body.data.ok_or_else(|| {
            AppError::NotFound(body.message.unwrap_or_else(|| "未找到落雪玩家数据".into()))
        })
    }

    /// Query player B50 via OAuth Bearer token
    pub async fn query_b50_oauth(&self, access_token: &str) -> Result<PlayerB50, AppError> {
        let player_info = self.query_player_oauth(access_token).await.ok();

        let url = "https://maimai.lxns.net/api/v0/user/maimai/player/bests";
        let resp = self
            .client
            .get(url)
            .header("Authorization", format!("Bearer {access_token}"))
            .send()
            .await?;

        if !resp.status().is_success() {
            let status = resp.status();
            let text = resp.text().await.unwrap_or_default();
            return Err(AppError::NotFound(format!(
                "落雪获取 B50 失败 (HTTP {status}): {text}"
            )));
        }

        let body: LxnsResponse<LxnsB50Data> = resp.json().await?;
        let data = body.data.ok_or_else(|| {
            AppError::NotFound(body.message.unwrap_or_else(|| "未找到落雪 B50 数据".into()))
        })?;

        let b35: Vec<ScoreRecord> = data
            .standard
            .unwrap_or_default()
            .into_iter()
            .map(|s| Self::convert_score(s, false))
            .collect();

        let b15: Vec<ScoreRecord> = data
            .dx
            .unwrap_or_default()
            .into_iter()
            .map(|s| Self::convert_score(s, true))
            .collect();

        let b35_rating: u32 = b35.iter().map(|s| s.dx_rating).sum();
        let b15_rating: u32 = b15.iter().map(|s| s.dx_rating).sum();
        let total_rating = player_info
            .as_ref()
            .map(|p| p.rating)
            .unwrap_or(b35_rating + b15_rating);
        let nickname = player_info
            .as_ref()
            .map(|p| p.name.clone())
            .unwrap_or_else(|| "LXNS Player".into());

        Ok(PlayerB50 {
            nickname,
            rating: total_rating,
            b35,
            b15,
            b35_rating,
            b15_rating,
            plate: None,
            course_rank: player_info.as_ref().and_then(|p| p.course_rank).unwrap_or(0),
            class_rank: player_info.as_ref().and_then(|p| p.class_rank).unwrap_or(0),
        })
    }

    /// Query player all scores via OAuth Bearer token
    pub async fn query_scores_oauth(&self, access_token: &str) -> Result<Vec<ScoreRecord>, AppError> {
        let url = "https://maimai.lxns.net/api/v0/user/maimai/player/scores";
        let resp = self
            .client
            .get(url)
            .header("Authorization", format!("Bearer {access_token}"))
            .send()
            .await?;

        if !resp.status().is_success() {
            return Err(AppError::NotFound("落雪获取成绩列表失败".into()));
        }

        let body: LxnsResponse<Vec<LxnsScore>> = resp.json().await?;
        let scores = body.data.unwrap_or_default();
        Ok(scores.into_iter().map(|s| Self::convert_score(s, false)).collect())
    }

    // ==========================================
    // Developer Token / Friend Code Endpoints (/api/v0/maimai)
    // ==========================================

    /// Query player info via QQ number
    pub async fn query_player_by_qq(&self, qq: &str) -> Result<LxnsPlayer, AppError> {
        let dev_token = CONFIG.lxns_token.as_deref().ok_or_else(|| {
            AppError::InvalidInput("落雪开发者 Token 未配置 (LXNS_DEVELOPER_TOKEN)".into())
        })?;

        let url = format!("https://maimai.lxns.net/api/v0/maimai/player/qq/{qq}");
        let resp = self
            .client
            .get(&url)
            .header("Authorization", dev_token)
            .send()
            .await?;

        if !resp.status().is_success() {
            return Err(AppError::NotFound(format!("落雪未找到绑定的 QQ: {qq}")));
        }

        let body: LxnsResponse<LxnsPlayer> = resp.json().await?;
        body.data.ok_or_else(|| {
            AppError::NotFound(body.message.unwrap_or_else(|| "未找到落雪玩家".into()))
        })
    }

    /// Query player info via friend code
    pub async fn query_player_by_friend_code(&self, friend_code: u64) -> Result<LxnsPlayer, AppError> {
        let dev_token = CONFIG.lxns_token.as_deref().ok_or_else(|| {
            AppError::InvalidInput("落雪开发者 Token 未配置".into())
        })?;

        let url = format!("https://maimai.lxns.net/api/v0/maimai/player/{friend_code}");
        let resp = self
            .client
            .get(&url)
            .header("Authorization", dev_token)
            .send()
            .await?;

        if !resp.status().is_success() {
            return Err(AppError::NotFound(format!(
                "落雪未找到好友码: {friend_code}"
            )));
        }

        let body: LxnsResponse<LxnsPlayer> = resp.json().await?;
        body.data.ok_or_else(|| AppError::NotFound("未找到落雪玩家".into()))
    }

    /// Query player B50 via friend_code
    pub async fn query_b50_by_friend_code(
        &self,
        friend_code: u64,
        user_token: Option<&str>,
    ) -> Result<PlayerB50, AppError> {
        let url = format!(
            "https://maimai.lxns.net/api/v0/maimai/player/{friend_code}/bests"
        );
        let mut req = self.client.get(&url);

        if let Some(t) = user_token {
            req = req.header("X-User-Token", t);
        } else if let Some(dev_t) = CONFIG.lxns_token.as_deref() {
            req = req.header("Authorization", dev_t);
        }

        let resp = req.send().await?;
        if !resp.status().is_success() {
            return Err(AppError::NotFound(format!(
                "落雪未找到用户好友码: {friend_code}"
            )));
        }

        let body: LxnsResponse<LxnsB50Data> = resp.json().await?;
        let data = body.data.ok_or_else(|| {
            AppError::NotFound(body.message.unwrap_or_else(|| "落雪接口返回错误".into()))
        })?;

        let b35: Vec<ScoreRecord> = data
            .standard
            .unwrap_or_default()
            .into_iter()
            .map(|s| Self::convert_score(s, false))
            .collect();

        let b15: Vec<ScoreRecord> = data
            .dx
            .unwrap_or_default()
            .into_iter()
            .map(|s| Self::convert_score(s, true))
            .collect();

        let b35_rating: u32 = b35.iter().map(|s| s.dx_rating).sum();
        let b15_rating: u32 = b15.iter().map(|s| s.dx_rating).sum();

        Ok(PlayerB50 {
            nickname: format!("LXNS:{friend_code}"),
            rating: b35_rating + b15_rating,
            b35,
            b15,
            b35_rating,
            b15_rating,
            plate: None,
            course_rank: 0,
            class_rank: 0,
        })
    }

    /// Query player B50 via personal user API token
    pub async fn query_b50_by_user_token(&self, user_token: &str) -> Result<PlayerB50, AppError> {
        let url = "https://maimai.lxns.net/api/v0/user/maimai/player/bests";
        let resp = self
            .client
            .get(url)
            .header("X-User-Token", user_token)
            .send()
            .await?;

        if !resp.status().is_success() {
            return Err(AppError::NotFound("落雪 Token 验证失败或未找到成绩".into()));
        }

        let body: LxnsResponse<LxnsB50Data> = resp.json().await?;
        let data = body.data.ok_or_else(|| {
            AppError::NotFound(body.message.unwrap_or_else(|| "落雪接口返回错误".into()))
        })?;

        let b35: Vec<ScoreRecord> = data
            .standard
            .unwrap_or_default()
            .into_iter()
            .map(|s| Self::convert_score(s, false))
            .collect();

        let b15: Vec<ScoreRecord> = data
            .dx
            .unwrap_or_default()
            .into_iter()
            .map(|s| Self::convert_score(s, true))
            .collect();

        let b35_rating: u32 = b35.iter().map(|s| s.dx_rating).sum();
        let b15_rating: u32 = b15.iter().map(|s| s.dx_rating).sum();

        Ok(PlayerB50 {
            nickname: "LXNS Player".to_string(),
            rating: b35_rating + b15_rating,
            b35,
            b15,
            b35_rating,
            b15_rating,
            plate: None,
            course_rank: 0,
            class_rank: 0,
        })
    }

    // ==========================================
    // Smart User Query Dispatcher
    // ==========================================

    /// Automatically select the optimal method to query B50 for a user (OAuth -> User Token -> Friend Code -> Dev Token QQ)
    pub async fn query_b50_smart(
        &self,
        qq: Option<&str>,
        friend_code: Option<u64>,
        user_token: Option<&str>,
    ) -> Result<PlayerB50, AppError> {
        // 1. Try OAuth token if QQ is provided and bound
        if let Some(q) = qq {
            if let Ok(access_token) = self.get_valid_token_for_user(q).await {
                if let Ok(b50) = self.query_b50_oauth(&access_token).await {
                    return Ok(b50);
                }
            }
        }

        // 2. Try User Token if provided or stored
        if let Some(tok) = user_token {
            if let Ok(b50) = self.query_b50_by_user_token(tok).await {
                return Ok(b50);
            }
        } else if let Some(q) = qq {
            let u = UserStore::get_user(q);
            if let Some(tok) = &u.lxns_token {
                if let Ok(b50) = self.query_b50_by_user_token(tok).await {
                    return Ok(b50);
                }
            }
            if let Some(fc) = u.lxns_friend_code {
                if let Ok(b50) = self.query_b50_by_friend_code(fc, None).await {
                    return Ok(b50);
                }
            }
        }

        // 3. Try Friend Code directly
        if let Some(fc) = friend_code {
            match self.query_b50_by_friend_code(fc, None).await {
                Ok(b50) => return Ok(b50),
                Err(_) => {
                    return Err(AppError::NotFound(format!(
                        "⚠️ 落雪查分失败 (好友码: {fc})\n【未找到成绩或未开启第三方公开权限】\n\n📌 解决方案：\n1️⃣ 请在落雪查分器 (https://maimai.lxns.net/) 设置中开启【允许第三方读取成绩】。\n2️⃣ 推荐使用落雪 OAuth 授权，无需公开设置即可查分。"
                    )));
                }
            }
        }

        // 4. Try Developer Token with QQ
        if let Some(q) = qq {
            if let Ok(player) = self.query_player_by_qq(q).await {
                if let Some(fc) = player.friend_code {
                    if let Ok(mut b50) = self.query_b50_by_friend_code(fc, None).await {
                        b50.nickname = player.name;
                        b50.rating = player.rating;
                        b50.course_rank = player.course_rank.unwrap_or(0);
                        b50.class_rank = player.class_rank.unwrap_or(0);
                        return Ok(b50);
                    }
                }
            }
        }

        // Generate friendly multi-scenario guidance
        let mut hint = format!(
            "⚠️ 落雪查分失败{}\n【未检测到落雪 OAuth 授权、有效 Token 或公开好友码】\n\n📌 解决方案（任选其一）：",
            qq.map(|q| format!(" (QQ: {q})")).unwrap_or_default()
        );

        if Self::is_oauth_configured() {
            if let Ok(auth_url) = Self::get_authorize_url(qq) {
                hint.push_str(&format!(
                    "\n1️⃣ 【落雪 OAuth 授权（推荐）】\n在浏览器打开下方链接授权，获取 12 位授权码（格式 `XXXX-XXXX-XXXX`）：\n👉 {}\n获取授权码后，发送 `mai_bind_player(lxns_code=\"授权码\")` 完成绑定即可查分。",
                    auth_url
                ));
            }
        }

        hint.push_str("\n2️⃣ 【绑定好友码】发送 `mai_bind_player(lxns_friend_code=你的好友码)`，并在落雪设置中开启【允许第三方读取成绩】。");
        hint.push_str("\n3️⃣ 【绑定个人 Token】在落雪查分器控制台生成 Token，发送 `mai_bind_player(lxns_token=\"你的Token\")`。");
        hint.push_str("\n4️⃣ 【切换水鱼源】若使用水鱼查分，请指定 `source=\"divingfish\"`。");

        Err(AppError::NotFound(hint))
    }

    /// Query all scores for a user (OAuth or friend_code)
    pub async fn query_all_scores_smart(
        &self,
        qq: Option<&str>,
        friend_code: Option<u64>,
    ) -> Result<Vec<ScoreRecord>, AppError> {
        if let Some(q) = qq {
            if let Ok(access_token) = self.get_valid_token_for_user(q).await {
                if let Ok(scores) = self.query_scores_oauth(&access_token).await {
                    return Ok(scores);
                }
            }
        }

        // Fallback to B50 scores
        let b50 = self.query_b50_smart(qq, friend_code, None).await?;
        Ok(b50.b35.into_iter().chain(b50.b15.into_iter()).collect())
    }
}

