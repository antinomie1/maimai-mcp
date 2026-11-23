use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum ScoreSource {
    DivingFish,
    Lxns,
}

impl Default for ScoreSource {
    fn default() -> Self {
        Self::DivingFish
    }
}

impl ScoreSource {
    pub fn parse(s: &str) -> Self {
        match s.to_lowercase().as_str() {
            "lxns" | "落雪" => Self::Lxns,
            _ => Self::DivingFish,
        }
    }
}

/// User local configuration stored by QQ ID
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UserConfig {
    pub qq: String,
    #[serde(default)]
    pub default_source: ScoreSource,
    #[serde(default)]
    pub divingfish_username: Option<String>,
    #[serde(default)]
    pub lxns_token: Option<String>,
    #[serde(default)]
    pub lxns_friend_code: Option<u64>,
    #[serde(default)]
    pub lxns_access_token: Option<String>,
    #[serde(default)]
    pub lxns_refresh_token: Option<String>,
    #[serde(default)]
    pub lxns_token_expires_at: Option<u64>,
    #[serde(default = "default_theme")]
    pub theme: String,
}

fn default_theme() -> String {
    "circle".to_string()
}

impl UserConfig {
    pub fn new(qq: String) -> Self {
        Self {
            qq,
            default_source: ScoreSource::DivingFish,
            divingfish_username: None,
            lxns_token: None,
            lxns_friend_code: None,
            lxns_access_token: None,
            lxns_refresh_token: None,
            lxns_token_expires_at: None,
            theme: default_theme(),
        }
    }
}
