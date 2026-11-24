//! MCP tool implementations. Each returns a [`Reply`]; errors become friendly text.

pub mod b50;
pub mod music;
pub mod progress;
pub mod score;
pub mod user;

use crate::core::models::{Alias, Song};
use crate::core::music::{Filter, MaiData};
use crate::draw;
use crate::error::{AppError, Result};
use base64::Engine;
use image::RgbaImage;
use rmcp::model::{CallToolResult, ContentBlock};

/// Tool output: text, or an image with a caption.
pub enum Reply {
    Text(String),
    Image {
        img: RgbaImage,
        prefix: &'static str,
        text: String,
    },
    /// A static image file sent as-is (e.g. the JPEG plate requirement chart).
    File {
        path: std::path::PathBuf,
        mime: &'static str,
        text: String,
    },
}

impl Reply {
    pub fn image(img: RgbaImage, prefix: &'static str, text: impl Into<String>) -> Self {
        Self::Image { img, prefix, text: text.into() }
    }

    /// Renders per `output_format`: `text` gives text only; anything else adds the image.
    pub fn into_result(self, format: Option<&str>) -> CallToolResult {
        let blocks = match self {
            Reply::Text(t) => vec![ContentBlock::text(t)],
            Reply::File { path, mime, text } => match std::fs::read(&path) {
                Ok(bytes) => vec![
                    ContentBlock::image(base64::engine::general_purpose::STANDARD.encode(bytes), mime),
                    ContentBlock::text(format!("{text}\n[图片路径: {}]", path.display())),
                ],
                Err(e) => vec![ContentBlock::text(format!("{text}\n（图片读取失败：{e}）"))],
            },
            Reply::Image { text, .. } if !wants_image(format) => vec![ContentBlock::text(text)],
            Reply::Image { img, prefix, text } => match draw::output(&img, prefix) {
                Ok((png, path)) => {
                    let b64 = base64::engine::general_purpose::STANDARD.encode(png);
                    vec![
                        ContentBlock::image(b64, "image/png"),
                        ContentBlock::text(format!("{text}\n[图片已保存: {}]", path.display())),
                    ]
                }
                Err(e) => vec![ContentBlock::text(format!("{text}\n（图片生成失败：{e}）"))],
            },
        };
        CallToolResult::success(blocks)
    }
}

/// Converts a tool outcome into an MCP result (errors are user-facing text, not protocol errors).
pub fn finish(r: Result<Reply>, format: Option<&str>) -> CallToolResult {
    match r {
        Ok(reply) => reply.into_result(format),
        Err(e) => CallToolResult::success(vec![ContentBlock::text(e.to_string())]),
    }
}

/// Runs CPU-heavy drawing off the async runtime.
pub async fn render<F>(f: F) -> Result<RgbaImage>
where
    F: FnOnce() -> Option<RgbaImage> + Send + 'static,
{
    tokio::task::spawn_blocking(f)
        .await
        .map_err(|e| AppError::Other(e.to_string()))?
        .ok_or_else(|| AppError::Msg("图片资源缺失，请检查静态资源或先调用 mai_update_data 生成表格。".into()))
}

pub fn wants_image(format: Option<&str>) -> bool {
    format.is_none_or(|f| !f.eq_ignore_ascii_case("text"))
}

pub fn require_qq(qq: Option<&str>) -> Result<&str> {
    qq.map(str::trim).filter(|q| !q.is_empty()).ok_or_else(|| AppError::Msg("请提供 QQ 号（qq 参数）。".into()))
}

pub fn ensure_data(d: &MaiData) -> Result<()> {
    if d.songs.is_empty() {
        return Err(AppError::Msg("曲目数据尚未加载完成，请稍后再试或调用 mai_update_data。".into()));
    }
    Ok(())
}

/// `「id」` padded to 7 chars + name, one per line.
pub fn id_lines<'a>(songs: impl Iterator<Item = (&'a u32, &'a String)>) -> String {
    songs.map(|(id, name)| format!("{:<7} {name}\n", format!("「{id}」"))).collect()
}

/// Upstream `search_song` used by `info` / `ginfo`: id, exact title, alias, then title search.
pub fn find_song(d: &MaiData, query: &str) -> Result<Song> {
    let q = query.trim().to_lowercase();
    if q.is_empty() {
        return Err(AppError::Msg("请输入曲目id或曲名".into()));
    }
    if let Some(s) = q.parse().ok().and_then(|id| d.by_id(id)) {
        return Ok(s.clone());
    }
    if let Some(s) = d.by_name(&q) {
        return Ok(s.clone());
    }
    let aliases: Vec<&Alias> = d.alias_by_alias(&q);
    match aliases.len() {
        0 => {}
        1 => {
            return d.by_id(aliases[0].song_id).cloned().ok_or_else(|| AppError::Msg("未找到曲目".into()));
        }
        _ => {
            let mut msg = String::from("找到相同别名的曲目，请使用以下ID查询：\n");
            for a in aliases {
                msg += &format!("{}：{}\n", a.song_id, a.alias.first().cloned().unwrap_or_default());
            }
            return Err(AppError::Msg(msg.trim().to_string()));
        }
    }
    let mut found = d.filter(&Filter { title: Some(&q), ..Default::default() });
    match found.len() {
        1 => Ok(found.remove(0)),
        2..=5 => {
            found.sort_by_key(|s| s.song_id);
            let list = id_lines(found.iter().map(|s| (&s.song_id, &s.song_name)));
            Err(AppError::Msg(format!(
                "未找到别名为「{query}」的歌曲，但找到「{}」个相似标题的曲目：\n{list}※ 请使用「info xxxxx」查询指定曲目",
                found.len()
            )))
        }
        0 => Err(AppError::Msg("未找到曲目".into())),
        _ => Err(AppError::Msg("查询相似的曲目过多，请缩小范围。".into())),
    }
}
