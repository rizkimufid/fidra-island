//! Synced lyrics fetching, metadata sanitization, disk caching, and LRC parsing.
//!
//! Decomposed into modular submodules (< 250 LOC each) adhering to the strict
//! file size limit:
//! - [`parser`]: LRC format and word-sync parsing.
//! - [`cache`]: Persistent disk cache with positive (14-day) and negative (2-day) TTL.
//! - [`client`]: LRCLIB HTTP requests, connection pooling, and metadata sanitization.

pub mod cache;
pub mod client;
pub mod parser;
pub mod romanize;

#[allow(unused_imports)]
pub use cache::{cache_dir, DiskCacheEntry, NEGATIVE_CACHE_TTL_SECS, POSITIVE_CACHE_TTL_SECS};
#[allow(unused_imports)]
pub use client::{fetch_lyrics_with_cache, sanitize_metadata};
#[allow(unused_imports)]
pub use parser::parse_lrc;
#[allow(unused_imports)]
pub use romanize::romanize_line;
use serde::{Deserialize, Serialize};

/// A single timestamped lyric line, optionally enriched with word-level timestamps.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct LyricLine {
    /// Line start time in seconds.
    pub time_sec: f64,
    /// Clean display text with any inline `<mm:ss.xx>` word tags stripped.
    pub text: String,
    /// Optional word/syllable timestamps parsed from enhanced LRC (`<mm:ss.xx>`).
    pub words: Vec<LyricWord>,
}

/// A word or syllable segment within an enhanced LRC line.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct LyricWord {
    /// Segment start time in seconds.
    pub time_sec: f64,
    /// Segment text including trailing/leading spacing as authored in the LRC.
    pub text: String,
}

/// Parsed synced lyrics for a track.
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
pub struct LyricsData {
    /// Sorted list of timestamped lyric lines.
    pub lines: Vec<LyricLine>,
}

/// Resolved 3-line lyric window and optional word-sync split at a given playback timestamp.
#[derive(Debug, Clone, Default, PartialEq)]
pub struct LyricContext {
    /// Text of the previous lyric line (empty if on the first line).
    pub prev_text: String,
    /// Text of the active lyric line.
    pub curr_text: String,
    /// Text of the next lyric line (empty if on the last line).
    pub next_text: String,
    /// Start timestamp of the previous lyric line in seconds.
    pub prev_time: Option<f64>,
    /// Start timestamp of the active lyric line in seconds.
    pub curr_time: Option<f64>,
    /// Start timestamp of the next lyric line in seconds.
    pub next_time: Option<f64>,
    /// Words already sung in the active line (when `has_word_sync` is `true`).
    pub word_before: String,
    /// Currently active word/syllable (when `has_word_sync` is `true`).
    pub word_current: String,
    /// Upcoming words in the active line (when `has_word_sync` is `true`).
    pub word_after: String,
    /// Whether the active line has enhanced word-level timestamps.
    pub has_word_sync: bool,
    /// Zero-based index of the active line within [`LyricsData::lines`].
    pub curr_index: usize,
}

impl LyricsData {
    /// Computes the active 3-line lyric context and word-sync segments for `elapsed_sec`.
    pub fn get_lyrics_context_detailed(&self, elapsed_sec: f64) -> LyricContext {
        if self.lines.is_empty() {
            return LyricContext::default();
        }

        let mut active_idx: Option<usize> = None;
        for (i, line) in self.lines.iter().enumerate() {
            if line.time_sec <= elapsed_sec + 0.15 {
                active_idx = Some(i);
            } else {
                break;
            }
        }

        let curr_idx = active_idx.unwrap_or_default();

        let prev = if curr_idx > 0 {
            (
                self.lines[curr_idx - 1].text.clone(),
                Some(self.lines[curr_idx - 1].time_sec),
            )
        } else {
            (String::new(), None)
        };

        let curr_time = self.lines.get(curr_idx).map(|l| l.time_sec);
        let curr_text = self
            .lines
            .get(curr_idx)
            .map(|l| l.text.clone())
            .unwrap_or_default();

        let next = if curr_idx + 1 < self.lines.len() {
            (
                self.lines[curr_idx + 1].text.clone(),
                Some(self.lines[curr_idx + 1].time_sec),
            )
        } else {
            (String::new(), None)
        };

        let words = self.lines.get(curr_idx).map(|line| &line.words);
        let has_word_sync = words.map(|words| words.len() > 1).unwrap_or(false);
        let (word_before, word_current, word_after) = if has_word_sync {
            let words = words.expect("word sync requires parsed word timestamps");
            let active_idx = words.iter().rposition(|word| word.time_sec <= elapsed_sec);
            match active_idx {
                Some(idx) => (
                    words[..idx].iter().map(|word| word.text.as_str()).collect(),
                    words[idx].text.clone(),
                    words[idx + 1..]
                        .iter()
                        .map(|word| word.text.as_str())
                        .collect(),
                ),
                None => (String::new(), String::new(), curr_text.clone()),
            }
        } else {
            (String::new(), String::new(), String::new())
        };

        LyricContext {
            prev_text: prev.0,
            curr_text,
            next_text: next.0,
            prev_time: prev.1,
            curr_time,
            next_time: next.1,
            word_before,
            word_current,
            word_after,
            has_word_sync,
            curr_index: curr_idx,
        }
    }

    /// Returns the plain display text of every line, in order.
    pub fn line_texts(&self) -> Vec<String> {
        self.lines.iter().map(|l| l.text.clone()).collect()
    }
}

/// Returns `true` when `text` contains Japanese script (hiragana, katakana, or CJK ideographs).
///
/// Used by the UI to pick the Japanese typeface for a lyric line.
pub fn is_japanese(text: &str) -> bool {
    text.chars().any(|c| {
        matches!(c as u32,
            0x3040..=0x309F   // Hiragana
            | 0x30A0..=0x30FF // Katakana
            | 0x31F0..=0x31FF // Katakana phonetic extensions
            | 0xFF66..=0xFF9F // Halfwidth katakana
            | 0x3400..=0x4DBF // CJK extension A
            | 0x4E00..=0x9FFF // CJK unified ideographs
        )
    })
}

/// Convenience entrypoint fetching lyrics using default cache directory and wall-clock timestamp.
pub fn fetch_lyrics(title: &str, artist: &str, duration_sec: f64) -> Option<LyricsData> {
    let now = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_secs())
        .unwrap_or(0);
    fetch_lyrics_with_cache(title, artist, duration_sec, &cache_dir(), now)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn detects_japanese_script_only() {
        assert!(is_japanese("こんにちは"));
        assert!(is_japanese("カタカナ"));
        assert!(is_japanese("夜空 mix"));
        assert!(!is_japanese("Plain latin lyric"));
        assert!(!is_japanese("Suatu saat nanti"));
        assert!(!is_japanese(""));
    }

    #[test]
    fn lyrics_context_resolves_prev_curr_next_and_words() {
        let lrc = "\
[00:05.00]Line one
[00:10.00]<00:10.00>Sing <00:10.50>along <00:11.00>now
[00:15.00]Line three
";
        let data = LyricsData {
            lines: parse_lrc(lrc),
        };
        let ctx = data.get_lyrics_context_detailed(10.6);
        assert_eq!(ctx.prev_text, "Line one");
        assert_eq!(ctx.curr_text, "Sing along now");
        assert_eq!(ctx.next_text, "Line three");
        assert!(ctx.has_word_sync);
        assert_eq!(ctx.word_before, "Sing ");
        assert_eq!(ctx.word_current, "along ");
        assert_eq!(ctx.word_after, "now");
    }
}
