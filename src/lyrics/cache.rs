//! Persistent lyrics disk caching with positive (14-day) and negative (2-day) TTL.

use super::LyricsData;
use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};

/// Default TTL for successfully resolved lyrics (14 days).
pub const POSITIVE_CACHE_TTL_SECS: u64 = 14 * 24 * 60 * 60;
/// Default TTL for negative lyric lookups (not found on LRCLIB: 2 days).
pub const NEGATIVE_CACHE_TTL_SECS: u64 = 2 * 24 * 60 * 60;

/// Persistent disk cache entry wrapping cached lyrics and the retrieval timestamp.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct DiskCacheEntry {
    /// Unix epoch timestamp in seconds when the entry was stored.
    pub cached_at: u64,
    /// Cached lyrics if found, or `None` for cached negative lookups.
    pub lyrics: Option<LyricsData>,
}

/// Returns the persistent lyrics disk cache directory (`%LOCALAPPDATA%\FidraIsland\cache\lyrics` on Windows).
pub fn cache_dir() -> PathBuf {
    let base = std::env::var_os("LOCALAPPDATA")
        .map(PathBuf::from)
        .or_else(|| std::env::var_os("HOME").map(|h| PathBuf::from(h).join(".local/share")))
        .unwrap_or_else(|| PathBuf::from("."));
    base.join("FidraIsland").join("cache").join("lyrics")
}

/// Generates a filesystem-safe cache filename based on primary artist, title, and a deterministic 64-bit hash.
pub fn cache_filename_for(title: &str, artist: &str) -> String {
    let key = format!("{} - {}", artist.trim(), title.trim()).to_lowercase();
    let mut hash: u64 = 0xcbf29ce484222325;
    for byte in key.as_bytes() {
        hash ^= *byte as u64;
        hash = hash.wrapping_mul(0x100000001b3);
    }
    let mut slug = String::new();
    for c in key.chars().take(24) {
        if c.is_ascii_alphanumeric() {
            slug.push(c);
        } else if !slug.ends_with('_') && !slug.is_empty() {
            slug.push('_');
        }
    }
    let slug = slug.trim_end_matches('_');
    if slug.is_empty() {
        format!("track_{hash:016x}.json")
    } else {
        format!("{slug}_{hash:016x}.json")
    }
}

/// Loads a cached entry if present on disk and within its valid TTL window.
pub fn load_from_cache(
    dir: &Path,
    title: &str,
    artist: &str,
    now_secs: u64,
) -> Option<Option<LyricsData>> {
    let file = dir.join(cache_filename_for(title, artist));
    let content = std::fs::read_to_string(file).ok()?;
    let entry: DiskCacheEntry = serde_json::from_str(&content).ok()?;
    let ttl = if entry.lyrics.is_some() {
        POSITIVE_CACHE_TTL_SECS
    } else {
        NEGATIVE_CACHE_TTL_SECS
    };
    if now_secs.saturating_sub(entry.cached_at) <= ttl {
        Some(entry.lyrics)
    } else {
        None
    }
}

/// Persists a lyric lookup result (positive or negative) to disk cache.
pub fn save_to_cache(
    dir: &Path,
    title: &str,
    artist: &str,
    lyrics: Option<&LyricsData>,
    now_secs: u64,
) {
    if std::fs::create_dir_all(dir).is_err() {
        return;
    }
    let filename = cache_filename_for(title, artist);
    let target = dir.join(&filename);
    let tmp = dir.join(format!("{filename}.tmp"));
    let entry = DiskCacheEntry {
        cached_at: now_secs,
        lyrics: lyrics.cloned(),
    };
    if let Ok(json) = serde_json::to_string_pretty(&entry) {
        if std::fs::write(&tmp, json).is_ok() {
            let _ = std::fs::rename(tmp, target);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::lyrics::LyricLine;

    #[test]
    fn cache_roundtrip_positive_and_negative_entries() {
        let temp_dir = std::env::temp_dir().join(format!("fidra_lrc_cache_{}", std::process::id()));
        let title = "Melody One";
        let artist = "Composer Demo";
        let now = 1_700_000_000;

        let sample_lyrics = LyricsData {
            lines: vec![LyricLine {
                time_sec: 12.0,
                text: "Sample lyrical sentence for testing".to_string(),
                words: vec![],
            }],
        };

        save_to_cache(&temp_dir, title, artist, Some(&sample_lyrics), now);
        let loaded = load_from_cache(&temp_dir, title, artist, now + 100);
        assert_eq!(loaded, Some(Some(sample_lyrics)));

        let missing_title = "Instrumental Interlude";
        save_to_cache(&temp_dir, missing_title, artist, None, now);
        let loaded_missing = load_from_cache(&temp_dir, missing_title, artist, now + 100);
        assert_eq!(loaded_missing, Some(None));

        let expired = load_from_cache(&temp_dir, title, artist, now + POSITIVE_CACHE_TTL_SECS + 5);
        assert_eq!(expired, None);

        let _ = std::fs::remove_dir_all(&temp_dir);
    }
}
