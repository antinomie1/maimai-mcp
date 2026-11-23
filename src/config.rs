use std::path::{Path, PathBuf};
use std::sync::LazyLock;

/// Global configuration loaded from environment variables.
#[derive(Debug, Clone)]
pub struct Config {
    pub static_dir: PathBuf,
    pub data_dir: PathBuf,
    pub output_dir: PathBuf,
    pub divingfish_token: Option<String>,
    pub divingfish_client_id: Option<String>,
    pub divingfish_client_secret: Option<String>,
    pub lxns_token: Option<String>,
    pub lxns_client_id: Option<String>,
    pub lxns_client_secret: Option<String>,
    pub lxns_redirect_uri: Option<String>,
}

impl Config {
    pub fn from_env() -> Self {
        let static_dir = std::env::var("MAIMAI_STATIC_DIR")
            .map(PathBuf::from)
            .unwrap_or_else(|_| {
                let default_path = Path::new("./static");
                if default_path.exists() {
                    default_path.to_path_buf()
                } else {
                    PathBuf::from("../maimaiDX/static")
                }
            });

        let data_dir = std::env::var("MAIMAI_DATA_DIR")
            .map(PathBuf::from)
            .unwrap_or_else(|_| PathBuf::from("./data"));

        let output_dir = std::env::var("MAIMAI_OUTPUT_DIR")
            .map(PathBuf::from)
            .unwrap_or_else(|_| PathBuf::from("./output"));

        let divingfish_token = std::env::var("DIVINGFISH_DEVELOPER_TOKEN").ok();
        let divingfish_client_id = std::env::var("DIVINGFISH_CLIENT_ID").ok();
        let divingfish_client_secret = std::env::var("DIVINGFISH_CLIENT_SECRET").ok();
        let lxns_token = std::env::var("LXNS_DEVELOPER_TOKEN")
            .or_else(|_| std::env::var("LXNS_DEV_TOKEN"))
            .or_else(|_| std::env::var("LXNS_TOKEN"))
            .ok();
        let lxns_client_id = std::env::var("LXNS_CLIENT_ID")
            .or_else(|_| std::env::var("LX_CLIENT_ID"))
            .ok();
        let lxns_client_secret = std::env::var("LXNS_CLIENT_SECRET")
            .or_else(|_| std::env::var("LX_CLIENT_SECRET"))
            .ok();
        let lxns_redirect_uri = std::env::var("LXNS_REDIRECT_URI")
            .or_else(|_| std::env::var("LX_REDIRECT_URI"))
            .ok();

        // Ensure directories exist
        let _ = std::fs::create_dir_all(&data_dir);
        let _ = std::fs::create_dir_all(data_dir.join("users"));
        let _ = std::fs::create_dir_all(&output_dir);

        Self {
            static_dir,
            data_dir,
            output_dir,
            divingfish_token,
            divingfish_client_id,
            divingfish_client_secret,
            lxns_token,
            lxns_client_id,
            lxns_client_secret,
            lxns_redirect_uri,
        }
    }
}

pub static CONFIG: LazyLock<Config> = LazyLock::new(Config::from_env);
