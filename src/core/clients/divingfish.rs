use crate::config::CONFIG;
use crate::core::calc::rating::compute_single_rating;
use crate::core::database::music_db::MUSIC_DB;
use crate::core::models::music::SongType;
use crate::core::models::score::{ComboStatus, PlayerB50, ScoreRecord, SyncStatus};
use crate::error::AppError;
use reqwest::Client;
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::collections::HashMap;
use std::sync::{LazyLock, RwLock};
use std::time::{Duration, Instant};

/// Cached access tokens: subject -> (access_token, expires_at)
static TOKEN_CACHE: LazyLock<RwLock<HashMap<String, (String, Instant)>>> =
    LazyLock::new(|| RwLock::new(HashMap::new()));

/// Compute SHA-256 hash of `{client_id}:{external_id}` in lowercase hex.
pub fn compute_subject_ref(client_id: &str, external_id: &str) -> String {
    let mut hasher = Sha256::new();
    hasher.update(format!("{client_id}:{external_id}").as_bytes());
    let hash = hasher.finalize();
    hash.iter().map(|b| format!("{:02x}", b)).collect()
}

/// Helper to mask QQ for safe display in OAuth binding label
pub fn mask_qq(qq: &str) -> String {
    let q = qq.trim();
    if q.len() > 4 {
        format!("QQ {}****{}", &q[..2], &q[q.len() - 2..])
    } else {
        "QQ ****".to_string()
    }
}

#[derive(Debug, Deserialize, Serialize, Clone)]
pub struct DeviceAuthResponse {
    pub device_code: String,
    pub user_code: String,
    pub verification_uri: String,
    pub verification_uri_complete: String,
    pub expires_in: u64,
    pub interval: u64,
}

#[derive(Debug, Deserialize)]
pub struct OAuthTokenResponse {
    pub access_token: String,
    pub token_type: String,
    pub expires_in: u64,
    pub scope: Option<String>,
    pub refresh_token: Option<String>,
    pub sub: Option<String>,
}

#[derive(Debug, Deserialize)]
pub struct OAuthUserInfoResponse {
    pub sub: String,
    pub preferred_username: Option<String>,
    pub name: Option<String>,
    pub nickname: Option<String>,
}

#[derive(Debug, Deserialize, Clone)]
#[allow(dead_code)]
pub struct DfScore {
    pub achievements: f64,
    pub ds: f64,
    #[serde(rename = "dxScore")]
    pub dx_score: Option<u32>,
    pub fc: Option<String>,
    pub fs: Option<String>,
    pub level_index: u8,
    pub level_label: Option<String>,
    pub ra: Option<u32>,
    pub rate: Option<String>,
    pub song_id: u32,
    pub title: String,
    #[serde(rename = "type")]
    pub song_type: Option<String>,
}

#[derive(Debug, Deserialize)]
pub struct OAuthRecordsResponse {
    pub username: Option<String>,
    pub nickname: Option<String>,
    pub rating: Option<u32>,
    pub additional_rating: Option<u32>,
    pub plate: Option<String>,
    pub records: Option<Vec<DfScore>>,
}

#[derive(Debug, Deserialize)]
struct DfB50Response {
    nickname: Option<String>,
    rating: Option<u32>,
    additional_rating: Option<u32>,
    charts: Option<DfCharts>,
    plate: Option<String>,
    message: Option<String>,
}

#[derive(Debug, Deserialize)]
struct DfCharts {
    dx: Option<Vec<DfScore>>,
    sd: Option<Vec<DfScore>>,
}

#[derive(Debug, Deserialize)]
struct DfPlateResponse {
    verlist: Option<Vec<DfScore>>,
}

pub struct DivingFishClient {
    client: Client,
}

impl DivingFishClient {
    pub fn new() -> Self {
        let client = Client::builder()
            .timeout(Duration::from_secs(15))
            .build()
            .unwrap_or_default();
        Self { client }
    }

    /// Check if OAuth is configured (both client_id and client_secret present)
    pub fn is_oauth_configured() -> bool {
        CONFIG.divingfish_client_id.is_some() && CONFIG.divingfish_client_secret.is_some()
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

    pub fn convert_score(s: DfScore, is_new: bool) -> Option<ScoreRecord> {
        // Exclude UTAGE charts (song_id >= 100000) from standard/dx rating calculations
        if s.song_id >= 100000 {
            return None;
        }

        let song_type = match s.song_type.as_deref() {
            Some("DX") => SongType::Deluxe,
            Some("UTAGE") => SongType::Utage,
            _ => SongType::Standard,
        };

        let rating = if let Some(ra) = s.ra {
            if ra > 0 {
                ra
            } else {
                compute_single_rating(s.ds, s.achievements)
            }
        } else {
            compute_single_rating(s.ds, s.achievements)
        };

        let rate = s.rate.unwrap_or_else(|| {
            crate::core::calc::rating::get_rate_string(s.achievements).to_string()
        });

        Some(ScoreRecord {
            song_id: s.song_id,
            title: s.title,
            level_index: s.level_index,
            song_type,
            ds: s.ds,
            achievements: s.achievements,
            dx_score: s.dx_score.unwrap_or(0),
            dx_rating: rating,
            rate,
            fc: Self::map_combo(s.fc.as_deref()),
            fs: Self::map_sync(s.fs.as_deref()),
            is_new,
        })
    }

    // ==========================================
    // OAuth 2.0 Flow Support
    // ==========================================

    /// Start Device Authorization Flow for binding a user
    pub async fn start_device_binding(
        &self,
        external_id: &str,
        label: Option<&str>,
    ) -> Result<DeviceAuthResponse, AppError> {
        let client_id = CONFIG.divingfish_client_id.as_deref().ok_or_else(|| {
            AppError::InvalidInput("水鱼 OAuth client_id 未配置 (DIVINGFISH_CLIENT_ID)".into())
        })?;
        let client_secret = CONFIG.divingfish_client_secret.as_deref().ok_or_else(|| {
            AppError::InvalidInput("水鱼 OAuth client_secret 未配置 (DIVINGFISH_CLIENT_SECRET)".into())
        })?;

        let subject_ref = compute_subject_ref(client_id, external_id);
        let binding_label = label
            .map(|s| s.to_string())
            .unwrap_or_else(|| mask_qq(external_id));

        let auth_url = "https://auth.diving-fish.com/oauth/device_authorization";
        let resp = self
            .client
            .post(auth_url)
            .form(&[
                ("client_id", client_id),
                ("client_secret", client_secret),
                ("scope", "prober.records.read"),
                ("subject_ref", &subject_ref),
                ("binding_label", &binding_label),
            ])
            .send()
            .await?;

        if !resp.status().is_success() {
            let status = resp.status();
            let text = resp.text().await.unwrap_or_default();
            return Err(AppError::Network(format!(
                "发起水鱼设备码绑定失败 (HTTP {status}): {text}"
            )));
        }

        let body: DeviceAuthResponse = resp.json().await?;
        Ok(body)
    }

    /// Obtain or return cached access token via `urn:diving-fish:params:oauth:grant-type:on-behalf-of`
    pub async fn get_access_token(&self, subject: &str) -> Result<String, AppError> {
        let client_id = CONFIG.divingfish_client_id.as_deref().ok_or_else(|| {
            AppError::InvalidInput("水鱼 OAuth client_id 未配置".into())
        })?;
        let client_secret = CONFIG.divingfish_client_secret.as_deref().ok_or_else(|| {
            AppError::InvalidInput("水鱼 OAuth client_secret 未配置".into())
        })?;

        // 1. Check local in-memory cache (valid for 5 minutes, leave 30s buffer)
        {
            let cache = TOKEN_CACHE.read().unwrap();
            if let Some((tok, exp)) = cache.get(subject) {
                if Instant::now() + Duration::from_secs(30) < *exp {
                    return Ok(tok.clone());
                }
            }
        }

        // 2. Token exchange request
        let token_url = "https://auth.diving-fish.com/oauth/token";
        let resp = self
            .client
            .post(token_url)
            .form(&[
                (
                    "grant_type",
                    "urn:diving-fish:params:oauth:grant-type:on-behalf-of",
                ),
                ("client_id", client_id),
                ("client_secret", client_secret),
                ("subject", subject),
                ("scope", "prober.records.read"),
            ])
            .send()
            .await?;

        if !resp.status().is_success() {
            let status = resp.status();
            let err_json: serde_json::Value = resp.json().await.unwrap_or_default();
            let error_code = err_json.get("error").and_then(|v| v.as_str()).unwrap_or("");
            if error_code == "consent_required" {
                return Err(AppError::NotFound(
                    "用户未授权水鱼查分 (consent_required)。".into(),
                ));
            } else if error_code == "slow_down" || status.as_u16() == 429 {
                return Err(AppError::Network(
                    "换票过于频繁，已触发限流 (slow_down)，请稍后重试。".into(),
                ));
            }
            let desc = err_json
                .get("error_description")
                .and_then(|v| v.as_str())
                .unwrap_or(error_code);
            return Err(AppError::Network(format!(
                "水鱼 OAuth 换票失败 (HTTP {status}): {desc}"
            )));
        }

        let body: OAuthTokenResponse = resp.json().await?;
        let expires_at = Instant::now() + Duration::from_secs(body.expires_in);
        {
            let mut cache = TOKEN_CACHE.write().unwrap();
            cache.insert(subject.to_string(), (body.access_token.clone(), expires_at));
        }

        Ok(body.access_token)
    }

    /// Query player full records via OAuth Bearer token (`GET /player/records`)
    pub async fn query_records_oauth(
        &self,
        subject: &str,
    ) -> Result<OAuthRecordsResponse, AppError> {
        let token = self.get_access_token(subject).await?;
        let url = "https://www.diving-fish.com/api/maimaidxprober/player/records";
        let resp = self
            .client
            .get(url)
            .header("Authorization", format!("Bearer {token}"))
            .send()
            .await?;

        if resp.status().as_u16() == 401 {
            // Invalidate cached token
            let mut cache = TOKEN_CACHE.write().unwrap();
            cache.remove(subject);
            return Err(AppError::Network("OAuth 令牌已失效，请重试".into()));
        }

        if !resp.status().is_success() {
            let status = resp.status();
            let text = resp.text().await.unwrap_or_default();
            return Err(AppError::Network(format!(
                "获取水鱼成绩失败 (HTTP {status}): {text}"
            )));
        }

        let body: OAuthRecordsResponse = resp.json().await?;
        Ok(body)
    }

    /// Query single song records via OAuth (`POST /player/record`)
    pub async fn query_song_record_oauth(
        &self,
        subject: &str,
        music_id: u32,
    ) -> Result<Vec<ScoreRecord>, AppError> {
        let token = self.get_access_token(subject).await?;
        let url = "https://www.diving-fish.com/api/maimaidxprober/player/record";
        let resp = self
            .client
            .post(url)
            .header("Authorization", format!("Bearer {token}"))
            .json(&serde_json::json!({ "music_id": [music_id] }))
            .send()
            .await?;

        if !resp.status().is_success() {
            let status = resp.status();
            let text = resp.text().await.unwrap_or_default();
            return Err(AppError::Network(format!(
                "获取水鱼单曲成绩失败 (HTTP {status}): {text}"
            )));
        }

        // Response is a map of music_id -> Vec<DfScore> or array
        let body: serde_json::Value = resp.json().await?;
        let mut results = Vec::new();
        if let Some(map) = body.as_object() {
            for (_, list_val) in map {
                if let Ok(scores) = serde_json::from_value::<Vec<DfScore>>(list_val.clone()) {
                    for s in scores {
                        if let Some(rec) = Self::convert_score(s, false) {
                            results.push(rec);
                        }
                    }
                }
            }
        } else if let Ok(scores) = serde_json::from_value::<Vec<DfScore>>(body) {
            for s in scores {
                if let Some(rec) = Self::convert_score(s, false) {
                    results.push(rec);
                }
            }
        }

        Ok(results)
    }

    /// Query player plate records via OAuth (`POST /player/plate`)
    pub async fn query_plate_records_oauth(
        &self,
        subject: &str,
        versions: &[&str],
    ) -> Result<Vec<ScoreRecord>, AppError> {
        let token = self.get_access_token(subject).await?;
        let url = "https://www.diving-fish.com/api/maimaidxprober/player/plate";
        let resp = self
            .client
            .post(url)
            .header("Authorization", format!("Bearer {token}"))
            .json(&serde_json::json!({ "version": versions }))
            .send()
            .await?;

        if !resp.status().is_success() {
            let status = resp.status();
            let text = resp.text().await.unwrap_or_default();
            return Err(AppError::Network(format!(
                "获取水鱼牌子成绩失败 (HTTP {status}): {text}"
            )));
        }

        let body: DfPlateResponse = resp.json().await?;
        let mut results = Vec::new();
        if let Some(verlist) = body.verlist {
            for s in verlist {
                if let Some(rec) = Self::convert_score(s, false) {
                    results.push(rec);
                }
            }
        }
        Ok(results)
    }

    /// Query player userinfo via OAuth (`GET /oauth/userinfo`)
    pub async fn query_userinfo(&self, subject: &str) -> Result<OAuthUserInfoResponse, AppError> {
        let token = self.get_access_token(subject).await?;
        let url = "https://auth.diving-fish.com/oauth/userinfo";
        let resp = self
            .client
            .get(url)
            .header("Authorization", format!("Bearer {token}"))
            .send()
            .await?;

        if !resp.status().is_success() {
            let status = resp.status();
            let text = resp.text().await.unwrap_or_default();
            return Err(AppError::Network(format!(
                "获取水鱼用户信息失败 (HTTP {status}): {text}"
            )));
        }

        let body: OAuthUserInfoResponse = resp.json().await?;
        Ok(body)
    }

    /// Revoke token via OAuth (`POST /oauth/revoke`)
    pub async fn revoke_token(&self, token: &str) -> Result<(), AppError> {
        let client_id = CONFIG.divingfish_client_id.as_deref().ok_or_else(|| {
            AppError::InvalidInput("水鱼 OAuth client_id 未配置".into())
        })?;
        let client_secret = CONFIG.divingfish_client_secret.as_deref().ok_or_else(|| {
            AppError::InvalidInput("水鱼 OAuth client_secret 未配置".into())
        })?;

        let url = "https://auth.diving-fish.com/oauth/revoke";
        let _ = self
            .client
            .post(url)
            .form(&[
                ("token", token),
                ("client_id", client_id),
                ("client_secret", client_secret),
            ])
            .send()
            .await;
        Ok(())
    }

    /// Convert full player records into standard PlayerB50 structure
    pub fn compute_b50_from_records(
        resp: OAuthRecordsResponse,
        fallback_name: &str,
    ) -> PlayerB50 {
        let music_db = MUSIC_DB.read().unwrap();
        let mut sd_candidates: Vec<ScoreRecord> = Vec::new();
        let mut dx_candidates: Vec<ScoreRecord> = Vec::new();

        if let Some(records) = resp.records {
            for score in records {
                let song_id_str = score.song_id.to_string();
                let is_new = if let Some(song) = music_db.get_by_id(&song_id_str) {
                    song.basic_info.is_new
                } else {
                    false
                };

                if let Some(rec) = Self::convert_score(score, is_new) {
                    if is_new {
                        dx_candidates.push(rec);
                    } else {
                        sd_candidates.push(rec);
                    }
                }
            }
        }

        // Sort descending by dx_rating, then achievements
        sd_candidates.sort_by(|a, b| {
            b.dx_rating
                .cmp(&a.dx_rating)
                .then_with(|| b.achievements.partial_cmp(&a.achievements).unwrap())
        });
        dx_candidates.sort_by(|a, b| {
            b.dx_rating
                .cmp(&a.dx_rating)
                .then_with(|| b.achievements.partial_cmp(&a.achievements).unwrap())
        });

        let b35: Vec<ScoreRecord> = sd_candidates.into_iter().take(35).collect();
        let b15: Vec<ScoreRecord> = dx_candidates.into_iter().take(15).collect();

        let b35_rating: u32 = b35.iter().map(|s| s.dx_rating).sum();
        let b15_rating: u32 = b15.iter().map(|s| s.dx_rating).sum();
        let total_rating = resp.rating.unwrap_or(b35_rating + b15_rating);

        PlayerB50 {
            nickname: resp
                .nickname
                .or(resp.username)
                .unwrap_or_else(|| fallback_name.to_string()),
            rating: total_rating,
            b35,
            b15,
            b35_rating,
            b15_rating,
            plate: resp.plate,
            course_rank: resp.additional_rating.unwrap_or(0),
            class_rank: 0,
        }
    }

    // ==========================================
    // High-Level Query Methods (with OAuth dispatch & Fallback)
    // ==========================================

    // ==========================================
    // High-Level Query Methods (with OAuth dispatch & Fallback)
    // ==========================================

    /// Query player B50 via QQ number (tries OAuth ref, then fallback to public /query/player, with rich multi-scenario guidance)
    pub async fn query_b50_by_qq(&self, qq: &str) -> Result<PlayerB50, AppError> {
        if Self::is_oauth_configured() {
            let client_id = CONFIG.divingfish_client_id.as_deref().unwrap();
            let subject = format!("ref:{}", compute_subject_ref(client_id, qq));

            match self.query_records_oauth(&subject).await {
                Ok(records_resp) => {
                    return Ok(Self::compute_b50_from_records(
                        records_resp,
                        &format!("QQ:{qq}"),
                    ));
                }
                Err(err) => {
                    log::info!("DivingFish OAuth query for QQ {qq} unfulfilled ({err}), falling back to public endpoint");
                }
            }
        }

        // Fallback: Legacy / Public B50 endpoint
        match self.query_legacy_b50_by_qq(qq).await {
            Ok(b50) => Ok(b50),
            Err(legacy_err) => {
                // Generate friendly multi-scenario guidance
                let mut hint = format!(
                    "⚠️ 水鱼查分失败 (QQ: {qq})\n【未找到公开成绩或未授权】\n\n📌 解决方案（任选其一）："
                );

                if Self::is_oauth_configured() {
                    if let Ok(dev) = self.start_device_binding(qq, None).await {
                        hint.push_str(&format!(
                            "\n1️⃣ 【水鱼 OAuth 授权（推荐）】\n在浏览器打开下方链接完成 1 次授权绑定（10分钟有效）：\n👉 {}\n绑定后查分无需公开隐私设置即可直接生效。",
                            dev.verification_uri_complete
                        ));
                    } else {
                        hint.push_str("\n1️⃣ 【水鱼 OAuth 授权】请使用 `mai_bind_player(qq=\"...\")` 获取授权链接完成绑定。");
                    }
                }

                hint.push_str("\n2️⃣ 【公开查分权限】登录水鱼查分器 (https://www.diving-fish.com/maimaidx/prober/)，在「编辑个人资料」中勾选【允许其他人查询成绩】。");
                hint.push_str("\n3️⃣ 【绑定用户名】若已注册水鱼，可使用 `username=\"水鱼用户名\"` 查分，或使用 `mai_bind_player(divingfish_username=\"...\")`。");
                hint.push_str("\n4️⃣ 【切换落雪源】可使用参数 `source=\"lxns\"` 或绑定落雪查分器。");

                log::warn!("DivingFish query failed for QQ {qq}: {legacy_err}");
                Err(AppError::NotFound(hint))
            }
        }
    }

    /// Query player B50 via username (tries OAuth username:..., then fallback to public /query/player)
    pub async fn query_b50_by_username(&self, username: &str) -> Result<PlayerB50, AppError> {
        if Self::is_oauth_configured() {
            let subject = format!("username:{username}");
            if let Ok(records_resp) = self.query_records_oauth(&subject).await {
                return Ok(Self::compute_b50_from_records(records_resp, username));
            }
        }

        // Fallback: Legacy / Public B50 endpoint
        match self.query_legacy_b50_by_username(username).await {
            Ok(b50) => Ok(b50),
            Err(_) => {
                let mut hint = format!(
                    "⚠️ 水鱼查分失败 (用户名: {username})\n【未找到公开成绩或用户名不存在】\n\n📌 解决方案："
                );
                hint.push_str("\n1️⃣ 请确认水鱼查分器用户名是否拼写正确（区分大小写）。");
                hint.push_str("\n2️⃣ 登录水鱼查分器官网，在「编辑个人资料」中确认已勾选【允许其他人查询成绩】。");
                hint.push_str("\n3️⃣ 若使用 QQ 查分，可直接指定 `qq=\"你的QQ\"`。");
                hint.push_str("\n4️⃣ 可使用 `source=\"lxns\"` 切换至落雪查分网。");
                Err(AppError::NotFound(hint))
            }
        }
    }

    /// Query player full records for a user (by QQ or username)
    pub async fn query_all_records(&self, qq: Option<&str>, username: Option<&str>) -> Result<Vec<ScoreRecord>, AppError> {
        if Self::is_oauth_configured() {
            let subject = if let (Some(client_id), Some(q)) = (CONFIG.divingfish_client_id.as_deref(), qq) {
                Some(format!("ref:{}", compute_subject_ref(client_id, q)))
            } else {
                username.map(|u| format!("username:{u}"))
            };

            if let Some(sub) = subject {
                if let Ok(resp) = self.query_records_oauth(&sub).await {
                    let mut all_scores = Vec::new();
                    if let Some(records) = resp.records {
                        for s in records {
                            if let Some(rec) = Self::convert_score(s, false) {
                                all_scores.push(rec);
                            }
                        }
                    }
                    return Ok(all_scores);
                }
            }
        }

        // Fallback to B50 records
        let b50 = if let Some(u) = username {
            self.query_b50_by_username(u).await?
        } else if let Some(q) = qq {
            self.query_b50_by_qq(q).await?
        } else {
            return Err(AppError::InvalidInput("请提供 QQ 号或水鱼用户名".into()));
        };

        Ok(b50.b35.into_iter().chain(b50.b15.into_iter()).collect())
    }

    // ==========================================
    // Legacy / Public Endpoints
    // ==========================================

    async fn query_legacy_b50_by_qq(&self, qq: &str) -> Result<PlayerB50, AppError> {
        let url = "https://www.diving-fish.com/api/maimaidxprober/query/player";
        let mut req = self
            .client
            .post(url)
            .json(&serde_json::json!({ "qq": qq, "b50": true }));

        if let Some(token) = &CONFIG.divingfish_token {
            req = req.header("developer-token", token);
        }

        let resp = req.send().await?;
        if !resp.status().is_success() {
            let status = resp.status();
            let text = resp.text().await.unwrap_or_default();
            return Err(AppError::NotFound(format!(
                "水鱼查分未找到用户 QQ: {qq} (HTTP {status}): {text}"
            )));
        }

        let body: DfB50Response = resp.json().await?;
        let charts = body.charts.ok_or_else(|| {
            AppError::NotFound(body.message.unwrap_or_else(|| "未找到成绩数据".into()))
        })?;

        let b35: Vec<ScoreRecord> = charts
            .sd
            .unwrap_or_default()
            .into_iter()
            .filter_map(|s| Self::convert_score(s, false))
            .collect();

        let b15: Vec<ScoreRecord> = charts
            .dx
            .unwrap_or_default()
            .into_iter()
            .filter_map(|s| Self::convert_score(s, true))
            .collect();

        let b35_rating: u32 = b35.iter().map(|s| s.dx_rating).sum();
        let b15_rating: u32 = b15.iter().map(|s| s.dx_rating).sum();
        let total_rating = body.rating.unwrap_or(b35_rating + b15_rating);

        Ok(PlayerB50 {
            nickname: body.nickname.unwrap_or_else(|| format!("QQ:{qq}")),
            rating: total_rating,
            b35,
            b15,
            b35_rating,
            b15_rating,
            plate: body.plate,
            course_rank: body.additional_rating.unwrap_or(0),
            class_rank: 0,
        })
    }

    async fn query_legacy_b50_by_username(&self, username: &str) -> Result<PlayerB50, AppError> {
        let url = "https://www.diving-fish.com/api/maimaidxprober/query/player";
        let req = self
            .client
            .post(url)
            .json(&serde_json::json!({ "username": username, "b50": true }));

        let resp = req.send().await?;
        if !resp.status().is_success() {
            return Err(AppError::NotFound(format!("水鱼未找到用户名: {username}")));
        }

        let body: DfB50Response = resp.json().await?;
        let charts = body.charts.ok_or_else(|| AppError::NotFound("未找到成绩数据".into()))?;

        let b35: Vec<ScoreRecord> = charts
            .sd
            .unwrap_or_default()
            .into_iter()
            .filter_map(|s| Self::convert_score(s, false))
            .collect();

        let b15: Vec<ScoreRecord> = charts
            .dx
            .unwrap_or_default()
            .into_iter()
            .filter_map(|s| Self::convert_score(s, true))
            .collect();

        let b35_rating: u32 = b35.iter().map(|s| s.dx_rating).sum();
        let b15_rating: u32 = b15.iter().map(|s| s.dx_rating).sum();

        Ok(PlayerB50 {
            nickname: body.nickname.unwrap_or_else(|| username.to_string()),
            rating: body.rating.unwrap_or(b35_rating + b15_rating),
            b35,
            b15,
            b35_rating,
            b15_rating,
            plate: body.plate,
            course_rank: body.additional_rating.unwrap_or(0),
            class_rank: 0,
        })
    }
}

