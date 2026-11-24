//! LXNS prober: OAuth user API (auto refresh) and developer API.

use super::{BULK_TIMEOUT, HTTP};
use crate::config::CONFIG;
use crate::core::models::User;
use crate::core::store;
use crate::error::{AppError, Result};
use reqwest::StatusCode;
use serde::Deserialize;
use serde_json::{Value, json};

const BASE: &str = "https://maimai.lxns.net";
const OAUTH_API: &str = "https://maimai.lxns.net/api/v0/user/maimai/player";
const DEV_API: &str = "https://maimai.lxns.net/api/v0/maimai";

#[derive(Debug, Clone, Deserialize)]
pub struct Collection {
    pub id: u32,
    #[serde(default)]
    pub name: String,
    #[serde(default)]
    pub color: Option<String>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct LxPlayer {
    pub name: String,
    pub rating: u32,
    pub friend_code: u64,
    #[serde(default)]
    pub course_rank: u32,
    #[serde(default)]
    pub class_rank: u32,
    pub trophy: Option<Collection>,
    pub icon: Option<Collection>,
    pub name_plate: Option<Collection>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct LxScore {
    pub id: u32,
    #[serde(default)]
    pub song_name: String,
    #[serde(default)]
    pub level: String,
    pub level_index: u8,
    pub achievements: f64,
    pub fc: Option<String>,
    pub fs: Option<String>,
    #[serde(default)]
    pub dx_score: u32,
    pub dx_rating: Option<f64>,
    pub rate: Option<String>,
    #[serde(rename = "type")]
    pub kind: String,
}

#[derive(Debug, Clone, Deserialize)]
pub struct LxBest50 {
    #[serde(default)]
    pub standard_total: u32,
    #[serde(default)]
    pub dx_total: u32,
    #[serde(default)]
    pub standard: Vec<LxScore>,
    #[serde(default)]
    pub dx: Vec<LxScore>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct OAuthToken {
    pub access_token: String,
    pub refresh_token: String,
}

fn status_error(status: StatusCode) -> AppError {
    match status.as_u16() {
        400 => AppError::LxParams,
        401 => AppError::LxOAuth,
        403 => AppError::LxPermission,
        404 => AppError::LxNotFound,
        429 => AppError::LxTooManyRequests,
        s => AppError::Other(format!("LXNS HTTP {s}")),
    }
}

fn data<T: serde::de::DeserializeOwned>(body: Value) -> Result<T> {
    Ok(serde_json::from_value(body.get("data").cloned().unwrap_or(Value::Null))?)
}

async fn dev_get<T: serde::de::DeserializeOwned>(path: &str) -> Result<T> {
    let resp = HTTP.get(format!("{DEV_API}{path}")).header("Authorization", dev_token()?).send().await?;
    if !resp.status().is_success() {
        return Err(status_error(resp.status()));
    }
    data(resp.json().await?)
}

fn dev_token() -> Result<&'static str> {
    CONFIG.lxns_dev_token.as_deref().ok_or(AppError::LxPermission)
}

async fn token_request(body: Value) -> Result<OAuthToken> {
    let resp = HTTP.post(format!("{BASE}/api/v0/oauth/token")).json(&body).send().await?;
    match resp.status() {
        StatusCode::OK => {
            let body: Value = resp.json().await?;
            let inner = body.get("data").filter(|d| d.is_object()).cloned().unwrap_or(body);
            serde_json::from_value(inner).map_err(|_| AppError::LxOAuth)
        }
        StatusCode::UNAUTHORIZED => Err(AppError::LxToken),
        _ => Err(AppError::LxOAuth),
    }
}

/// Exchanges an authorization code for tokens.
pub async fn exchange_code(code: &str) -> Result<OAuthToken> {
    token_request(json!({
        "client_id": CONFIG.lxns_client_id,
        "client_secret": CONFIG.lxns_client_secret,
        "grant_type": "authorization_code",
        "code": code,
        "redirect_uri": CONFIG.lxns_redirect_uri,
    }))
    .await
}

pub fn authorize_url() -> String {
    let params = [
        ("response_type", "code"),
        ("client_id", CONFIG.lxns_client_id.as_deref().unwrap_or_default()),
        ("redirect_uri", CONFIG.lxns_redirect_uri.as_deref().unwrap_or_default()),
        ("scope", "read_player read_user_profile write_player"),
    ];
    reqwest::Url::parse_with_params(&format!("{BASE}/oauth/authorize"), params).map(String::from).unwrap_or_default()
}

/// AP 50 of a player via the developer API.
pub async fn ap50(friend_code: u64) -> Result<LxBest50> {
    dev_get(&format!("/player/{friend_code}/bests/ap")).await
}

/// Raw developer-API JSON (`/song/list?notes=true`, `/alias/list`).
pub async fn download(path: &str) -> Result<Vec<u8>> {
    let resp =
        HTTP.get(format!("{DEV_API}{path}")).header("Authorization", dev_token()?).timeout(BULK_TIMEOUT).send().await?;
    Ok(resp.error_for_status()?.bytes().await?.to_vec())
}

/// OAuth client for one user; refreshes and persists tokens on 401.
pub struct Lxns {
    qq: String,
    access: String,
    refresh: Option<String>,
}

impl Lxns {
    pub fn for_user(user: &User) -> Result<Self> {
        match (&user.lxns_access_token, &user.lxns_refresh_token) {
            (Some(a), r) => Ok(Self { qq: user.qq.clone(), access: a.clone(), refresh: r.clone() }),
            _ => Err(AppError::LxNotBound),
        }
    }

    async fn refresh(&mut self) -> bool {
        let Some(refresh_token) = self.refresh.take() else {
            return false;
        };
        let body = json!({
            "client_id": CONFIG.lxns_client_id,
            "client_secret": CONFIG.lxns_client_secret,
            "grant_type": "refresh_token",
            "refresh_token": refresh_token,
        });
        let Ok(tok) = token_request(body).await else {
            return false;
        };
        let saved = store::update_user(&self.qq, |u| {
            u.lxns_access_token = Some(tok.access_token.clone());
            u.lxns_refresh_token = Some(tok.refresh_token.clone());
        });
        self.access = tok.access_token;
        self.refresh = Some(tok.refresh_token);
        saved.is_ok()
    }

    async fn get<T: serde::de::DeserializeOwned>(&mut self, path: &str, query: &[(&str, String)]) -> Result<T> {
        let mut retried = false;
        loop {
            let resp = HTTP.get(format!("{OAUTH_API}{path}")).bearer_auth(&self.access).query(query).send().await?;
            if resp.status() == StatusCode::UNAUTHORIZED && !retried && self.refresh().await {
                retried = true;
                continue;
            }
            if !resp.status().is_success() {
                return Err(status_error(resp.status()));
            }
            return data(resp.json().await?);
        }
    }

    pub async fn player(&mut self) -> Result<LxPlayer> {
        self.get("", &[]).await
    }

    pub async fn best50(&mut self) -> Result<LxBest50> {
        self.get("/bests", &[]).await
    }

    pub async fn all_best(&mut self) -> Result<Vec<LxScore>> {
        self.get("/scores", &[]).await
    }

    /// All difficulties of one song; `kind` is `standard` / `dx` / `utage`.
    pub async fn song_bests(&mut self, song_id: u32, kind: &str) -> Result<Vec<LxScore>> {
        self.get("/bests", &[("song_id", song_id.to_string()), ("song_type", kind.to_string())]).await
    }
}

/// Accepts a bare code, `授权码 XXXX`, or a full callback URL.
pub fn extract_authorization_code(text: &str) -> Option<String> {
    fn valid(v: &str) -> bool {
        let short = v.len() == 14
            && v.split('-').count() == 3
            && v.split('-').all(|p| p.len() == 4 && p.chars().all(|c| c.is_ascii_uppercase() || c.is_ascii_digit()));
        let long =
            (16..=256).contains(&v.len()) && v.chars().all(|c| c.is_ascii_alphanumeric() || c == '_' || c == '-');
        short || long
    }
    let value = text.trim();
    if valid(value) {
        return Some(value.to_string());
    }
    if let Some(rest) = value.strip_prefix("授权码") {
        let code = rest.trim_start().trim_start_matches([':', '：']).trim_start();
        return valid(code).then(|| code.to_string());
    }
    let url = reqwest::Url::parse(value).ok().filter(|u| matches!(u.scheme(), "http" | "https"))?;
    let code = url.query_pairs().find(|(k, _)| k == "code")?.1.to_string();
    valid(&code).then_some(code)
}
