use crate::config::CONFIG;
use crate::error::AppError;
use serde::Deserialize;
use std::collections::HashMap;
use std::fs;
use std::sync::{LazyLock, RwLock};

#[derive(Debug, Clone, Deserialize)]
pub struct AliasEntry {
    #[serde(alias = "SongID", alias = "song_id")]
    pub song_id: u32,
    #[serde(default, alias = "Name", alias = "name", alias = "song_name")]
    pub name: String,
    #[serde(default, alias = "Alias", alias = "alias", alias = "aliases")]
    pub aliases: Vec<String>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct LxnsAliasWrapper {
    #[serde(default)]
    pub aliases: Vec<AliasEntry>,
}

pub struct AliasDatabase {
    pub alias_to_song_ids: HashMap<String, Vec<u32>>,
    pub song_id_to_aliases: HashMap<u32, Vec<String>>,
}

impl AliasDatabase {
    pub fn load() -> Result<Self, AppError> {
        let candidates = [
            CONFIG.static_dir.join("data/merge_music_alias.json"),
            CONFIG.static_dir.join("data/music_alias.json"),
            CONFIG.data_dir.join("alias_cache.json"),
        ];

        let mut content = String::new();
        for p in &candidates {
            if p.exists() {
                if let Ok(c) = fs::read_to_string(p) {
                    if !c.trim().is_empty() && c.trim() != "[]" {
                        content = c;
                        break;
                    }
                }
            }
        }

        let entries: Vec<AliasEntry> = if !content.is_empty() {
            serde_json::from_str(&content)
                .or_else(|_| {
                    serde_json::from_str::<LxnsAliasWrapper>(&content).map(|w| w.aliases)
                })
                .unwrap_or_default()
        } else {
            Vec::new()
        };

        let mut alias_to_song_ids: HashMap<String, Vec<u32>> = HashMap::new();
        let mut song_id_to_aliases: HashMap<u32, Vec<String>> = HashMap::new();

        for entry in entries {
            let mut list = Vec::new();
            for a in entry.aliases {
                let trimmed = a.trim();
                if trimmed.is_empty() {
                    continue;
                }
                let lower = trimmed.to_lowercase();
                alias_to_song_ids
                    .entry(lower)
                    .or_default()
                    .push(entry.song_id);
                list.push(trimmed.to_string());
            }
            if !list.is_empty() {
                song_id_to_aliases.insert(entry.song_id, list);
            }
        }

        log::info!("Loaded {} song alias entries into AliasDatabase", song_id_to_aliases.len());
        Ok(Self {
            alias_to_song_ids,
            song_id_to_aliases,
        })
    }

    pub fn find_song_ids_by_alias(&self, query: &str) -> Vec<u32> {
        let q = query.trim().to_lowercase();
        if q.is_empty() {
            return Vec::new();
        }

        // 1. Exact match
        if let Some(ids) = self.alias_to_song_ids.get(&q) {
            return ids.clone();
        }

        // 2. Substring match
        let mut results = Vec::new();
        for (alias, ids) in &self.alias_to_song_ids {
            if alias.contains(&q) || q.contains(alias) {
                for &id in ids {
                    if !results.contains(&id) {
                        results.push(id);
                    }
                }
            }
        }

        if !results.is_empty() {
            return results;
        }

        // 3. Jaro-Winkler fuzzy match
        for (alias, ids) in &self.alias_to_song_ids {
            if strsim::jaro_winkler(alias, &q) > 0.82 {
                for &id in ids {
                    if !results.contains(&id) {
                        results.push(id);
                    }
                }
            }
        }

        results
    }

    pub fn get_aliases_for_song(&self, song_id: u32) -> Option<&Vec<String>> {
        self.song_id_to_aliases.get(&song_id)
    }
}

pub static ALIAS_DB: LazyLock<RwLock<AliasDatabase>> = LazyLock::new(|| {
    RwLock::new(AliasDatabase::load().unwrap_or_else(|_| AliasDatabase {
        alias_to_song_ids: HashMap::new(),
        song_id_to_aliases: HashMap::new(),
    }))
});
