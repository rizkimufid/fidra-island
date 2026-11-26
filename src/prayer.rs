//! Astronomical Islamic prayer time calculations (Kemenag / SIHAT & MWL standard).
//!
//! Provides pure, offline solar position and prayer time calculation with zero heavyweight
//! external dependencies. Supports custom coordinates (e.g. Jakarta/WIB default) and standard
//! Indonesian Kemenag twilight parameters (Fajr: 20°, Isha: 18°, Shafi'i Asr).

use std::time::{SystemTime, UNIX_EPOCH};

/// Standard prayer times for a single solar day.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct PrayerTimes {
    /// Subuh (Fajr) start time in fractional hours (0.0 .. 24.0) local time.
    pub fajr: f64,
    /// Terbit matahari (Sunrise) in fractional hours.
    pub sunrise: f64,
    /// Dzuhur (Dhuhr) start time in fractional hours.
    pub dhuhr: f64,
    /// Ashar (Asr) start time in fractional hours.
    pub asr: f64,
    /// Maghrib start time in fractional hours.
    pub maghrib: f64,
    /// Isya (Isha) start time in fractional hours.
    pub isha: f64,
}

/// Active prayer status and countdown to the upcoming prayer.
#[derive(Debug, Clone, PartialEq)]
pub struct PrayerStatus {
    /// Name of the current active prayer window (e.g. "Dzuhur").
    pub current_name: String,
    /// Name of the next upcoming prayer (e.g. "Ashar").
    pub next_name: String,
    /// Formatted time of the upcoming prayer (e.g. "15:18").
    pub next_time_str: String,
    /// Countdown string remaining until next prayer (e.g. "45m" or "2j 10m").
    pub countdown_str: String,
    /// Total seconds remaining until the next prayer.
    pub seconds_remaining: u32,
    /// Whether an adhan/alert should trigger (e.g. within 60s of arrival).
    pub is_alert: bool,
}

/// Geographic coordinates for prayer calculation.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct GeoLocation {
    pub latitude: f64,
    pub longitude: f64,
    /// Timezone UTC offset in hours (e.g. +7.0 for WIB, +8.0 for WITA, +9.0 for WIT).
    pub timezone_offset: f64,
}

impl Default for GeoLocation {
    /// Default location: Jakarta, Indonesia (WIB UTC+7).
    fn default() -> Self {
        Self {
            latitude: -6.2088,
            longitude: 106.8456,
            timezone_offset: 7.0,
        }
    }
}

/// Calculates prayer times for a given Julian Day, location, and Kemenag twilight angles.
pub fn calculate_prayer_times(year: i32, month: u32, day: u32, loc: GeoLocation) -> PrayerTimes {
    let jd = julian_day(year, month, day);
    let d = jd - 2451545.0;

    // Mean anomaly and solar longitude
    let g = (357.529 + 0.98560028 * d).to_radians();
    let q = (280.459 + 0.98564736 * d).to_radians();
    let l = (q.to_degrees() + 1.915 * g.sin() + 0.020 * (2.0 * g).sin()).to_radians();

    // Obliquity of the ecliptic and sun declination
    let e = (23.439 - 0.00000036 * d).to_radians();
    let decl = (e.sin() * l.sin()).asin();

    // Equation of time in hours
    let ra = (e.cos() * l.sin()).atan2(l.cos());
    let mut eqt = (q.to_degrees() / 15.0) - (ra.to_degrees() / 15.0);
    while eqt > 12.0 {
        eqt -= 24.0;
    }
    while eqt < -12.0 {
        eqt += 24.0;
    }

    // Solar noon (Dhuhr) in local hours
    let noon = 12.0 + loc.timezone_offset - (loc.longitude / 15.0) - eqt;
    let dhuhr = noon + (2.0 / 60.0); // +2 minutes ihtiyat (safety precaution standard Kemenag)

    let lat_rad = loc.latitude.to_radians();

    // Sun altitude angles
    // Kemenag standard: Subuh = -20.0°, Isya = -18.0°, Sunrise/Sunset = -0.833°
    let fajr_angle = (-20.0f64).to_radians();
    let isha_angle = (-18.0f64).to_radians();
    let horizon_angle = (-0.833f64).to_radians();

    let sunrise_hour_angle = hour_angle(lat_rad, decl, horizon_angle);
    let fajr_hour_angle = hour_angle(lat_rad, decl, fajr_angle);
    let isha_hour_angle = hour_angle(lat_rad, decl, isha_angle);

    // Asr (Shafi'i: shadow length factor = 1.0)
    let asr_alt = (1.0 + (loc.latitude - decl.to_degrees()).abs().to_radians().tan())
        .recip()
        .atan();
    let asr_hour_angle = hour_angle(lat_rad, decl, asr_alt);

    let sunrise = noon - sunrise_hour_angle;
    let sunset = noon + sunrise_hour_angle;
    let fajr = noon - fajr_hour_angle;
    let asr = noon + asr_hour_angle;
    let maghrib = sunset + (2.0 / 60.0); // +2 minutes ihtiyat
    let isha = noon + isha_hour_angle + (2.0 / 60.0);

    PrayerTimes {
        fajr: fajr.rem_euclid(24.0),
        sunrise: sunrise.rem_euclid(24.0),
        dhuhr: dhuhr.rem_euclid(24.0),
        asr: asr.rem_euclid(24.0),
        maghrib: maghrib.rem_euclid(24.0),
        isha: isha.rem_euclid(24.0),
    }
}

/// Formatted schedule row for UI presentation.
#[derive(Debug, Clone, PartialEq)]
pub struct PrayerScheduleItem {
    pub name: String,
    pub time_str: String,
    pub is_next: bool,
}

/// Resolves current prayer status and full daily schedule for UI presentation.
pub fn get_current_prayer_status(loc: GeoLocation) -> (PrayerStatus, Vec<PrayerScheduleItem>) {
    let now = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs();
    let local_secs = (now as i64 + (loc.timezone_offset * 3600.0) as i64).rem_euclid(86400) as u32;

    let current_hour = local_secs / 3600;
    let current_min = (local_secs % 3600) / 60;
    let current_sec = local_secs % 60;
    let current_frac =
        current_hour as f64 + (current_min as f64 / 60.0) + (current_sec as f64 / 3600.0);

    // Approximate UTC date to Julian Day
    let days_since_epoch = (now as f64 + loc.timezone_offset * 3600.0) / 86400.0;
    let (year, month, day) = civil_from_days(days_since_epoch as i64);

    let prayers = calculate_prayer_times(year, month, day, loc);

    let schedule = [
        ("Subuh", prayers.fajr),
        ("Terbit", prayers.sunrise),
        ("Dzuhur", prayers.dhuhr),
        ("Ashar", prayers.asr),
        ("Maghrib", prayers.maghrib),
        ("Isya", prayers.isha),
    ];

    let mut current_idx = schedule.len() - 1;
    let mut next_idx = 0;

    for (i, &(_, time)) in schedule.iter().enumerate() {
        if current_frac >= time {
            current_idx = i;
            next_idx = (i + 1) % schedule.len();
        }
    }

    let (curr_name, _) = schedule[current_idx];
    let (next_name, next_time) = schedule[next_idx];

    let next_secs = (next_time * 3600.0).round() as u32;
    let seconds_remaining = if next_secs >= local_secs {
        next_secs - local_secs
    } else {
        (86400 - local_secs) + next_secs
    };

    let hours_left = seconds_remaining / 3600;
    let mins_left = (seconds_remaining % 3600) / 60;

    let countdown_str = if hours_left > 0 {
        format!("{hours_left}j {mins_left}m")
    } else {
        format!("{mins_left}m")
    };

    let next_h = (next_secs / 3600) % 24;
    let next_m = (next_secs % 3600) / 60;
    let next_time_str = format!("{next_h:02}:{next_m:02}");

    let status = PrayerStatus {
        current_name: curr_name.to_string(),
        next_name: next_name.to_string(),
        next_time_str,
        countdown_str,
        seconds_remaining,
        is_alert: seconds_remaining <= 120, // Alert 2 menit sebelum waktu sholat
    };

    let schedule_items: Vec<PrayerScheduleItem> = schedule
        .iter()
        .enumerate()
        .map(|(i, &(name, time))| {
            let s = (time * 3600.0).round() as u32;
            let h = (s / 3600) % 24;
            let m = (s % 3600) / 60;
            PrayerScheduleItem {
                name: name.to_string(),
                time_str: format!("{h:02}:{m:02}"),
                is_next: i == next_idx,
            }
        })
        .collect();

    (status, schedule_items)
}

fn hour_angle(lat: f64, decl: f64, alt: f64) -> f64 {
    let cos_ha = (alt.sin() - lat.sin() * decl.sin()) / (lat.cos() * decl.cos());
    cos_ha.clamp(-1.0, 1.0).acos().to_degrees() / 15.0
}

fn julian_day(year: i32, month: u32, day: u32) -> f64 {
    let mut y = year;
    let mut m = month as i32;
    if m <= 2 {
        y -= 1;
        m += 12;
    }
    let a = y / 100;
    let b = 2 - a + (a / 4);
    (365.25 * (y + 4716) as f64).floor()
        + (30.6001 * (m + 1) as f64).floor()
        + day as f64
        + b as f64
        - 1524.5
}

fn civil_from_days(days: i64) -> (i32, u32, u32) {
    let z = days + 719468;
    let era = if z >= 0 { z } else { z - 146096 } / 146097;
    let doe = (z - era * 146097) as u32;
    let yoe = (doe - doe / 1020 + doe / 1461 - doe / 146096) / 365;
    let y = (yoe as i64) + era * 400;
    let doy = doe - (365 * yoe + yoe / 4 - yoe / 100);
    let mp = (5 * doy + 2) / 153;
    let d = doy - (153 * mp + 2) / 5 + 1;
    let m = if mp < 10 { mp + 3 } else { mp - 9 };
    (y as i32 + if m <= 2 { 1 } else { 0 }, m, d)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_jakarta_prayer_times_ranges() {
        let loc = GeoLocation::default();
        let prayers = calculate_prayer_times(2026, 11, 26, loc);

        // Subuh di Jakarta biasanya antara 04:00 - 04:45
        assert!(prayers.fajr >= 4.0 && prayers.fajr <= 4.75);
        // Dzuhur antara 11:30 - 12:15
        assert!(prayers.dhuhr >= 11.5 && prayers.dhuhr <= 12.25);
        // Maghrib antara 17:45 - 18:25
        assert!(prayers.maghrib >= 17.75 && prayers.maghrib <= 18.5);
    }

    #[test]
    fn test_current_prayer_status() {
        let loc = GeoLocation::default();
        let (status, schedule) = get_current_prayer_status(loc);
        assert!(!status.current_name.is_empty());
        assert!(!status.next_name.is_empty());
        assert!(!status.countdown_str.is_empty());
        assert_eq!(schedule.len(), 6);
    }
}
