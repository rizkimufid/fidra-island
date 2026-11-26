//! State application and telemetry helpers for Fidra Island.

use crate::{
    media::{MediaSnapshot, ThumbnailUpdate},
    platform,
    prayer::{self, GeoLocation},
    MainWindow, PrayerRow,
};

#[derive(Default)]
pub(crate) struct SystemStatusCache {
    pub time_str: String,
    pub battery_pct: Option<u8>,
    pub is_charging: Option<bool>,
    pub prayer_countdown: String,
}

pub(crate) fn apply_snapshot(window: &MainWindow, snapshot: MediaSnapshot) {
    let duration = snapshot.duration_seconds.max(0.0);
    let elapsed = if duration > 0.0 {
        snapshot.elapsed_seconds.clamp(0.0, duration)
    } else {
        snapshot.elapsed_seconds.max(0.0)
    };
    let progress = if duration > 0.0 {
        elapsed / duration
    } else {
        0.0
    };
    let remaining = (duration - elapsed).max(0.0);

    window.set_track_title(snapshot.title.into());
    window.set_artist_name(snapshot.artist.into());
    window.set_album_name(snapshot.album.into());
    window.set_elapsed_time(format_time(elapsed).into());
    window.set_duration_time(format_time(duration).into());
    window.set_remaining_time(
        if duration > 0.0 {
            format!("-{}", format_time(remaining))
        } else {
            "—".to_string()
        }
        .into(),
    );
    window.set_duration_seconds(duration);
    window.set_progress(progress);
    window.set_is_playing(snapshot.is_playing);

    // Update dominant ambient accent color
    let (r, g, b) = snapshot.ambient_color;
    window.set_ambient_color(slint::Color::from_rgb_u8(r, g, b));

    match snapshot.thumbnail {
        ThumbnailUpdate::Unchanged => {}
        ThumbnailUpdate::Cleared => {
            window.set_has_album_art(false);
        }
        ThumbnailUpdate::Updated(thumb_bytes) => {
            if let Ok(img) = slint::Image::load_from_data(&thumb_bytes, None) {
                window.set_album_art(img);
                window.set_has_album_art(true);
            } else {
                window.set_has_album_art(false);
            }
        }
    }
}

pub(crate) fn update_system_status(window: &MainWindow, cache: &mut SystemStatusCache) {
    let status = platform::query_system_status();
    if cache.time_str != status.time_str {
        cache.time_str = status.time_str.clone();
        window.set_current_time_str(status.time_str.into());
    }
    if cache.battery_pct != Some(status.battery_pct) {
        cache.battery_pct = Some(status.battery_pct);
        let pct = status.battery_pct as f32 / 100.0;
        window.set_battery_level(pct);
        window.set_battery_arc_path(platform::get_battery_arc_path(pct).into());
    }
    if cache.is_charging != Some(status.is_charging) {
        cache.is_charging = Some(status.is_charging);
        window.set_is_charging(status.is_charging);
    }

    if window.get_prayer_enabled() {
        let (p_status, schedule_items) = prayer::get_current_prayer_status(GeoLocation::default());
        if cache.prayer_countdown != p_status.countdown_str {
            cache.prayer_countdown = p_status.countdown_str.clone();
            window.set_prayer_name(p_status.next_name.into());
            window.set_prayer_countdown(p_status.countdown_str.into());
            window.set_prayer_alert(p_status.is_alert);

            let rows: Vec<PrayerRow> = schedule_items
                .into_iter()
                .map(|item| PrayerRow {
                    name: item.name.into(),
                    time_str: item.time_str.into(),
                    is_next: item.is_next,
                })
                .collect();
            window.set_prayer_schedule(slint::ModelRc::new(slint::VecModel::from(rows)));
        }
    }
}

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
