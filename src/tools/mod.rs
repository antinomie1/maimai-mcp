pub mod b50;
pub mod fortune;
pub mod group_rank;
pub mod music;
pub mod progress;
pub mod random;
pub mod score;
pub mod user;

use crate::core::database::user_store::UserStore;
use crate::core::models::user::{ScoreSource, UserConfig};
use crate::draw::b50::B50Theme;
use crate::draw::process_and_save_image;
use image::RgbaImage;
use rmcp::model::{CallToolResult, ContentBlock};

pub struct ToolHelper;

impl ToolHelper {
    /// Resolves user configuration given optional QQ, username, and source overrides.
    pub fn resolve_user_config(
        qq: Option<&str>,
        username: Option<&str>,
        source: Option<&str>,
    ) -> (UserConfig, ScoreSource, B50Theme) {
        let qq_clean = qq.unwrap_or("").trim();
        let mut user_config = if !qq_clean.is_empty() {
            UserStore::get_user(qq_clean)
        } else {
            UserConfig::new(String::new())
        };

        if let Some(uname) = username {
            if !uname.trim().is_empty() {
                user_config.divingfish_username = Some(uname.trim().to_string());
            }
        }

        let resolved_source = if let Some(src_str) = source {
            ScoreSource::parse(src_str)
        } else {
            user_config.default_source
        };

        let theme = match user_config.theme.as_str() {
            "prism_plus" => B50Theme::PrismPlus,
            _ => B50Theme::Circle,
        };

        (user_config, resolved_source, theme)
    }

    /// Formats the tool result according to output_format ("image", "text", "both").
    pub fn format_result(
        img: Option<&RgbaImage>,
        img_prefix: &str,
        text_summary: &str,
        output_format: Option<&str>,
    ) -> CallToolResult {
        let fmt = output_format.unwrap_or("image").to_lowercase();
        let mut blocks = Vec::new();

        match fmt.as_str() {
            "text" => {
                blocks.push(ContentBlock::text(text_summary));
            }
            "both" => {
                if let Some(image_data) = img {
                    if let Ok((base64_str, path)) = process_and_save_image(image_data, img_prefix) {
                        blocks.push(ContentBlock::image(base64_str, "image/png"));
                        blocks.push(ContentBlock::text(format!("{text_summary}\n[图片已保存至: {}]", path.display())));
                    } else {
                        blocks.push(ContentBlock::text(text_summary));
                    }
                } else {
                    blocks.push(ContentBlock::text(text_summary));
                }
            }
            _ => {
                // "image" (default)
                if let Some(image_data) = img {
                    if let Ok((base64_str, path)) = process_and_save_image(image_data, img_prefix) {
                        blocks.push(ContentBlock::image(base64_str, "image/png"));
                        // Also attach brief caption with saved path for bot media sending
                        let first_line = text_summary.lines().next().unwrap_or(text_summary);
                        blocks.push(ContentBlock::text(format!("{first_line}\n[图片已保存: {}]", path.display())));
                    } else {
                        blocks.push(ContentBlock::text(text_summary));
                    }
                } else {
                    blocks.push(ContentBlock::text(text_summary));
                }
            }
        }

        CallToolResult::success(blocks)
    }
}

#[cfg(test)]
mod tests {
    use crate::core::database::alias_db::ALIAS_DB;
    use crate::core::database::music_db::MUSIC_DB;
    use crate::core::models::music::{BasicInfo, ChartInfo, Song, SongType};
    use crate::tools::fortune::{FortuneTool, MaiFortuneParams};
    use crate::tools::music::{MaiAliasQueryParams, MaiChartInfoParams, MaiSearchSongParams, MusicTools};
    use crate::tools::random::{MaiRandomSongParams, MaiWhatParams, RandomTools};
    use crate::tools::user::{MaiBindPlayerParams, MaiUserSettingsParams, UserTools};

    fn ensure_test_mock_data() {
        let mut db = MUSIC_DB.write().unwrap();
        if db.songs.is_empty() {
            let sample_song = Song {
                id: "8".to_string(),
                title: "True Love Song".to_string(),
                song_type: SongType::Standard,
                ds: vec![3.0, 5.0, 7.5, 13.0],
                level: vec!["3".into(), "5".into(), "7+".into(), "13".into()],
                cids: vec![8, 108, 208, 308],
                charts: vec![
                    ChartInfo { notes: vec![100, 10, 10, 10], charter: Some("Tester".into()) },
                    ChartInfo { notes: vec![200, 20, 20, 20], charter: Some("Tester".into()) },
                    ChartInfo { notes: vec![300, 30, 30, 30], charter: Some("Tester".into()) },
                    ChartInfo { notes: vec![500, 50, 50, 50], charter: Some("Tester".into()) },
                ],
                basic_info: BasicInfo {
                    title: "True Love Song".to_string(),
                    artist: "SEGA".to_string(),
                    genre: "maimai".to_string(),
                    bpm: 140,
                    version: "maimai".to_string(),
                    is_new: false,
                },
            };
            db.songs.push(sample_song);
            db.id_map.insert("8".to_string(), 0);
            db.title_map.entry("true love song".to_string()).or_default().push(0);
        }

        let mut alias_db = ALIAS_DB.write().unwrap();
        if alias_db.alias_to_song_ids.is_empty() {
            alias_db.alias_to_song_ids.insert("垃圾桶".to_string(), vec![8]);
            alias_db.song_id_to_aliases.insert(8, vec!["垃圾桶".to_string()]);
        }
    }

    #[tokio::test]
    async fn test_search_and_alias() {
        ensure_test_mock_data();
        let res = MusicTools::search_song(MaiSearchSongParams {
            keyword: "True Love".into(),
            limit: Some(5),
        }).await.unwrap();
        assert!(!res.content.is_empty());

        // Test search by alias directly
        let alias_search_res = MusicTools::search_song(MaiSearchSongParams {
            keyword: "垃圾桶".into(),
            limit: Some(5),
        }).await.unwrap();
        assert!(!alias_search_res.content.is_empty());

        // Test chart_info by alias directly
        let chart_by_alias = MusicTools::chart_info(MaiChartInfoParams {
            song_name: "垃圾桶".into(),
            theme: Some("circle".into()),
            output_format: Some("text".into()),
        }).await.unwrap();
        assert!(!chart_by_alias.content.is_empty());

        let alias_res = MusicTools::alias_query(MaiAliasQueryParams {
            song_name: "垃圾桶".into(),
        }).await.unwrap();
        assert!(!alias_res.content.is_empty());
    }

    #[tokio::test]
    async fn test_user_bind_and_settings() {
        let bind_res = UserTools::bind_player(MaiBindPlayerParams {
            qq: "12345678".into(),
            divingfish_username: Some("test_user".into()),
            lxns_token: None,
            lxns_friend_code: None,
            lxns_code: None,
            default_source: Some("divingfish".into()),
            theme: Some("circle".into()),
            request_oauth_link: None,
        }).await.unwrap();
        assert!(!bind_res.content.is_empty());

        let settings_res = UserTools::user_settings(MaiUserSettingsParams {
            qq: "12345678".into(),
        }).await.unwrap();
        assert!(!settings_res.content.is_empty());
    }

    #[tokio::test]
    async fn test_fortune_and_random() {
        ensure_test_mock_data();
        let fort = FortuneTool::execute(MaiFortuneParams {
            qq: Some("12345678".into()),
            output_format: Some("text".into()),
        }).await.unwrap();
        assert!(!fort.content.is_empty());

        let rand = RandomTools::random_song(MaiRandomSongParams {
            level: Some("13".into()),
            song_type: None,
            output_format: Some("text".into()),
        }).await.unwrap();
        assert!(!rand.content.is_empty());

        let what = RandomTools::mai_what(MaiWhatParams {
            output_format: Some("text".into()),
        }).await.unwrap();
        assert!(!what.content.is_empty());
    }

    #[tokio::test]
    async fn test_chart_info() {
        ensure_test_mock_data();
        let chart = MusicTools::chart_info(MaiChartInfoParams {
            song_name: "8".into(),
            theme: Some("circle".into()),
            output_format: Some("text".into()),
        }).await.unwrap();
        assert!(!chart.content.is_empty());
    }

    #[tokio::test]
    async fn test_divingfish_oauth_helpers() {
        use crate::core::clients::divingfish::{compute_subject_ref, mask_qq};
        let client_id = "test_client_id";
        let qq = "123456789";
        let sub_ref = compute_subject_ref(client_id, qq);
        assert_eq!(sub_ref.len(), 64);
        assert_eq!(mask_qq(qq), "QQ 12****89");
        assert_eq!(mask_qq("123"), "QQ ****");
    }

    #[tokio::test]
    async fn test_lxns_client_helpers() {
        use crate::core::clients::lxns::LxnsClient;
        // Check mapping and score conversion
        let combo = LxnsClient::map_combo(Some("ap"));
        assert_eq!(combo, crate::core::models::score::ComboStatus::AP);
        let sync = LxnsClient::map_sync(Some("fdxp"));
        assert_eq!(sync, crate::core::models::score::SyncStatus::FDXP);
    }
}

