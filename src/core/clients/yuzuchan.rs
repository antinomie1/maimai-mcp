use crate::error::AppError;
use reqwest::Client;
use serde::Deserialize;
use std::collections::HashMap;
use std::time::Duration;

#[derive(Debug, Deserialize)]
pub struct YuzuAliasResponse {
    pub status: Option<String>,
    pub message: Option<String>,
    pub content: Option<HashMap<String, Vec<String>>>,
}

pub struct YuzuchanClient {
    client: Client,
}

impl YuzuchanClient {
    pub fn new() -> Self {
        let client = Client::builder()
            .timeout(Duration::from_secs(15))
            .build()
            .unwrap_or_default();
        Self { client }
    }

    /// Fetch latest aliases mapping from Yuzuchan API
    pub async fn fetch_all_aliases(&self) -> Result<HashMap<String, Vec<String>>, AppError> {
        let url = "https://api.yuzuchan.moe/maimaidx/prober/alias";
        let resp = self.client.get(url).send().await?;
        if !resp.status().is_success() {
            return Err(AppError::Network(format!("柚子别名接口返回状态码 {}", resp.status())));
        }

        let body: YuzuAliasResponse = resp.json().await?;
        body.content.ok_or_else(|| AppError::NotFound("未获取到柚子别名数据".into()))
    }
}
