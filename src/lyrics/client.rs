//! LRCLIB HTTP client, track metadata sanitization, and query execution.

use super::parser::parse_lrc;
use super::{cache, LyricsData};
use regex::Regex;
use serde::Deserialize;
use std::path::Path;
use std::sync::OnceLock;
use std::time::Duration;

const USER_AGENT: &str = concat!(
    "FidraIsland/",
    env!("CARGO_PKG_VERSION"),
    " (Windows Desktop Media Controller)"
);

#[derive(Deserialize)]
struct LrclibResponse {
    #[serde(rename = "syncedLyrics")]
    synced_lyrics: Option<String>,
    duration: Option<f64>,
}

pub(crate) fn http_agent() -> &'static ureq::Agent {
    static AGENT: OnceLock<ureq::Agent> = OnceLock::new();
    AGENT.get_or_init(|| {
        ureq::Agent::config_builder()
            .timeout_global(Some(Duration::from_secs(5)))
            .user_agent(USER_AGENT)
            .build()
            .into()
    })
}

fn title_cleanup_regexes() -> &'static [Regex] {
    static PATTERNS: OnceLock<Vec<Regex>> = OnceLock::new();
    PATTERNS.get_or_init(|| {
        [
            r"(?i)\s*[\(\[](?:official\s+(?:video|audio|music\s+video|lyric\s+video)|lyrics?|lyric\s+video|audio|official\s+audio|music\s+video|mv|hd|4k|visualizer|album\s+version|radio\s+edit|radio\s+mix).*?[\)\]]",
            r"(?i)\s*[\(\[](?:remastered(?:\s+\d{4})?|deluxe(?:\s+edition)?|anniversary|live(?:\s+at\s+.*?)?|bonus\s+track|mono|stereo|explicit).*?[\)\]]",
            r#"(?i)\s*-\s*(?:remastered(?:\s+\d{4})?|live|mono|stereo|bonus(?:\s+track)?|radio\s+edit|from\s+".*?").*"#,
            r"(?i)\s*[\(\[](?:feat\.|ft\.|with)\s+.*?[\)\]]",
            r"(?i)\s+(?:feat\.|ft\.)\s+.*",
        ]
        .iter()
        .filter_map(|p| Regex::new(p).ok())
        .collect()
    })
}

fn artist_feat_regex() -> &'static Regex {
    static RE: OnceLock<Regex> = OnceLock::new();
    RE.get_or_init(|| Regex::new(r"(?i)\s+(?:feat\.|ft\.)\s+.*").expect("valid static regex"))
}

/// Sanitizes raw GSMTC track title and artist metadata for higher LRCLIB match accuracy.
pub fn sanitize_metadata(raw_title: &str, raw_artist: &str) -> (String, String) {
    let mut title = raw_title.trim().to_string();
    let mut artist = raw_artist.trim().to_string();

    if title.is_empty() {
        return (String::new(), String::new());
    }

    if title.contains(" - ")
        && (artist.is_empty()
            || artist.eq_ignore_ascii_case("fidra island")
            || artist.eq_ignore_ascii_case("fira"))
    {
        let parts: Vec<&str> = title.splitn(2, " - ").collect();
        if parts.len() == 2 {
            artist = parts[0].trim().to_string();
            title = parts[1].trim().to_string();
        }
    }

    for re in title_cleanup_regexes() {
        title = re.replace_all(&title, "").to_string();
    }

    if !artist.is_empty() {
        if let Some(first) = artist.split(&[',', '&', ';', '/'][..]).next() {
            artist = first.trim().to_string();
        }
        artist = artist_feat_regex().replace_all(&artist, "").to_string();
    }

    (title.trim().to_string(), artist.trim().to_string())
}

/// Fetches synced lyrics using disk cache, exact match, and fuzzy search fallback.
pub fn fetch_lyrics_with_cache(
    title: &str,
    artist: &str,
    duration_sec: f64,
    dir: &Path,
    now_secs: u64,
) -> Option<LyricsData> {
    let (clean_title, clean_artist) = sanitize_metadata(title, artist);
    if clean_title.is_empty() {
        return None;
    }

    if let Some(cached_result) = cache::load_from_cache(dir, &clean_title, &clean_artist, now_secs)
    {
        return cached_result;
    }

    let agent = http_agent();
    let fetched = if let Some(lrc) =
        try_lrclib_get(agent, &clean_title, &clean_artist, duration_sec)
    {
        let lines = parse_lrc(&lrc);
        if !lines.is_empty() {
            Some(LyricsData { lines })
        } else {
            None
        }
    } else if let Some(lrc) = try_lrclib_search(agent, &clean_title, &clean_artist, duration_sec) {
        let lines = parse_lrc(&lrc);
        if !lines.is_empty() {
            Some(LyricsData { lines })
        } else {
            None
        }
    } else {
        None
    };

    cache::save_to_cache(dir, &clean_title, &clean_artist, fetched.as_ref(), now_secs);

    fetched
}

pub(crate) fn try_lrclib_get(
    agent: &ureq::Agent,
    title: &str,
    artist: &str,
    duration_sec: f64,
) -> Option<String> {
    let mut url = format!(
        "https://lrclib.net/api/get?track_name={}&artist_name={}",
        url_encode(title),
        url_encode(artist)
    );
    if (1.0..=7200.0).contains(&duration_sec) {
        url.push_str(&format!("&duration={}", duration_sec.round() as u64));
    }

    let mut resp = agent.get(&url).call().ok()?;
    let json: LrclibResponse = resp.body_mut().read_json().ok()?;
    json.synced_lyrics.filter(|s| !s.trim().is_empty())
}

pub(crate) fn try_lrclib_search(
    agent: &ureq::Agent,
    title: &str,
    artist: &str,
    duration_sec: f64,
) -> Option<String> {
    let query = format!("{} {}", title, artist);
    let url = format!(
        "https://lrclib.net/api/search?q={}",
        url_encode(query.trim())
    );

    let mut resp = agent.get(&url).call().ok()?;
    let list: Vec<LrclibResponse> = resp.body_mut().read_json().ok()?;

    let has_valid_duration = (1.0..=7200.0).contains(&duration_sec);
    let mut first_synced: Option<String> = None;

    for item in list {
        let Some(synced) = item.synced_lyrics.filter(|s| !s.trim().is_empty()) else {
            continue;
        };
        if has_valid_duration {
            if let Some(d) = item.duration {
                if (d - duration_sec).abs() <= 5.0 {
                    return Some(synced);
                }
            }
        }
        if first_synced.is_none() {
            first_synced = Some(synced);
        }
    }

    first_synced
}

fn url_encode(input: &str) -> String {
    let mut encoded = String::with_capacity(input.len() * 3);
    for b in input.bytes() {
        match b {
            b'a'..=b'z' | b'A'..=b'Z' | b'0'..=b'9' | b'-' | b'_' | b'.' | b'~' => {
                encoded.push(b as char);
            }
            b' ' => encoded.push_str("%20"),
            _ => {
                encoded.push_str(&format!("%{:02X}", b));
            }
        }
    }
    encoded
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn sanitize_strips_common_suffixes_and_splits_artist() {
        let (title, artist) = sanitize_metadata(
            "Demo Artist - Track Alpha (Official Video) (feat. Guest Vocalist)",
            "",
        );
        assert_eq!(title, "Track Alpha");
        assert_eq!(artist, "Demo Artist");

        let (title2, artist2) = sanitize_metadata(
            "Song Beta - Remastered 2021",
            "Example Band, Lead Vocal & Guitarist",
        );
        assert_eq!(title2, "Song Beta");
        assert_eq!(artist2, "Example Band");
    }
}
