//! DivingFish prober: public B50, OAuth records, ranking and the OAuth binding
//! flow (`handoff=code` device authorization + confirmation code).

use super::{BULK_TIMEOUT, HTTP};
use crate::config::CONFIG;
use crate::core::models::Played;
use crate::error::{AppError, Result};
use base64::Engine;
use reqwest::{Response, StatusCode};
use serde::Deserialize;
use serde_json::{Value, json};
use sha2::{Digest, Sha256};
use std::collections::HashMap;
use std::sync::{LazyLock, Mutex};
use std::time::{Duration, Instant};

const ON_BEHALF_OF_GRANT: &str = "urn:diving-fish:params:oauth:grant-type:on-behalf-of";
const CONFIRMATION_CODE_GRANT: &str = "urn:diving-fish:params:oauth:grant-type:confirmation-code";
pub const REVOKE_URL: &str = "/apps";
const EXPIRES_MARGIN: u64 = 30;

pub fn api_base() -> &'static str {
    if CONFIG.df_proxy {
        "https://proxy.yuzuchan.site/maimaidxprober"
    } else {
        "https://maimai.diving-fish.com/api/maimaidxprober"
    }
}

#[derive(Debug, Clone, Deserialize)]
pub struct DfRecord {
    pub achievements: f64,
    #[serde(default)]
    pub fc: String,
    #[serde(default)]
    pub fs: String,
    #[serde(default)]
    pub level: String,
    pub level_index: u8,
    #[serde(default)]
    pub title: String,
    #[serde(rename = "type", default)]
    pub kind: String,
    #[serde(default)]
    pub ds: f64,
    #[serde(rename = "dxScore", default)]
    pub dx_score: u32,
    #[serde(default)]
    pub ra: u32,
    #[serde(default)]
    pub rate: String,
    pub song_id: u32,
}

impl DfRecord {
    pub fn to_played(&self) -> Played {
        let opt = |s: &str| (!s.is_empty()).then(|| s.to_string());
        Played {
            song_id: self.song_id,
            song_name: self.title.clone(),
            level: self.level.clone(),
            level_index: self.level_index,
            level_value: self.ds,
            kind: self.kind.to_uppercase(),
            rating: self.ra,
            achievements: self.achievements,
            rate: opt(&self.rate),
            fc: opt(&self.fc),
            fs: opt(&self.fs),
            dx_score: self.dx_score,
        }
    }
}

#[derive(Debug, Default, Deserialize)]
pub struct DfCharts {
    #[serde(default)]
    pub sd: Vec<DfRecord>,
    #[serde(default)]
    pub dx: Vec<DfRecord>,
}

/// `UserInfo` (b50) and `UserAllRecords` share this shape.
#[derive(Debug, Default, Deserialize)]
pub struct DfUser {
    pub additional_rating: Option<u32>,
    pub nickname: Option<String>,
    pub plate: Option<String>,
    pub rating: Option<u32>,
    pub username: Option<String>,
    #[serde(default)]
    pub charts: DfCharts,
    #[serde(default)]
    pub records: Vec<DfRecord>,
}

/// `RecordsQueryRequest` for `GET /player/records`.
#[derive(Debug, Default, Clone)]
pub struct RecordsQuery {
    pub song_id: Option<u32>,
    pub level: Option<String>,
    pub level_value: Option<f64>,
    pub fc: Vec<String>,
    pub fs: Vec<String>,
    pub version: Vec<String>,
}

impl RecordsQuery {
    fn params(&self) -> Vec<(&'static str, String)> {
        let mut p = Vec::new();
        p.extend(self.song_id.map(|v| ("song_id", v.to_string())));
        p.extend(self.level.clone().map(|v| ("level", v)));
        p.extend(self.level_value.map(|v| ("ds", crate::util::py_float(v))));
        p.extend(self.fc.iter().map(|v| ("fc", v.clone())));
        p.extend(self.fs.iter().map(|v| ("fs", v.clone())));
        p.extend(self.version.iter().map(|v| ("version", v.clone())));
        p
    }
}

pub struct DivingFish {
    qq: Option<String>,
    username: Option<String>,
}

impl DivingFish {
    pub fn new(qq: Option<&str>, username: Option<&str>) -> Self {
        Self { qq: qq.map(str::to_string), username: username.map(str::to_string) }
    }

    fn oauth(&self) -> bool {
        CONFIG.df_oauth_enabled() && self.qq.is_some() && self.username.is_none()
    }

    async fn check(&self, resp: Response) -> Result<Response> {
        match resp.status() {
            StatusCode::OK => Ok(resp),
            StatusCode::BAD_REQUEST => {
                let body: Value = resp.json().await.unwrap_or_default();
                let msg = body.get("message").or_else(|| body.get("msg")).and_then(Value::as_str);
                Err(match msg {
                    Some("no such user") => AppError::DfUserNotFound,
                    Some("user not exists") => AppError::UserNotExists,
                    Some("开发者token有误" | "开发者token被禁用" | "请先联系水鱼申请开发者token") => {
                        AppError::DfToken
                    }
                    _ => AppError::Other(format!("DivingFish 400 {msg:?}")),
                })
            }
            StatusCode::UNAUTHORIZED => Err(AppError::DfToken),
            StatusCode::FORBIDDEN if self.oauth() => Err(AppError::DfNotAuthorized),
            StatusCode::FORBIDDEN => Err(AppError::DfQueryDisabled),
            StatusCode::TOO_MANY_REQUESTS => Err(AppError::DfTooManyRequests),
            s => Err(AppError::Other(format!("DivingFish HTTP {s}"))),
        }
    }

    /// Best 50 via the public endpoint (works without OAuth).
    pub async fn query_b50(&self) -> Result<DfUser> {
        let mut body = json!({ "b50": true });
        if let Some(qq) = &self.qq {
            body["qq"] = json!(qq.parse::<u64>().unwrap_or(0));
        }
        if let Some(name) = &self.username {
            body["username"] = json!(name);
        }
        let resp = HTTP.post(format!("{}/query/player", api_base())).json(&body).send().await?;
        Ok(self.check(resp).await?.json().await?)
    }

    /// All records on behalf of the user (OAuth), retrying once with a fresh token.
    pub async fn records(&self, query: &RecordsQuery) -> Result<DfUser> {
        let qq = self.qq.as_deref().ok_or(AppError::DfNotAuthorized)?;
        let url = format!("{}/player/records", api_base());
        let mut refresh = false;
        loop {
            let token = access_token(qq, refresh).await?;
            let resp = HTTP.get(&url).bearer_auth(token).query(&query.params()).send().await?;
            if resp.status() == StatusCode::UNAUTHORIZED && !refresh {
                refresh = true;
                continue;
            }
            return Ok(self.check(resp).await?.json().await?);
        }
    }

    /// Prober rating ranking sorted by rating descending.
    pub async fn rating_ranking() -> Result<Vec<(String, u32)>> {
        #[derive(Deserialize)]
        struct Rank {
            username: String,
            ra: u32,
        }
        let resp = HTTP.get(format!("{}/rating_ranking", api_base())).timeout(BULK_TIMEOUT).send().await?;
        let mut list: Vec<Rank> = Self::new(None, None).check(resp).await?.json().await?;
        list.sort_by_key(|r| std::cmp::Reverse(r.ra));
        Ok(list.into_iter().map(|r| (r.username, r.ra)).collect())
    }

    /// Raw `music_data` / `chart_stats` JSON.
    pub async fn download(endpoint: &str) -> Result<Vec<u8>> {
        let resp = HTTP.get(format!("{}/{endpoint}", api_base())).timeout(BULK_TIMEOUT).send().await?;
        Ok(resp.error_for_status()?.bytes().await?.to_vec())
    }
}

// ---------------------------------------------------------------- OAuth

#[derive(Debug, Clone, Deserialize)]
pub struct DeviceAuthorization {
    pub verification_uri_complete: String,
    pub expires_in: u64,
}

#[derive(Debug, Clone, Deserialize)]
struct AccessToken {
    access_token: String,
    expires_in: u64,
    #[serde(default)]
    sub: Option<String>,
}

static TOKENS: LazyLock<Mutex<HashMap<String, (String, Instant)>>> = LazyLock::new(Default::default);

/// `sha256("<client_id>:<qq>")`; the QQ number never leaves the bot.
pub fn subject_ref(qq: &str) -> String {
    let id = CONFIG.df_client_id.as_deref().unwrap_or_default();
    Sha256::digest(format!("{id}:{qq}").as_bytes()).iter().map(|b| format!("{b:02x}")).collect()
}

/// Identity shown on the consent page, e.g. `QQ 12*****89`.
pub fn binding_label(qq: &str) -> String {
    let n = qq.chars().count();
    if n <= 4 {
        return format!("QQ {qq}");
    }
    let head: String = qq.chars().take(2).collect();
    let tail: String = qq.chars().skip(n - 2).collect();
    format!("QQ {head}{}{tail}", "*".repeat(n - 4))
}

/// Reads `sub` from a JWT payload without verifying it (consistency check only).
pub fn token_subject(token: &str) -> Option<String> {
    let payload = token.split('.').nth(1)?;
    let bytes = base64::engine::general_purpose::URL_SAFE_NO_PAD.decode(payload.trim_end_matches('=')).ok()?;
    serde_json::from_slice::<Value>(&bytes).ok()?.get("sub")?.as_str().map(str::to_string)
}

async fn oauth_post<T: serde::de::DeserializeOwned>(endpoint: &str, form: &[(&str, &str)]) -> Result<T> {
    if !CONFIG.df_oauth_enabled() {
        return Err(AppError::DfToken);
    }
    let resp = HTTP.post(format!("{}{endpoint}", CONFIG.df_auth_url)).form(form).send().await?;
    if resp.status() == StatusCode::OK {
        return Ok(resp.json().await?);
    }
    let body: Value = resp.json().await.unwrap_or_default();
    Err(match body.get("error").and_then(Value::as_str).unwrap_or_default() {
        "consent_required" => AppError::DfNotAuthorized,
        "subject_mismatch" => AppError::DfBindingMismatch,
        "invalid_grant" => AppError::DfConfirmationCode,
        _ => AppError::DfOAuth,
    })
}

fn credentials() -> (&'static str, &'static str) {
    (CONFIG.df_client_id.as_deref().unwrap_or_default(), CONFIG.df_client_secret.as_deref().unwrap_or_default())
}

/// Starts binding; also drops the cached token so a re-bind takes effect at once.
pub async fn device_authorization(qq: &str) -> Result<DeviceAuthorization> {
    let sref = subject_ref(qq);
    if let Ok(mut t) = TOKENS.lock() {
        t.remove(&sref);
    }
    let (id, secret) = credentials();
    let label = binding_label(qq);
    let form = [
        ("client_id", id),
        ("client_secret", secret),
        ("scope", CONFIG.df_scope.as_str()),
        ("subject_ref", sref.as_str()),
        ("binding_label", label.as_str()),
        ("handoff", "code"),
    ];
    oauth_post("/oauth/device_authorization", &form).await
}

async fn redeem(qq: &str, code: &str) -> Result<AccessToken> {
    let (id, secret) = credentials();
    let sref = subject_ref(qq);
    let form = [
        ("grant_type", CONFIRMATION_CODE_GRANT),
        ("client_id", id),
        ("client_secret", secret),
        ("confirmation_code", code),
        ("subject_ref", sref.as_str()),
    ];
    oauth_post("/oauth/token", &form).await
}

/// Token to act on behalf of the user; cached until shortly before expiry.
pub async fn access_token(qq: &str, refresh: bool) -> Result<String> {
    let sref = subject_ref(qq);
    if let Ok(mut cache) = TOKENS.lock() {
        if refresh {
            cache.remove(&sref);
        } else if let Some((tok, exp)) = cache.get(&sref)
            && *exp > Instant::now()
        {
            return Ok(tok.clone());
        }
    }
    let (id, secret) = credentials();
    let subject = format!("ref:{sref}");
    let form = [
        ("grant_type", ON_BEHALF_OF_GRANT),
        ("client_id", id),
        ("client_secret", secret),
        ("subject", subject.as_str()),
        ("scope", CONFIG.df_scope.as_str()),
    ];
    let token: AccessToken = oauth_post("/oauth/token", &form).await?;
    if let Ok(mut cache) = TOKENS.lock() {
        let ttl = Duration::from_secs(token.expires_in.saturating_sub(EXPIRES_MARGIN));
        cache.insert(sref, (token.access_token.clone(), Instant::now() + ttl));
    }
    Ok(token.access_token)
}

/// Redeems the confirmation code and verifies the grant belongs to this QQ.
pub async fn complete_binding(qq: &str, code: &str) -> Result<()> {
    let redeemed = redeem(qq, code).await?;
    let token = match access_token(qq, true).await {
        Err(AppError::DfNotAuthorized) => return Err(AppError::DfBindingMismatch),
        other => other?,
    };
    if redeemed.sub.is_some() && token_subject(&token) != redeemed.sub {
        return Err(AppError::DfBindingMismatch);
    }
    Ok(())
}

const CONFIRMATION_ALPHABET: &str = "BCDFGHJKLMNPQRSTVWXZ";

/// Normalises a whole message into `XXXX-XXXX-XXXX`, or `None` if it is not a code.
pub fn extract_confirmation_code(text: &str) -> Option<String> {
    let mut value = text.trim();
    for prefix in ["确认码", "授权码"] {
        if let Some(rest) = value.strip_prefix(prefix) {
            let rest = rest.trim_start().trim_start_matches([':', '：']).trim_start();
            if !rest.is_empty() && !rest.contains(char::is_whitespace) {
                value = rest;
            }
            break;
        }
    }
    let body: String = value
        .chars()
        .filter(|c| !c.is_whitespace() && !matches!(c, '-' | '—' | '_'))
        .collect::<String>()
        .to_uppercase();
    if body.chars().count() != 12 || !body.chars().all(|c| CONFIRMATION_ALPHABET.contains(c)) {
        return None;
    }
    Some(format!("{}-{}-{}", &body[0..4], &body[4..8], &body[8..12]))
}
