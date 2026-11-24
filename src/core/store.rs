//! Per-QQ user configuration (`data/users/<qq>.json`) and pending OAuth sessions.

use crate::config::CONFIG;
use crate::core::models::{Service, User};
use crate::error::{AppError, Result};
use std::collections::HashMap;
use std::path::PathBuf;
use std::sync::{LazyLock, Mutex};
use std::time::{Duration, Instant};

fn user_path(qq: &str) -> PathBuf {
    CONFIG.data_dir.join("users").join(format!("{qq}.json"))
}

fn valid_qq(qq: &str) -> Result<&str> {
    let qq = qq.trim();
    if qq.is_empty() || !qq.chars().all(|c| c.is_ascii_digit()) {
        return Err(AppError::Msg("请提供正确的 QQ 号。".into()));
    }
    Ok(qq)
}

/// Loads a user, creating a default record when absent (upstream `GetOrCreateUser`).
pub fn get_user(qq: &str) -> Result<User> {
    let qq = valid_qq(qq)?;
    let user = std::fs::read(user_path(qq))
        .ok()
        .and_then(|b| serde_json::from_slice::<User>(&b).ok())
        .unwrap_or_else(|| User { qq: qq.to_string(), ..Default::default() });
    Ok(User { qq: qq.to_string(), ..user })
}

pub fn save_user(user: &User) -> Result<()> {
    let qq = valid_qq(&user.qq)?;
    std::fs::write(user_path(qq), serde_json::to_vec_pretty(user)?)?;
    Ok(())
}

/// Applies `f` to the stored user and saves it.
pub fn update_user(qq: &str, f: impl FnOnce(&mut User)) -> Result<User> {
    let mut user = get_user(qq)?;
    f(&mut user);
    save_user(&user)?;
    Ok(user)
}

/// Users waiting to send back an OAuth code: `qq -> (service, deadline)`.
static PENDING: LazyLock<Mutex<HashMap<String, (Service, Instant)>>> = LazyLock::new(Default::default);

pub fn pending_start(qq: &str, service: Service, ttl: Duration) {
    if let Ok(mut map) = PENDING.lock() {
        let now = Instant::now();
        map.retain(|_, (_, deadline)| *deadline > now);
        map.insert(qq.to_string(), (service, now + ttl));
    }
}

pub fn pending_is_active(qq: &str, service: Service) -> bool {
    PENDING.lock().is_ok_and(|map| map.get(qq).is_some_and(|(s, d)| *s == service && *d > Instant::now()))
}

pub fn pending_clear(qq: &str) {
    if let Ok(mut map) = PENDING.lock() {
        map.remove(qq);
    }
}
