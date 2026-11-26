//! Transliteration (romanization) for CJK lyrics (Japanese, Korean, Chinese).
//!
//! Converts non-Latin East Asian scripts into readable Latin text:
//! - **Japanese**: Kanji/Kana to Romaji using [`kakasi`].
//! - **Chinese**: Hanzi to Pinyin using [`pinyin`].
//! - **Korean**: Hangul syllables to Revised Romanization algorithmically.

use pinyin::ToPinyin;

/// Translates a line of East Asian text (Japanese, Korean, or Chinese) into Latin/Romaji/Pinyin.
/// Returns `None` if the input text contains no CJK characters.
pub fn romanize_line(text: &str) -> Option<String> {
    if !has_cjk(text) {
        return None;
    }

    // 1. Japanese check (Hiragana, Katakana, or Kakasi recognizes it as Japanese)
    if is_japanese(text) {
        let res = kakasi::convert(text);
        if !res.romaji.is_empty() && res.romaji != text {
            return Some(res.romaji);
        }
    }

    // 2. Korean check (Hangul syllables or Jamo)
    if is_korean(text) {
        let rom = romanize_hangul(text);
        if !rom.is_empty() && rom != text {
            return Some(rom);
        }
    }

    // 3. Chinese / Hanzi fallback (or Japanese Kanji missed)
    let pinyin_str = romanize_chinese(text);
    if !pinyin_str.is_empty() && pinyin_str != text {
        return Some(pinyin_str);
    }

    None
}

/// Checks whether the string contains any East Asian characters (Japanese, Korean, or CJK ideographs).
pub fn has_cjk(text: &str) -> bool {
    text.chars().any(|c| {
        matches!(c as u32,
            0x1100..=0x11FF   // Hangul Jamo
            | 0x3040..=0x309F // Hiragana
            | 0x30A0..=0x30FF // Katakana
            | 0x3130..=0x318F // Hangul Compatibility Jamo
            | 0x3400..=0x4DBF // CJK Unified Ideographs Extension A
            | 0x4E00..=0x9FFF // CJK Unified Ideographs
            | 0xAC00..=0xD7AF // Hangul Syllables
        )
    })
}

/// Checks whether the string specifically contains Japanese kana (Hiragana/Katakana).
pub fn is_japanese(text: &str) -> bool {
    text.chars().any(|c| {
        matches!(c as u32,
            0x3040..=0x309F   // Hiragana
            | 0x30A0..=0x30FF // Katakana
            | 0x31F0..=0x31FF // Katakana phonetic extensions
            | 0xFF66..=0xFF9F // Halfwidth katakana
        )
    })
}

/// Checks whether the string contains Korean Hangul syllables or Jamo.
pub fn is_korean(text: &str) -> bool {
    text.chars().any(|c| {
        matches!(c as u32,
            0xAC00..=0xD7AF   // Hangul Syllables
            | 0x1100..=0x11FF // Hangul Jamo
            | 0x3130..=0x318F // Hangul Compatibility Jamo
        )
    })
}

/// Romanizes Chinese Hanzi characters into Pinyin with tone marks.
fn romanize_chinese(text: &str) -> String {
    let mut out = String::with_capacity(text.len() * 2);
    let mut prev_was_pinyin = false;

    for (c, p) in text.chars().zip(text.to_pinyin()) {
        if let Some(p) = p {
            if prev_was_pinyin {
                out.push(' ');
            }
            out.push_str(p.with_tone());
            prev_was_pinyin = true;
        } else {
            out.push(c);
            prev_was_pinyin = false;
        }
    }
    out
}

/// Converts Korean Hangul syllables into Revised Romanization of Korean.
fn romanize_hangul(text: &str) -> String {
    // Revised Romanization onset (choseong) table (19)
    const ONSET: [&str; 19] = [
        "g", "kk", "n", "d", "tt", "r", "m", "b", "pp", "s", "ss", "", "j", "jj", "ch", "k", "t",
        "p", "h",
    ];
    // Revised Romanization nucleus (jungseong) table (21)
    const VOWEL: [&str; 21] = [
        "a", "ae", "ya", "yae", "eo", "e", "yeo", "ye", "o", "wa", "wae", "oe", "yo", "u", "wo",
        "we", "wi", "yu", "eu", "ui", "i",
    ];
    // Revised Romanization coda (jongseong) table (28)
    const CODA: [&str; 28] = [
        "", "k", "k", "ks", "n", "nj", "nh", "t", "l", "lg", "lm", "lb", "ls", "lt", "lp", "lh",
        "m", "p", "bs", "t", "ss", "ng", "t", "t", "k", "t", "p", "h",
    ];

    let mut out = String::with_capacity(text.len() * 2);
    for c in text.chars() {
        let code = c as u32;
        if (0xAC00..=0xD7A3).contains(&code) {
            let offset = code - 0xAC00;
            let onset_idx = (offset / 588) as usize;
            let vowel_idx = ((offset % 588) / 28) as usize;
            let coda_idx = (offset % 28) as usize;

            out.push_str(ONSET[onset_idx]);
            out.push_str(VOWEL[vowel_idx]);
            out.push_str(CODA[coda_idx]);
        } else {
            out.push(c);
        }
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_has_cjk_detection() {
        assert!(has_cjk("こんにちは"));
        assert!(has_cjk("안녕하세요"));
        assert!(has_cjk("你好世界"));
        assert!(!has_cjk("Hello World 123"));
    }

    #[test]
    fn test_japanese_romanization() {
        let rom = romanize_line("こんにちは").unwrap();
        assert!(!rom.is_empty());
        // kakasi converts to romaji
        assert!(rom.to_lowercase().contains("konnichi") || rom.to_lowercase().contains("ha"));
    }

    #[test]
    fn test_korean_romanization() {
        let rom = romanize_line("안녕").unwrap();
        assert_eq!(rom, "annyeong");
    }

    #[test]
    fn test_chinese_romanization() {
        let rom = romanize_line("你好").unwrap();
        assert!(rom.contains("nǐ") || rom.contains("hǎo"));
    }
}
