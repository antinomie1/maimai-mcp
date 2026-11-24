use std::path::PathBuf;
use std::sync::LazyLock;

/// Global configuration loaded from environment variables (upstream `.env` names).
#[derive(Debug, Clone)]
pub struct Config {
    pub static_dir: PathBuf,
    pub data_dir: PathBuf,
    pub output_dir: PathBuf,
    pub bot_name: String,
    /// Fetch LXNS icons/plates from the asset server instead of `static/mai`.
    pub assets_online: bool,
    /// Use the `.cn` alias server mirror.
    pub alias_proxy: bool,
    /// Daily data update hour (UTC+8); `None` disables the scheduler.
    pub update_hour: Option<u32>,
    pub df_proxy: bool,
    pub df_client_id: Option<String>,
    pub df_client_secret: Option<String>,
    pub df_auth_url: String,
    pub df_scope: String,
    pub lxns_dev_token: Option<String>,
    pub lxns_client_id: Option<String>,
    pub lxns_client_secret: Option<String>,
    pub lxns_redirect_uri: Option<String>,
}

const DF_SCOPES: [&str; 6] = [
    "profile",
    "prober.profile.read",
    "prober.records.read",
    "prober.records.write",
    "chunithm.records.read",
    "chunithm.records.write",
];

fn env(keys: &[&str]) -> Option<String> {
    keys.iter().find_map(|k| std::env::var(k).ok()).map(|v| v.trim().to_string()).filter(|v| !v.is_empty())
}

fn env_bool(key: &str, default: bool) -> bool {
    env(&[key]).map_or(default, |v| matches!(v.to_lowercase().as_str(), "1" | "true" | "yes" | "on"))
}

/// Normalises `DIVINGFISH_SCOPE` into upstream's canonical order, dropping unknown names.
fn df_scope(raw: Option<String>) -> String {
    let wanted: Vec<String> =
        raw.unwrap_or_else(|| "prober.records.read".into()).split_whitespace().map(str::to_string).collect();
    for name in &wanted {
        if !DF_SCOPES.contains(&name.as_str()) {
            log::warn!("未知的 DivingFish scope: {name:?}；可选值：{}", DF_SCOPES.join(", "));
        }
    }
    let scope: Vec<&str> = DF_SCOPES.iter().copied().filter(|s| wanted.iter().any(|w| w == s)).collect();
    if scope.is_empty() { "prober.records.read".into() } else { scope.join(" ") }
}

impl Config {
    pub fn from_env() -> Self {
        let static_dir = env(&["MAIMAI_STATIC_DIR", "MAIMAIDX_PATH"]).map(PathBuf::from).unwrap_or_else(|| {
            let local = PathBuf::from("./static");
            if local.exists() { local } else { PathBuf::from("../maimaiDX/static") }
        });
        let data_dir = env(&["MAIMAI_DATA_DIR"]).map_or_else(|| PathBuf::from("./data"), PathBuf::from);
        let output_dir = env(&["MAIMAI_OUTPUT_DIR"]).map_or_else(|| PathBuf::from("./output"), PathBuf::from);
        let update_hour = match env(&["MAIMAI_UPDATE_HOUR"]).as_deref() {
            Some("off" | "none" | "-1") => None,
            Some(v) => v.parse().ok().filter(|h| *h < 24),
            None => Some(4),
        };

        for dir in [
            data_dir.join("users"),
            output_dir.clone(),
            static_dir.join("data"),
            static_dir.join("mai/rating_table"),
            static_dir.join("mai/plate_table"),
        ] {
            let _ = std::fs::create_dir_all(dir);
        }

        Self {
            static_dir,
            data_dir,
            output_dir,
            bot_name: env(&["MAIMAI_BOT_NAME", "BOT_NAME"]).unwrap_or_else(|| "Sakura".into()),
            assets_online: env_bool("ASSETS_ONLINE", true),
            alias_proxy: env_bool("MAIMAIDX_ALIAS_PROXY", false),
            update_hour,
            df_proxy: env_bool("DIVINGFISH_PROBER_PROXY", false),
            df_client_id: env(&["DIVINGFISH_CLIENT_ID"]),
            df_client_secret: env(&["DIVINGFISH_CLIENT_SECRET"]),
            df_auth_url: env(&["DIVINGFISH_AUTH_URL"])
                .unwrap_or_else(|| "https://auth.diving-fish.com".into())
                .trim_end_matches('/')
                .to_string(),
            df_scope: df_scope(env(&["DIVINGFISH_SCOPE"])),
            lxns_dev_token: env(&["LXNS_DEV_TOKEN", "LXNS_DEVELOPER_TOKEN"]),
            lxns_client_id: env(&["LX_CLIENT_ID", "LXNS_CLIENT_ID"]),
            lxns_client_secret: env(&["LX_CLIENT_SECRET", "LXNS_CLIENT_SECRET"]),
            lxns_redirect_uri: env(&["REDIRECT_URI", "LXNS_REDIRECT_URI"]),
        }
    }

    pub fn df_oauth_enabled(&self) -> bool {
        self.df_client_id.is_some() && self.df_client_secret.is_some()
    }

    pub fn lxns_oauth_enabled(&self) -> bool {
        self.lxns_client_id.is_some() && self.lxns_client_secret.is_some() && self.lxns_redirect_uri.is_some()
    }

    pub fn data_file(&self, name: &str) -> PathBuf {
        self.static_dir.join("data").join(name)
    }
}

pub static CONFIG: LazyLock<Config> = LazyLock::new(Config::from_env);
