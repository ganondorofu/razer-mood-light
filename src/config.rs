use serde::{Deserialize, Serialize};
use std::fs;
use std::path::PathBuf;

fn default_color_compacting() -> String {
    "#00FFFF".to_string()
}

#[derive(Serialize, Deserialize, Clone)]
pub struct Config {
    pub color_generating: String,
    pub color_idle: String,
    pub color_waiting: String,
    #[serde(default = "default_color_compacting")]
    pub color_compacting: String,
    pub breath_period_ms: u64,
    pub breath_min: f64,
    pub breath_step_ms: u64,
}

impl Default for Config {
    fn default() -> Self {
        Config {
            color_generating: "#FF0000".to_string(),
            color_idle: "#00FF00".to_string(),
            color_waiting: "#FFFF00".to_string(),
            color_compacting: default_color_compacting(),
            breath_period_ms: 3000,
            breath_min: 0.15,
            breath_step_ms: 100,
        }
    }
}

fn config_path() -> PathBuf {
    let appdata = std::env::var("APPDATA").unwrap_or_else(|_| ".".to_string());
    PathBuf::from(appdata)
        .join("ClaudeMoodLight")
        .join("config.json")
}

/// Loads the config file, creating it with defaults on first run so the user
/// has something to edit. Falls back to defaults (without overwriting) if the
/// existing file fails to parse.
pub fn load_or_create() -> Config {
    let path = config_path();
    if let Ok(text) = fs::read_to_string(&path) {
        match serde_json::from_str::<Config>(&text) {
            Ok(cfg) => return cfg,
            Err(e) => eprintln!("[config] failed to parse {}: {e}", path.display()),
        }
    }

    let cfg = Config::default();
    if let Some(dir) = path.parent() {
        let _ = fs::create_dir_all(dir);
    }
    if let Ok(text) = serde_json::to_string_pretty(&cfg) {
        let _ = fs::write(&path, text);
    }
    cfg
}

/// Parses "#RRGGBB" (or "RRGGBB") into the packed 0x00BBGGRR format the
/// Chroma REST API expects (R | G<<8 | B<<16).
pub fn parse_color(s: &str) -> u32 {
    let s = s.trim_start_matches('#');
    let byte = |range: std::ops::Range<usize>| {
        s.get(range)
            .and_then(|h| u8::from_str_radix(h, 16).ok())
            .unwrap_or(0) as u32
    };
    let r = byte(0..2);
    let g = byte(2..4);
    let b = byte(4..6);
    r | (g << 8) | (b << 16)
}
