//! LRC format parser supporting standard line timestamps, enhanced `<mm:ss.xx>`
//! word timestamps, and `[offset:+/-ms]` tags.

use super::{LyricLine, LyricWord};

/// Parses standard LRC (`[mm:ss.xx] text`) and enhanced word-synced LRC (`<mm:ss.xx>word`)
/// content, honoring `[offset:+/-ms]` tags and sorting lines chronologically.
pub fn parse_lrc(lrc_content: &str) -> Vec<LyricLine> {
    let mut raw_lines: Vec<(f64, String, Vec<LyricWord>)> = Vec::new();
    let mut offset_sec: f64 = 0.0;

    for line in lrc_content.lines() {
        let line = line.trim();
        if !line.starts_with('[') {
            continue;
        }

        // Handle optional [offset:+/-ms] metadata tag
        if let Some(rest) = line
            .strip_prefix("[offset:")
            .or_else(|| line.strip_prefix("[OFFSET:"))
        {
            if let Some(val_str) = rest.strip_suffix(']') {
                if let Ok(ms) = val_str.trim().parse::<f64>() {
                    offset_sec = ms / 1000.0;
                }
            }
            continue;
        }

        let mut remaining = line;
        let mut timestamps = Vec::new();

        while remaining.starts_with('[') {
            if let Some(close_idx) = remaining.find(']') {
                let stamp = &remaining[1..close_idx];
                remaining = &remaining[close_idx + 1..];

                if let Some(total_sec) = parse_timestamp(stamp) {
                    timestamps.push(total_sec);
                }
            } else {
                break;
            }
        }

        let raw_text = remaining.trim();
        if !raw_text.is_empty() && !timestamps.is_empty() {
            let words = parse_word_timestamps(raw_text);
            let clean_text = if words.is_empty() {
                raw_text.to_string()
            } else {
                words
                    .iter()
                    .map(|w| w.text.as_str())
                    .collect::<String>()
                    .trim()
                    .to_string()
            };

            if !clean_text.is_empty() {
                for t in timestamps {
                    raw_lines.push((t, clean_text.clone(), words.clone()));
                }
            }
        }
    }

    // Apply LRC offset (positive offset shifts lyrics earlier)
    if offset_sec != 0.0 {
        for (time_sec, _, words) in &mut raw_lines {
            *time_sec = (*time_sec - offset_sec).max(0.0);
            for w in words {
                w.time_sec = (w.time_sec - offset_sec).max(0.0);
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

pub(crate) fn parse_word_timestamps(text: &str) -> Vec<LyricWord> {
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

pub(crate) fn parse_timestamp(stamp: &str) -> Option<f64> {
    let (minutes, seconds) = stamp.split_once(':')?;
    let minutes = minutes.parse::<f64>().ok()?;
    let seconds = seconds.replace(':', ".").parse::<f64>().ok()?;
    Some(minutes * 60.0 + seconds)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_lrc_handles_multi_timestamps_and_sorting() {
        let lrc = "\
[00:15.00]Second line
[00:05.50][00:25.00]Chorus line
[ar:Test Artist]
plain non-timestamped text
";
        let lines = parse_lrc(lrc);
        assert_eq!(lines.len(), 3);
        assert_eq!(lines[0].time_sec, 5.5);
        assert_eq!(lines[0].text, "Chorus line");
        assert_eq!(lines[1].time_sec, 15.0);
        assert_eq!(lines[1].text, "Second line");
        assert_eq!(lines[2].time_sec, 25.0);
        assert_eq!(lines[2].text, "Chorus line");
    }

    #[test]
    fn parse_lrc_strips_word_sync_tags_from_display_text() {
        let lrc = "[00:10.00]<00:10.00>Hello <00:10.50>world <00:11.00>today";
        let lines = parse_lrc(lrc);
        assert_eq!(lines.len(), 1);
        assert_eq!(lines[0].text, "Hello world today");
        assert_eq!(lines[0].words.len(), 3);
        assert_eq!(lines[0].words[1].text, "world ");
        assert_eq!(lines[0].words[1].time_sec, 10.5);
    }

    #[test]
    fn parse_lrc_applies_offset_tag() {
        let lrc = "\
[offset:500]
[00:10.00]Shifted line
";
        let lines = parse_lrc(lrc);
        assert_eq!(lines.len(), 1);
        assert!((lines[0].time_sec - 9.5).abs() < 1e-6);
    }
}
