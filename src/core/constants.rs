//! Game constants ported from upstream `constants.py`.

pub const FORTUNE: [&str; 13] =
    ["拼机", "推分", "越级", "下埋", "夜勤", "练底力", "练手法", "打旧框", "干饭", "抓绝赞", "收歌", "打大歌", "推AP"];

pub const RANK_SP: [&str; 14] = ["d", "c", "b", "bb", "bbb", "a", "aa", "aaa", "s", "sp", "ss", "ssp", "sss", "sssp"];
pub const RANK_PLUS: [&str; 14] = ["d", "c", "b", "bb", "bbb", "a", "aa", "aaa", "s", "s+", "ss", "ss+", "sss", "sss+"];
pub const STATISTICS_KEYS: [&str; 16] =
    ["clear", "s", "sp", "ss", "ssp", "sss", "sssp", "sync", "fc", "fcp", "ap", "app", "fs", "fsp", "fsd", "fsdp"];
pub const COMBO_SP: [&str; 4] = ["fc", "fcp", "ap", "app"];
pub const COMBO_PLUS: [&str; 4] = ["fc", "fc+", "ap", "ap+"];
pub const SYNC_D_SP: [&str; 4] = ["fs", "fsp", "fsd", "fsdp"];
pub const SYNC_SP: [&str; 4] = ["fs", "fsp", "fdx", "fdxp"];
pub const SYNC_PLUS: [&str; 4] = ["fs", "fs+", "fdx", "fdx+"];
pub const DIFFS: [&str; 5] = ["Basic", "Advanced", "Expert", "Master", "Re:Master"];
pub const LEVEL_LIST: [&str; 23] = [
    "1", "2", "3", "4", "5", "6", "7", "7+", "8", "8+", "9", "9+", "10", "10+", "11", "11+", "12", "12+", "13", "13+",
    "14", "14+", "15",
];
pub const ACHIEVEMENT_LIST: [f64; 13] =
    [50.0, 60.0, 70.0, 75.0, 80.0, 90.0, 94.0, 97.0, 98.0, 99.0, 99.5, 100.0, 100.5];

/// `RANK_MAP`: `sssp` -> `SSSp`, `aa` -> `AA`.
pub fn rank_map(rate: &str) -> String {
    match rate.strip_suffix('p') {
        Some(head) => format!("{}p", head.to_uppercase()),
        None => rate.to_uppercase(),
    }
}

/// `COMBO_MAP | SYNC_MAP`: icon suffix for a fc/fs value.
pub fn badge_map(v: &str) -> Option<&'static str> {
    Some(match v {
        "fc" => "FC",
        "fcp" => "FCp",
        "ap" => "AP",
        "app" => "APp",
        "fs" => "FS",
        "fsp" => "FSp",
        "fsd" | "fdx" => "FSD",
        "fsdp" | "fdxp" => "FSDp",
        "sync" => "Sync",
        _ => return None,
    })
}

/// `CB_SYNC_MAP`: completion-table plan -> accepted fc/fs values.
pub fn cb_sync_map(plan: &str) -> Option<&'static [&'static str]> {
    Some(match plan {
        "fc" => &["fc", "fcp", "ap", "app"],
        "fc+" => &["fcp", "ap", "app"],
        "ap" => &["ap", "app"],
        "ap+" => &["app"],
        "fs" => &["fs", "fsp", "fsd", "fsdp"],
        "fs+" => &["fsp", "fsd", "fsdp"],
        "fdx" => &["fsd", "fsdp"],
        "fdx+" => &["fsdp"],
        _ => return None,
    })
}

pub const SD_VERSION: [(&str, &str); 17] = [
    ("初", "maimai"),
    ("真", "maimai PLUS"),
    ("超", "maimai GreeN"),
    ("檄", "maimai GreeN PLUS"),
    ("橙", "maimai ORANGE"),
    ("暁", "maimai ORANGE PLUS"),
    ("晓", "maimai ORANGE PLUS"),
    ("桃", "maimai PiNK"),
    ("櫻", "maimai PiNK PLUS"),
    ("樱", "maimai PiNK PLUS"),
    ("紫", "maimai MURASAKi"),
    ("菫", "maimai MURASAKi PLUS"),
    ("堇", "maimai MURASAKi PLUS"),
    ("白", "maimai MiLK"),
    ("雪", "MiLK PLUS"),
    ("輝", "maimai FiNALE"),
    ("辉", "maimai FiNALE"),
];
pub const DX_ONLY_VERSION: [(&str, &str); 14] = [
    ("熊", "maimai でらっくす"),
    ("華", "maimai でらっくす PLUS"),
    ("华", "maimai でらっくす PLUS"),
    ("爽", "maimai でらっくす Splash"),
    ("煌", "maimai でらっくす Splash PLUS"),
    ("宙", "maimai でらっくす UNiVERSE"),
    ("星", "maimai でらっくす UNiVERSE PLUS"),
    ("祭", "maimai でらっくす FESTiVAL"),
    ("祝", "maimai でらっくす FESTiVAL PLUS"),
    ("双", "maimai でらっくす BUDDiES"),
    ("宴", "maimai でらっくす BUDDiES PLUS"),
    ("镜", "maimai でらっくす PRiSM"),
    ("鏡", "maimai でらっくす PRiSM"),
    ("彩", "maimai でらっくす PRiSM PLUS"),
];

/// `DX_VERSION` entries in upstream insertion order.
pub fn dx_version() -> impl Iterator<Item = (&'static str, &'static str)> {
    SD_VERSION.iter().chain(DX_ONLY_VERSION.iter()).copied()
}

pub fn dx_version_name(key: &str) -> Option<&'static str> {
    dx_version().find(|(k, _)| *k == key).map(|(_, v)| v)
}

/// `DX_CN_VERSION`: (CN title, plate key, version name).
pub const DX_CN_VERSION: [(&str, &str, &str); 7] = [
    ("舞萌DX", "熊&华", "maimai でらっくす"),
    ("舞萌DX 2021", "爽&煌", "maimai でらっくす Splash"),
    ("舞萌DX 2022", "宙&星", "maimai でらっくす UNiVERSE"),
    ("舞萌DX 2023", "祭&祝", "maimai でらっくす FESTiVAL"),
    ("舞萌DX 2024", "双&宴", "maimai でらっくす BUDDiES"),
    ("舞萌DX 2025", "鏡", "maimai でらっくす PRiSM"),
    ("舞萌DX 2026", "彩", "maimai でらっくす PRiSM PLUS"),
];

/// Newest version name (`list(DX_CN_VERSION.values())[-1][-1]`).
pub fn new_version_name() -> &'static str {
    DX_CN_VERSION[DX_CN_VERSION.len() - 1].2
}

/// Newest plate key (`list(DX_CN_VERSION.values())[-1][0]`).
pub fn new_version_plate() -> &'static str {
    DX_CN_VERSION[DX_CN_VERSION.len() - 1].1
}

/// `ALL_VERSION`: unique version names in order.
pub fn all_versions() -> Vec<&'static str> {
    let mut out: Vec<&str> = Vec::new();
    for (_, v) in dx_version() {
        if !out.contains(&v) {
            out.push(v);
        }
    }
    out
}

/// `VERSION_MAP`: plate char -> (version names for querying, plate data key).
pub fn version_map(key: &str) -> Option<(Vec<&'static str>, &'static str)> {
    let sd = |k: &str| dx_version_name(k).unwrap_or_default();
    Some(match key {
        "真" => (vec![sd("真"), sd("初")], "真"),
        "霸" | "舞" => {
            let mut names: Vec<&str> = SD_VERSION.iter().map(|(_, v)| *v).collect();
            names.dedup();
            (names, "舞")
        }
        "熊" | "华" | "華" => (vec![sd("熊")], "熊&华"),
        "爽" | "煌" => (vec![sd("爽")], "爽&煌"),
        "宙" | "星" => (vec![sd("宙")], "宙&星"),
        "祭" | "祝" => (vec![sd("祭")], "祭&祝"),
        "双" | "宴" => (vec![sd("双")], "双&宴"),
        "超" | "檄" | "橙" | "暁" | "桃" | "櫻" | "紫" | "菫" | "白" | "雪" | "輝" | "镜" | "鏡" | "彩" =>
        {
            let name =
                SD_VERSION.iter().chain(DX_ONLY_VERSION.iter()).find(|(k, _)| *k == key).map(|(k, v)| (*k, *v))?;
            (vec![name.1], name.0)
        }
        _ => return None,
    })
}

/// `PLATE_CN`: simplified -> plate file character.
pub fn plate_cn(v: &str) -> &str {
    match v {
        "晓" => "暁",
        "樱" => "櫻",
        "堇" => "菫",
        "辉" => "輝",
        "华" => "華",
        "镜" => "鏡",
        _ => v,
    }
}

/// `CATEGORY`: genre -> info badge file suffix.
pub fn category(genre: &str) -> &str {
    match genre {
        "流行&动漫" | "POPSアニメ" => "anime",
        "舞萌" | "maimai" => "maimai",
        "niconico & VOCALOID" | "niconicoボーカロイド" => "niconico",
        "东方Project" | "東方Project" => "touhou",
        "其他游戏" | "ゲームバラエティ" => "game",
        "音击&中二节奏" | "オンゲキCHUNITHM" => "ongeki",
        "宴会場" => "宴会场",
        other => other,
    }
}

/// Maps a difficulty colour / name to its index.
pub fn parse_difficulty(s: &str) -> Option<usize> {
    let s = s.trim().to_lowercase();
    Some(match s.as_str() {
        "绿" | "basic" | "bas" | "0" => 0,
        "黄" | "advanced" | "adv" | "1" => 1,
        "红" | "expert" | "exp" | "2" => 2,
        "紫" | "master" | "mas" | "3" => 3,
        "白" | "re:master" | "remaster" | "rem" | "4" => 4,
        _ => return None,
    })
}
