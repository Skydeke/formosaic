//! Level management: local storage, metadata, and level registry.
//!
//! Levels come from two sources:
//!
//! 1. **Online** — fetched from <https://poly.pizza/explore> at runtime.
//!    The model file + credits are saved locally so the level can be replayed
//!    without a network connection.
//!
//! 2. **Offline** — levels that were previously downloaded and saved to
//!    `{data_dir}/levels/`.
//!
//! Each level is stored as:
//!
//! ```
//! {data_dir}/levels/{id}/
//!     model.glb        (or .fbx / .obj)
//!     meta.json        (LevelMeta serialised as JSON)
//! ```
//!
//! `meta.json` format:
//! ```json
//! {
//!   "id":       "7S5Snphkam",
//!   "name":     "Cactus",
//!   "author":   "SoyMaria",
//!   "license":  "CC-BY",
//!   "source_url": "https://poly.pizza/m/7S5Snphkam",
//!   "model_file": "model.glb",
//!   "best_time_secs": null,
//!   "play_count": 0,
//!   "difficulty": 0.42
//! }
//! ```

use std::path::{Path, PathBuf};

// ─── Types ────────────────────────────────────────────────────────────────────

/// Metadata stored alongside every downloaded level.
#[derive(Debug, Clone)]
pub struct LevelMeta {
    /// Unique identifier (e.g. poly.pizza model ID).
    pub id: String,
    /// Human-readable name.
    pub name: String,
    /// Author / creator name.
    pub author: String,
    /// License string (e.g. "CC-BY 3.0").
    pub license: String,
    /// Original source URL.
    pub source_url: String,
    /// Filename of the model within the level directory.
    pub model_file: String,
    /// Best completion time in seconds (None if never completed).
    pub best_time_secs: Option<f32>,
    /// Number of times this level has been played.
    pub play_count: u32,
    /// Entropy-derived difficulty [0,1].
    pub difficulty: f32,
}

impl LevelMeta {
    /// Attribution line for display / credits screen.
    pub fn attribution(&self) -> String {
        format!(
            "{} by {} [{}] via Poly Pizza ({})",
            self.name, self.author, self.license, self.source_url
        )
    }

    /// Serialise to a simple JSON string (no external crate required).
    pub fn to_json(&self) -> String {
        let best = match self.best_time_secs {
            Some(t) => format!("{:.2}", t),
            None    => "null".to_string(),
        };
        format!(
            r#"{{"id":"{id}","name":"{name}","author":"{author}","license":"{lic}","source_url":"{url}","model_file":"{mf}","best_time_secs":{best},"play_count":{pc},"difficulty":{diff:.4}}}"#,
            id   = self.id,
            name = self.name,
            author = self.author,
            lic  = self.license,
            url  = self.source_url,
            mf   = self.model_file,
            best = best,
            pc   = self.play_count,
            diff = self.difficulty,
        )
    }

    /// Parse from a JSON string produced by `to_json`.
    pub fn from_json(s: &str) -> Option<Self> {
        fn extract<'a>(json: &'a str, key: &str) -> Option<&'a str> {
            let needle = format!("\"{}\":\"", key);
            let start  = json.find(needle.as_str())? + needle.len();
            let end    = json[start..].find('"')? + start;
            Some(&json[start..end])
        }
        fn extract_num(json: &str, key: &str) -> Option<f32> {
            let needle = format!("\"{}\":", key);
            let start  = json.find(needle.as_str())? + needle.len();
            let rest   = &json[start..];
            let end    = rest.find(|c: char| !c.is_ascii_digit() && c != '.' && c != '-').unwrap_or(rest.len());
            rest[..end].parse().ok()
        }

        let best_time_secs = {
            let needle = "\"best_time_secs\":";
            if let Some(pos) = s.find(needle) {
                let rest = &s[pos + needle.len()..];
                if rest.trim_start().starts_with("null") {
                    None
                } else {
                    let end = rest.find(|c: char| !c.is_ascii_digit() && c != '.').unwrap_or(rest.len());
                    rest[..end].trim().parse::<f32>().ok()
                }
            } else {
                None
            }
        };

        Some(LevelMeta {
            id:          extract(s, "id")?.to_string(),
            name:        extract(s, "name")?.to_string(),
            author:      extract(s, "author")?.to_string(),
            license:     extract(s, "license")?.to_string(),
            source_url:  extract(s, "source_url")?.to_string(),
            model_file:  extract(s, "model_file")?.to_string(),
            best_time_secs,
            play_count:  extract_num(s, "play_count").unwrap_or(0.0) as u32,
            difficulty:  extract_num(s, "difficulty").unwrap_or(0.5),
        })
    }
}

// ─── Registry ────────────────────────────────────────────────────────────────

/// In-memory registry of all locally available levels.
pub struct LevelRegistry {
    pub levels: Vec<LevelMeta>,
    base_dir: PathBuf,
}

impl LevelRegistry {
    /// Load all levels from `{base_dir}/levels/`.
    pub fn load(base_dir: &Path) -> Self {
        let levels_dir = base_dir.join("levels");
        let mut levels = Vec::new();

        if let Ok(entries) = std::fs::read_dir(&levels_dir) {
            for entry in entries.flatten() {
                let meta_path = entry.path().join("meta.json");
                if let Ok(json) = std::fs::read_to_string(&meta_path) {
                    if let Some(meta) = LevelMeta::from_json(&json) {
                        levels.push(meta);
                    }
                }
            }
        }

        // Sort by difficulty ascending so easy levels come first.
        levels.sort_by(|a, b| a.difficulty.partial_cmp(&b.difficulty).unwrap());

        Self { levels, base_dir: base_dir.to_path_buf() }
    }

    /// Path to the model file for a level.
    pub fn model_path(&self, meta: &LevelMeta) -> PathBuf {
        self.base_dir
            .join("levels")
            .join(&meta.id)
            .join(&meta.model_file)
    }

    /// Save a new level (model bytes + meta) to disk.
    pub fn save_level(
        &mut self,
        meta: LevelMeta,
        model_bytes: &[u8],
    ) -> Result<(), std::io::Error> {
        let dir = self.base_dir.join("levels").join(&meta.id);
        std::fs::create_dir_all(&dir)?;

        let model_path = dir.join(&meta.model_file);
        std::fs::write(&model_path, model_bytes)?;

        let meta_path = dir.join("meta.json");
        std::fs::write(&meta_path, meta.to_json())?;

        // Add or update in-memory list.
        if let Some(existing) = self.levels.iter_mut().find(|l| l.id == meta.id) {
            *existing = meta;
        } else {
            self.levels.push(meta);
        }
        Ok(())
    }

    /// Update the best time and increment play count for a level.
    pub fn record_completion(&mut self, id: &str, time_secs: f32) {
        if let Some(meta) = self.levels.iter_mut().find(|l| l.id == id) {
            meta.play_count += 1;
            meta.best_time_secs = Some(match meta.best_time_secs {
                Some(prev) => prev.min(time_secs),
                None       => time_secs,
            });
            // Persist update.
            let dir  = self.base_dir.join("levels").join(&meta.id);
            let path = dir.join("meta.json");
            let _    = std::fs::write(path, meta.to_json());
        }
    }

    /// Returns a random level meta (for the "Random Level" button).
    pub fn random_level(&self) -> Option<&LevelMeta> {
        use rand::Rng;
        if self.levels.is_empty() { return None; }
        let idx = rand::rng().random_range(0..self.levels.len());
        Some(&self.levels[idx])
    }

    /// Platform-appropriate data directory.
    pub fn default_data_dir() -> PathBuf {
        #[cfg(target_os = "android")]
        {
            // On Android, use the internal files directory.
            PathBuf::from("/data/user/0/com.formosaic.game/files")
        }
        #[cfg(not(target_os = "android"))]
        {
            dirs_or_home().join("formosaic")
        }
    }
}

#[cfg(not(target_os = "android"))]
fn dirs_or_home() -> PathBuf {
    // Try $XDG_DATA_HOME, then ~/.local/share, then ~/.
    if let Ok(xdg) = std::env::var("XDG_DATA_HOME") {
        return PathBuf::from(xdg);
    }
    if let Some(home) = std::env::var("HOME").ok() {
        let xdg = PathBuf::from(&home).join(".local").join("share");
        if xdg.exists() { return xdg; }
        return PathBuf::from(home);
    }
    PathBuf::from(".")
}
