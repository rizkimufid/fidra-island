use serde::Deserialize;

#[derive(Debug, Clone)]
pub struct LyricLine {
    pub time_sec: f64,
    pub text: String,
    pub words: Vec<LyricWord>,
}

#[derive(Debug, Clone)]
pub struct LyricWord {
    pub time_sec: f64,
    pub text: String,
}

#[derive(Debug, Clone, Default)]
pub struct LyricsData {
    #[allow(dead_code)]
    pub track_key: String,
    pub lines: Vec<LyricLine>,
}

#[derive(Debug, Clone, Default)]
pub struct LyricContext {
    pub prev_text: String,
    pub curr_text: String,
    pub next_text: String,
    pub prev_time: Option<f64>,
    pub curr_time: Option<f64>,
    pub next_time: Option<f64>,
    pub word_before: String,
    pub word_current: String,
    pub word_after: String,
    pub has_word_sync: bool,
}

impl LyricsData {
    #[allow(dead_code)]
    pub fn get_current_line(&self, elapsed_sec: f64) -> Option<&str> {
        if self.lines.is_empty() {
            return None;
        }

        let mut active: Option<&LyricLine> = None;
        for line in &self.lines {
            if line.time_sec <= elapsed_sec + 0.25 {
                active = Some(line);
            } else {
                break;
            }
        }

        active.map(|l| l.text.as_str())
    }

    /// Returns previous, current, and next lines for multi-line display
    #[allow(dead_code)]
    pub fn get_lyrics_context(&self, elapsed_sec: f64) -> (String, String, String) {
        let ctx = self.get_lyrics_context_detailed(elapsed_sec);
        (ctx.prev_text, ctx.curr_text, ctx.next_text)
    }

    /// Returns detailed context including line and optional word timestamps for lyric sync and seek.
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

        let curr_idx = match active_idx {
            Some(i) => i,
            None => 0,
        };

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
        }
    }
}

/// Sanitize track title and artist similar to Fira's LyricsService
pub fn sanitize_metadata(raw_title: &str, raw_artist: &str) -> (String, String) {
    let mut title = raw_title.trim().to_string();
    let mut artist = raw_artist.trim().to_string();

    if title.is_empty() {
        return (String::new(), String::new());
    }

    // Split "Artist - Title" if artist is missing or generic
    if title.contains(" - ") && (artist.is_empty() || artist.eq_ignore_ascii_case("fira")) {
        let parts: Vec<&str> = title.splitn(2, " - ").collect();
        if parts.len() == 2 {
            artist = parts[0].trim().to_string();
            title = parts[1].trim().to_string();
        }
    }

    // Strip common suffixes: (Official Video), [Official Audio], (feat. ...), etc.
    let patterns = [
        r"(?i)\s*[\(\[](?:official\s+(?:video|audio|music\s+video|lyric\s+video)|lyrics?|lyric\s+video|audio|official\s+audio|music\s+video|mv|hd|4k|visualizer|album\s+version|radio\s+edit|radio\s+mix).*?[\)\]]",
        r"(?i)\s*[\(\[](?:remastered(?:\s+\d{4})?|deluxe(?:\s+edition)?|anniversary|live(?:\s+at\s+.*?)?|bonus\s+track|mono|stereo|explicit).*?[\)\]]",
        r#"(?i)\s*-\s*(?:remastered(?:\s+\d{4})?|live|mono|stereo|bonus(?:\s+track)?|radio\s+edit|from\s+".*?").*"#,
        r"(?i)\s*[\(\[](?:feat\.|ft\.|with)\s+.*?[\)\]]",
        r"(?i)\s+(?:feat\.|ft\.)\s+.*",
    ];

    for pattern in patterns {
        if let Ok(re) = regex::Regex::new(pattern) {
            title = re.replace_all(&title, "").to_string();
        }
    }

    // Sanitize Artist: keep primary artist only (split by , & ; /)
    if !artist.is_empty() {
        let parts: Vec<&str> = artist.split(&[',', '&', ';', '/'][..]).collect();
        if let Some(first) = parts.first() {
            artist = first.trim().to_string();
        }
        if let Ok(re) = regex::Regex::new(r"(?i)\s+(?:feat\.|ft\.)\s+.*") {
            artist = re.replace_all(&artist, "").to_string();
        }
    }

    (title.trim().to_string(), artist.trim().to_string())
}

#[derive(Deserialize)]
struct LrclibResponse {
    #[serde(rename = "syncedLyrics")]
    synced_lyrics: Option<String>,
}

/// Fetch synced lyrics using LRCLIB (matching Fira's service)
pub fn fetch_lyrics(title: &str, artist: &str, duration_sec: f64) -> Option<LyricsData> {
    let (clean_title, clean_artist) = sanitize_metadata(title, artist);
    if clean_title.is_empty() {
        return None;
    }

    let track_key = format!("{} - {}", clean_title, clean_artist);

    // 1. Try LRCLIB GET (Exact match)
    if let Some(lrc) = try_lrclib_get(&clean_title, &clean_artist, duration_sec) {
        let lines = parse_lrc(&lrc);
        if !lines.is_empty() {
            return Some(LyricsData { track_key, lines });
        }
    }

    // 2. Try LRCLIB SEARCH (Fuzzy query fallback)
    if let Some(lrc) = try_lrclib_search(&clean_title, &clean_artist) {
        let lines = parse_lrc(&lrc);
        if !lines.is_empty() {
            return Some(LyricsData { track_key, lines });
        }
    }

    None
}

fn try_lrclib_get(title: &str, artist: &str, duration_sec: f64) -> Option<String> {
    let mut url = format!(
        "https://lrclib.net/api/get?track_name={}&artist_name={}",
        url_encode(title),
        url_encode(artist)
    );
    if duration_sec >= 1.0 && duration_sec <= 7200.0 {
        url.push_str(&format!("&duration={}", duration_sec.round() as u64));
    }

    let mut resp = ureq::get(&url).call().ok()?;
    let json: LrclibResponse = resp.body_mut().read_json().ok()?;
    json.synced_lyrics.filter(|s| !s.trim().is_empty())
}

fn try_lrclib_search(title: &str, artist: &str) -> Option<String> {
    let query = format!("{} {}", title, artist);
    let url = format!(
        "https://lrclib.net/api/search?q={}",
        url_encode(query.trim())
    );

    let mut resp = ureq::get(&url).call().ok()?;
    let list: Vec<LrclibResponse> = resp.body_mut().read_json().ok()?;
    for item in list {
        if let Some(synced) = item.synced_lyrics {
            if !synced.trim().is_empty() {
                return Some(synced);
            }
        }
    }

    None
}

/// Simple percent-encoding for query params
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

/// Parse standard LRC timestamp lines `[mm:ss.xx] lyric text`
pub fn parse_lrc(lrc_content: &str) -> Vec<LyricLine> {
    let mut raw_lines: Vec<(f64, String, Vec<LyricWord>)> = Vec::new();

    for line in lrc_content.lines() {
        let line = line.trim();
        if !line.starts_with('[') {
            continue;
        }

        let mut remaining = line;
        let mut timestamps = Vec::new();

        while remaining.starts_with('[') {
            if let Some(close_idx) = remaining.find(']') {
                let stamp = &remaining[1..close_idx];
                remaining = &remaining[close_idx + 1..];

                if let Some(colon_idx) = stamp.find(':') {
                    let min_str = &stamp[..colon_idx];
                    let sec_str = &stamp[colon_idx + 1..];

                    if let Ok(minutes) = min_str.parse::<f64>() {
                        let sec_clean = sec_str.replace(':', ".");
                        if let Ok(seconds) = sec_clean.parse::<f64>() {
                            let total_sec = minutes * 60.0 + seconds;
                            timestamps.push(total_sec);
                        }
                    }
                }
            } else {
                break;
            }
        }

        let text = remaining.trim().to_string();
        if !text.is_empty() && !timestamps.is_empty() {
            let words = parse_word_timestamps(&text);
            for t in timestamps {
                raw_lines.push((t, text.clone(), words.clone()));
            }
        }
    }

    raw_lines.sort_by(|a, b| a.0.partial_cmp(&b.0).unwrap_or(std::cmp::Ordering::Equal));

    raw_lines
        .into_iter()
        .map(|(time_sec, text, words)| LyricLine {
            time_sec,
            text,
            words,
        })
        .collect()
}

fn parse_word_timestamps(text: &str) -> Vec<LyricWord> {
    let mut words = Vec::new();
    let mut remaining = text;
    let mut current_time = None;
    let mut current_text = String::new();

    while let Some(open_idx) = remaining.find('<') {
        current_text.push_str(&remaining[..open_idx]);
        remaining = &remaining[open_idx + 1..];
        let Some(close_idx) = remaining.find('>') else {
            current_text.push('<');
            current_text.push_str(remaining);
            remaining = "";
            break;
        };

        if let Some(time_sec) = parse_timestamp(&remaining[..close_idx]) {
            if let Some(time_sec) = current_time {
                if !current_text.is_empty() {
                    words.push(LyricWord {
                        time_sec,
                        text: std::mem::take(&mut current_text),
                    });
                }
            }
            current_time = Some(time_sec);
        } else {
            current_text.push('<');
            current_text.push_str(&remaining[..=close_idx]);
        }
        remaining = &remaining[close_idx + 1..];
    }
    current_text.push_str(remaining);
    if let Some(time_sec) = current_time {
        if !current_text.is_empty() {
            words.push(LyricWord {
                time_sec,
                text: current_text,
            });
        }
    }

    words
}

fn parse_timestamp(stamp: &str) -> Option<f64> {
    let (minutes, seconds) = stamp.split_once(':')?;
    let minutes = minutes.parse::<f64>().ok()?;
    let seconds = seconds.replace(':', ".").parse::<f64>().ok()?;
    Some(minutes * 60.0 + seconds)
}
