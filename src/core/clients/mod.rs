pub mod divingfish;
pub mod lxns;
pub mod yuzuchan;

use crate::core::database::user_store::UserStore;
use crate::core::models::score::PlayerB50;
use crate::core::models::user::ScoreSource;
use crate::error::AppError;
use divingfish::DivingFishClient;
use lxns::LxnsClient;

pub struct ScoreClientManager {
    df: DivingFishClient,
    lxns: LxnsClient,
}

impl ScoreClientManager {
    pub fn new() -> Self {
        Self {
            df: DivingFishClient::new(),
            lxns: LxnsClient::new(),
        }
    }

    /// Query player B50 with smart fallback and priority
    pub async fn query_b50(
        &self,
        qq: Option<&str>,
        username: Option<&str>,
        source: Option<ScoreSource>,
    ) -> Result<PlayerB50, AppError> {
        // 1. Explicit username query
        if let Some(name) = username {
            return self.df.query_b50_by_username(name).await;
        }

        // 2. QQ based query
        let Some(qq_id) = qq else {
            return Err(AppError::InvalidInput("请提供 QQ 号或用户名进行查分".into()));
        };

        let user_cfg = UserStore::get_user(qq_id);
        let target_source = source.unwrap_or(user_cfg.default_source);

        match target_source {
            ScoreSource::DivingFish => {
                // Try DivingFish first
                match self.df.query_b50_by_qq(qq_id).await {
                    Ok(b50) => Ok(b50),
                    Err(e) => {
                        // Fallback to LXNS if user has OAuth / token / friend code configured
                        if let Ok(lx_b50) = self.lxns.query_b50_smart(Some(qq_id), user_cfg.lxns_friend_code, user_cfg.lxns_token.as_deref()).await {
                            return Ok(lx_b50);
                        }
                        Err(e)
                    }
                }
            }
            ScoreSource::Lxns => {
                self.lxns.query_b50_smart(Some(qq_id), user_cfg.lxns_friend_code, user_cfg.lxns_token.as_deref()).await
            }
        }
    }
}
