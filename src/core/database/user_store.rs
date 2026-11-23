use crate::config::CONFIG;
use crate::core::models::user::UserConfig;
use crate::error::AppError;
use std::fs;
use std::path::PathBuf;

pub struct UserStore;

impl UserStore {
    fn user_file_path(qq: &str) -> PathBuf {
        CONFIG.data_dir.join("users").join(format!("{qq}.json"))
    }

    pub fn get_user(qq: &str) -> UserConfig {
        let path = Self::user_file_path(qq);
        if path.exists() {
            if let Ok(content) = fs::read_to_string(&path) {
                if let Ok(config) = serde_json::from_str(&content) {
                    return config;
                }
            }
        }
        UserConfig::new(qq.to_string())
    }

    pub fn save_user(config: &UserConfig) -> Result<(), AppError> {
        let path = Self::user_file_path(&config.qq);
        let content = serde_json::to_string_pretty(config)?;
        fs::write(path, content)?;
        Ok(())
    }
}
