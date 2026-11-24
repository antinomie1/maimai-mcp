//! Yuzuchan alias / plate server.

use super::{BULK_TIMEOUT, HTTP};
use crate::config::CONFIG;
use crate::error::{AppError, Result};
use serde::Deserialize;
use serde_json::Value;

fn base() -> &'static str {
    if CONFIG.alias_proxy { "https://www.yuzuchan.cn/api/v2" } else { "https://www.yuzuchan.moe/api/v2" }
}

#[derive(Debug, Clone, Deserialize)]
pub struct YuzuAlias {
    pub song_id: u32,
    #[serde(default)]
    pub name: String,
    #[serde(default)]
    pub alias: Vec<String>,
}

/// Raw JSON of all aliases (`/aliases/maimaidx/aliases`).
pub async fn download_aliases() -> Result<Vec<u8>> {
    download("/aliases/maimaidx/aliases").await
}

/// Raw JSON of plate requirements (`/maimaidx/music/get_plate`).
pub async fn download_plates() -> Result<Vec<u8>> {
    download("/maimaidx/music/get_plate").await
}

async fn download(path: &str) -> Result<Vec<u8>> {
    let resp = HTTP.get(format!("{}{path}", base())).timeout(BULK_TIMEOUT).send().await?;
    Ok(resp.error_for_status()?.bytes().await?.to_vec())
}

/// Result of `/aliases/maimaidx/songs?name=`.
pub enum SongsByAlias {
    /// Approved aliases matching the name.
    Songs(Vec<YuzuAlias>),
    /// Ongoing votes `(tag, song_id)` for that alias.
    Votes(Vec<(String, u32)>),
    None,
}

pub async fn songs_by_alias(name: &str) -> Result<SongsByAlias> {
    let resp = HTTP.get(format!("{}/aliases/maimaidx/songs", base())).query(&[("name", name)]).send().await?;
    if resp.status().is_server_error() {
        return Err(AppError::Network(format!("别名服务器 HTTP {}", resp.status())));
    }
    let body: Value = resp.json().await?;
    if body.get("message").is_some() {
        return Ok(SongsByAlias::None);
    }
    let items = body.get("data").and_then(Value::as_array).cloned().unwrap_or_default();
    if body.get("type").and_then(Value::as_str) == Some("ongoing")
        && items.first().is_some_and(|i| i.get("tag").is_some())
    {
        let votes = items
            .iter()
            .filter_map(|i| Some((i.get("tag")?.as_str()?.to_string(), i.get("song_id")?.as_u64()? as u32)))
            .collect();
        return Ok(SongsByAlias::Votes(votes));
    }
    Ok(SongsByAlias::Songs(items.into_iter().filter_map(|i| serde_json::from_value(i).ok()).collect()))
}
