use maimai_mcp::core::calc::{compute_rate, compute_rating, dx_star};
use maimai_mcp::core::clients::divingfish::{binding_label, extract_confirmation_code, token_subject};
use maimai_mcp::core::clients::lxns::extract_authorization_code;
use maimai_mcp::core::constants::{badge_map, cb_sync_map, plate_cn, rank_map, version_map};
use maimai_mcp::core::models::{Difficulty, Notes, Service, Song, Theme, User};
use maimai_mcp::core::music::{Filter, MaiData};
use maimai_mcp::util::{Rng, civil_from_days, column_width, ellipsis, py_float, py_round};

#[test]
fn rating_matches_upstream() {
    assert_eq!(compute_rating(13.7, 100.5), 308);
    assert_eq!(compute_rating(13.7, 101.0), 308);
    assert_eq!(compute_rating(14.0, 100.0), 302);
    assert_eq!(compute_rating(10.0, 40.0), 28);
    assert_eq!(compute_rate(100.5), "SSSp");
    assert_eq!(compute_rate(99.0), "SS");
    assert_eq!(compute_rate(98.5), "Sp");
    assert_eq!(compute_rate(97.0), "S");
    assert_eq!(compute_rate(49.9), "D");
    assert_eq!(dx_star(84.9), 0);
    assert_eq!(dx_star(97.0), 5);
}

#[test]
fn python_formatting() {
    assert_eq!(py_float(180.0), "180.0");
    assert_eq!(py_float(13.7), "13.7");
    assert_eq!(py_round(13.456, 2), "13.46");
    assert_eq!(py_round(12.0, 2), "12.0");
    assert_eq!(column_width("舞萌ab"), 6);
    assert_eq!(ellipsis("abcdef", 4), "abc...");
    assert_eq!(ellipsis("abc", 4), "abc");
    assert_eq!(civil_from_days(0), (1970, 1, 1));
    assert_eq!(civil_from_days(20_781), (2026, 11, 24));
}

#[test]
fn rng_sample_is_unique() {
    let mut rng = Rng::new(42);
    let s = rng.sample(&[1, 2, 3, 4, 5, 6], 5);
    let mut sorted = s.clone();
    sorted.sort_unstable();
    sorted.dedup();
    assert_eq!(sorted.len(), 5);
}

#[test]
fn constant_maps() {
    assert_eq!(rank_map("sssp"), "SSSp");
    assert_eq!(rank_map("aa"), "AA");
    assert_eq!(badge_map("fdxp"), Some("FSDp"));
    assert_eq!(cb_sync_map("ap+"), Some(&["app"][..]));
    assert_eq!(plate_cn("晓"), "暁");
    assert_eq!(version_map("华").unwrap().1, "熊&华");
    assert_eq!(version_map("霸").unwrap().1, "舞");
    assert_eq!(version_map("鏡").unwrap(), (vec!["maimai でらっくす PRiSM"], "鏡"));
    assert!(version_map("晓").is_none());
}

#[test]
fn divingfish_oauth_helpers() {
    assert_eq!(binding_label("123456789"), "QQ 12*****89");
    assert_eq!(binding_label("1234"), "QQ 1234");
    assert_eq!(extract_confirmation_code("bcdf-ghjk-lmnp").as_deref(), Some("BCDF-GHJK-LMNP"));
    assert_eq!(extract_confirmation_code("确认码： BCDFGHJKLMNP").as_deref(), Some("BCDF-GHJK-LMNP"));
    assert_eq!(extract_confirmation_code("ABCD-GHJK-LMNP"), None);
    assert_eq!(extract_confirmation_code("hello world"), None);
    // header.{"sub":"df-42"}.sig
    assert_eq!(token_subject("x.eyJzdWIiOiJkZi00MiJ9.y").as_deref(), Some("df-42"));
}

#[test]
fn lxns_code_extraction() {
    assert_eq!(extract_authorization_code("JVJ6-VPTM-MGHZ").as_deref(), Some("JVJ6-VPTM-MGHZ"));
    assert_eq!(extract_authorization_code("授权码：abcdefghijklmnop1234").as_deref(), Some("abcdefghijklmnop1234"));
    assert_eq!(
        extract_authorization_code("https://x.example/cb?code=abcdefghijklmnop1234&state=1").as_deref(),
        Some("abcdefghijklmnop1234")
    );
    assert_eq!(extract_authorization_code("hi"), None);
}

fn song(id: u32, name: &str, levels: &[(&str, f64)]) -> Song {
    Song {
        song_id: id,
        song_name: name.into(),
        artist: "SEGA".into(),
        genre: "maimai".into(),
        bpm: 150.0,
        version_str: "maimai".into(),
        version_int: 0,
        kind: "SD".into(),
        isnew: false,
        difficulties: levels
            .iter()
            .enumerate()
            .map(|(i, (lv, ds))| Difficulty {
                level_index: i as u8,
                level: lv.to_string(),
                level_value: *ds,
                note_designer: "Tester".into(),
                notes: Notes::default(),
                dx_score: 300,
                stats: None,
            })
            .collect(),
        kanji: None,
        description: None,
        is_buddy: None,
    }
}

#[test]
fn music_filters_and_level_table() {
    let songs = vec![
        song(8, "True Love Song", &[("5", 5.0), ("7", 7.0), ("10", 10.0), ("13", 13.0)]),
        song(9, "Color My World", &[("6", 6.0), ("8", 8.0), ("11+", 11.7), ("13", 13.2)]),
    ];
    let d = MaiData::new(songs, vec![], Default::default());
    assert_eq!(d.by_id(9).unwrap().song_name, "Color My World");
    let lv13 = d.level_charts("13").unwrap();
    assert_eq!(lv13[0].0, "13.2");
    assert_eq!(lv13[1].0, "13.0");
    assert_eq!(d.level_data.first().unwrap().0, "13");
    let found = d.filter(&Filter { title: Some("love"), ..Default::default() });
    assert_eq!(found.len(), 1);
    let ds = d.filter(&Filter { level_value: Some((11.0, 12.0)), matched_diffs_only: true, ..Default::default() });
    assert_eq!(ds.len(), 1);
    assert_eq!(ds[0].difficulties.len(), 1);
    assert!(d.level_charts("5").is_none(), "levels below 7.0 are excluded");
}

#[test]
fn user_json_backward_compatible() {
    let old = r#"{"qq":"123","default_source":"lxns","theme":"circle","divingfish_username":"x"}"#;
    let u: User = serde_json::from_str(old).unwrap();
    assert_eq!(u.service, Service::Lxns);
    assert_eq!(u.theme, Theme::Circle);
    let fresh: User = serde_json::from_str(r#"{"qq":"1"}"#).unwrap();
    assert_eq!(fresh.theme, Theme::PrismPlus);
}
