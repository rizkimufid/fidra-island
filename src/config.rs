//! Persistent user configuration stored in `%APPDATA%\FidraIsland\config.json`.
//!
//! Uses atomic temp-file write + rename on save and `#[serde(default)]` on load
//! so partial or upgraded configuration files never fail startup.

use serde::{Deserialize, Serialize};
use std::fs;
use std::path::{Path, PathBuf};

/// User-configurable persistent settings for Fidra Island.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct AppConfig {
    /// Whether synced lyrics display is enabled.
    pub lyrics_enabled: bool,
    /// Whether the island stays expanded when the pointer leaves.
    pub is_pinned: bool,
    /// Whether Fidra Island launches automatically at Windows sign-in.
    pub launch_at_startup: bool,
    /// Whether CJK transliteration (Romaji/Pinyin/Hangul Latin) is enabled.
    pub romanize_enabled: bool,
    /// Whether the Islamic prayer countdown banner/pill is enabled.
    pub prayer_enabled: bool,
    /// Latitude for prayer calculation (default: -6.2088 Jakarta).
    pub latitude: f64,
    /// Longitude for prayer calculation (default: 106.8456 Jakarta).
    pub longitude: f64,
    /// Timezone UTC offset in hours (default: +7.0 WIB).
    pub timezone_offset: f64,
}

impl Default for AppConfig {
    fn default() -> Self {
        Self {
            lyrics_enabled: false,
            is_pinned: false,
            launch_at_startup: false,
            romanize_enabled: true,
            prayer_enabled: true,
            latitude: -6.2088,
            longitude: 106.8456,
            timezone_offset: 7.0,
        }
    }
}

/// Returns the default configuration file path (`%APPDATA%\FidraIsland\config.json` on Windows).
pub fn config_path() -> PathBuf {
    let base = std::env::var_os("APPDATA")
        .map(PathBuf::from)
        .or_else(|| std::env::var_os("HOME").map(|h| PathBuf::from(h).join(".config")))
        .unwrap_or_else(|| PathBuf::from("."));
    base.join("FidraIsland").join("config.json")
}

/// Loads [`AppConfig`] from the default path, falling back to defaults on missing or invalid files.
pub fn load() -> AppConfig {
    load_from(&config_path())
}

/// Loads [`AppConfig`] from a specific file path, returning defaults if unreadable.
pub fn load_from(path: &Path) -> AppConfig {
    match fs::read_to_string(path) {
        Ok(content) => serde_json::from_str(&content).unwrap_or_default(),
        Err(_) => AppConfig::default(),
    }
}

/// Saves [`AppConfig`] to the default path.
pub fn save(config: &AppConfig) -> std::io::Result<()> {
    save_to(&config_path(), config)
}

/// Atomically writes [`AppConfig`] to `path` (creates parent directories if needed).
pub fn save_to(path: &Path, config: &AppConfig) -> std::io::Result<()> {
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent)?;
    }
    let json = serde_json::to_string_pretty(config)
        .map_err(|e| std::io::Error::new(std::io::ErrorKind::InvalidData, e))?;
    let tmp_path = path.with_extension("json.tmp");
    fs::write(&tmp_path, json)?;
    fs::rename(&tmp_path, path)?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn roundtrip_save_and_load_config() {
        let dir = std::env::temp_dir().join(format!("fidra_cfg_test_{}", std::process::id()));
        let path = dir.join("config.json");
        let original = AppConfig {
            lyrics_enabled: true,
            is_pinned: true,
            launch_at_startup: false,
            romanize_enabled: true,
            prayer_enabled: true,
            latitude: -6.2088,
            longitude: 106.8456,
            timezone_offset: 7.0,
        };

        save_to(&path, &original).expect("save config");
        let loaded = load_from(&path);
        assert_eq!(loaded, original);

        let _ = fs::remove_dir_all(&dir);
    }

    #[test]
    fn partial_json_uses_defaults_for_missing_fields() {
        let dir = std::env::temp_dir().join(format!("fidra_cfg_partial_{}", std::process::id()));
        let path = dir.join("config.json");
        fs::create_dir_all(&dir).unwrap();
        fs::write(&path, r#"{"lyrics_enabled": true}"#).unwrap();

        let loaded = load_from(&path);
        assert!(loaded.lyrics_enabled);
        assert!(!loaded.is_pinned);
        assert!(!loaded.launch_at_startup);

        let _ = fs::remove_dir_all(&dir);
    }

    #[test]
    fn invalid_or_missing_file_returns_default() {
        let missing = std::env::temp_dir().join("fidra_nonexistent_config_12345.json");
        assert_eq!(load_from(&missing), AppConfig::default());
    }
}
