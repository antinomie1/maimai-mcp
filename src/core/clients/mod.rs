pub mod divingfish;
pub mod lxns;
pub mod yuzuchan;

use std::sync::LazyLock;
use std::time::Duration;

/// Shared HTTP client (connection pooling across all services).
pub static HTTP: LazyLock<reqwest::Client> = LazyLock::new(|| {
    reqwest::Client::builder()
        .timeout(Duration::from_secs(60))
        .user_agent(concat!("maimai-mcp/", env!("CARGO_PKG_VERSION")))
        .build()
        .unwrap_or_default()
});

/// Timeout for bulk data downloads (upstream uses 180s).
pub const BULK_TIMEOUT: Duration = Duration::from_secs(180);

/// Downloads a binary resource; `None` on any failure.
pub async fn fetch_bytes(url: &str) -> Option<Vec<u8>> {
    let resp = HTTP.get(url).send().await.ok()?.error_for_status().ok()?;
    resp.bytes().await.ok().map(|b| b.to_vec())
}

/// QQ avatar (upstream `qqlogo`).
pub async fn qq_avatar(qq: &str) -> Option<Vec<u8>> {
    fetch_bytes(&format!("https://q1.qlogo.cn/g?b=qq&nk={qq}&s=100")).await
}

/// LXNS collection asset (`plate` / `icon`) from the yuzuchan asset server.
pub async fn online_asset(kind: &str, id: u32) -> Option<Vec<u8>> {
    let file = format!("UI_{}_{id:06}.png", capitalize(kind));
    fetch_bytes(&format!("https://www.yuzuchan.moe/assets/maimaidx/{kind}/{file}")).await
}

pub fn capitalize(s: &str) -> String {
    let mut c = s.chars();
    c.next().map(|f| f.to_uppercase().chain(c).collect()).unwrap_or_default()
}
