//! Time formatting utilities for the audio timeline and UI indicators.

/// Formats a non-negative duration in seconds as `MM:SS`.
pub(crate) fn format_time(seconds: f32) -> String {
    let seconds = seconds.max(0.0).round() as u32;
    format!("{:02}:{:02}", seconds / 60, seconds % 60)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn format_time_clamps_negative_and_formats_mm_ss() {
        assert_eq!(format_time(-5.0), "00:00");
        assert_eq!(format_time(0.0), "00:00");
        assert_eq!(format_time(65.4), "01:05");
        assert_eq!(format_time(65.6), "01:06");
        assert_eq!(format_time(3661.0), "61:01");
    }
}
