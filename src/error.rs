use crate::config::CONFIG;
use std::fmt;

/// Errors with upstream's user-facing messages (`handler_error.py`).
#[derive(Debug)]
pub enum AppError {
    DfNotAuthorized,
    DfTooManyRequests,
    DfOAuth,
    DfUserNotFound,
    UserNotExists,
    DfQueryDisabled,
    DfBindingMismatch,
    DfConfirmationCode,
    /// OAuth app not configured / rejected credentials.
    DfToken,
    LxToken,
    LxPermission,
    LxNotFound,
    LxTooManyRequests,
    LxParams,
    LxOAuth,
    LxNotBound,
    MusicNotPlay,
    NoRecommendation,
    /// Plain user-facing message.
    Msg(String),
    Network(String),
    Other(String),
}

impl fmt::Display for AppError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let bot = &CONFIG.bot_name;
        match self {
            Self::DfNotAuthorized => write!(
                f,
                "您尚未授权「{bot} BOT」访问您的水鱼查分器成绩，或已取消授权。\n请发送「绑定水鱼」，按提示完成授权后再试。"
            ),
            Self::DfTooManyRequests => f.write_str("水鱼查分器请求次数已达上限，请稍后再试。"),
            Self::DfOAuth => f.write_str("水鱼账号服务暂时不可用，请稍后再试。"),
            Self::DfUserNotFound => f.write_str(
                "未在水鱼查分器找到此玩家，请确保此玩家的用户名和查分器中的用户名相同。\n如未绑定，请前往查分器官网进行绑定。\nhttps://www.diving-fish.com/maimaidx/prober/",
            ),
            Self::UserNotExists => f.write_str("查询的用户不存在。"),
            Self::DfQueryDisabled => f.write_str("该用户禁止了其他人获取数据或未同意用户协议。"),
            Self::DfBindingMismatch => f.write_str("您发送的授权码不属于您。"),
            Self::DfConfirmationCode => f.write_str("水鱼确认码无效、已过期或已被使用。"),
            Self::DfToken => f.write_str("请联系BOT管理员检查水鱼查分器相关信息，暂时无法查询。"),
            Self::LxToken => f.write_str("落雪查分器授权错误，请尝试重新绑定授权。"),
            Self::LxPermission => f.write_str("使用落雪查分器的权限不足，请联系BOT管理员检查相关信息。"),
            Self::LxNotFound => f.write_str("未找到落雪查分器相关资源，请联系BOT管理员检查相关信息。"),
            Self::LxTooManyRequests => f.write_str("使用落雪查分器的请求次数过多，请稍后再试。"),
            Self::LxParams => f.write_str("使用落雪查分器请求时发生错误，请联系BOT管理员检查相关信息。"),
            Self::LxOAuth => f.write_str("落雪查分器授权错误，请重试，依旧错误请重新绑定授权。"),
            Self::LxNotBound => write!(
                f,
                "您尚未授权「{bot} BOT」访问您的落雪查分器数据，请先使用「lxbind」指令进行绑定。"
            ),
            Self::MusicNotPlay => f.write_str("您未游玩过曲目。"),
            Self::NoRecommendation => f.write_str("没有乐曲推荐呢。可能是您太强了。"),
            Self::Msg(m) => f.write_str(m),
            Self::Network(m) => write!(f, "网络请求错误：{m}"),
            Self::Other(m) => write!(f, "发生未知错误：{m}\n请联系BOT管理员。"),
        }
    }
}

impl std::error::Error for AppError {}

impl From<std::io::Error> for AppError {
    fn from(err: std::io::Error) -> Self {
        Self::Other(err.to_string())
    }
}

impl From<serde_json::Error> for AppError {
    fn from(err: serde_json::Error) -> Self {
        Self::Other(format!("JSON: {err}"))
    }
}

impl From<reqwest::Error> for AppError {
    fn from(err: reqwest::Error) -> Self {
        Self::Network(err.to_string())
    }
}

pub type Result<T> = std::result::Result<T, AppError>;
