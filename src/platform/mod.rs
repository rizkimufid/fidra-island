//! Platform abstraction layer for windowing, hit-testing, system tray,
//! auto-start registration, COM initialization, and system telemetry.

use std::path::Path;
use std::sync::atomic::{AtomicBool, AtomicI32, Ordering};

/// Fixed logical width of the transparent root bounding window (Slint logical px).
pub const WINDOW_WIDTH: i32 = 680;
/// Logical resting compact island width.
pub const COMPACT_WIDTH: i32 = 290;
/// Logical hovered compact island width.
pub const COMPACT_HOVER_WIDTH: i32 = 310;
/// Minimum logical compact island width when displaying a synced lyric line.
pub const COMPACT_LYRIC_MIN_WIDTH: f32 = 280.0;
/// Maximum logical compact island width when displaying a synced lyric line.
pub const COMPACT_LYRIC_MAX_WIDTH: f32 = 420.0;
/// Logical expanded island width (player-only).
pub const EXPANDED_WIDTH: i32 = 320;
/// Logical expanded island width (2-column player + lyrics).
pub const EXPANDED_LYRICS_WIDTH: i32 = 610;
/// Logical compact island height.
pub const COMPACT_HEIGHT: i32 = 38;
/// Logical expanded island height.
pub const EXPANDED_HEIGHT: i32 = 175;
/// Horizontal width of each tangential top-edge ear flare.
pub const EAR_FLARE_WIDTH: i32 = 14;

/// Actions triggered by the native system tray icon and context menu.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TrayAction {
    /// Show or hide the Dynamic Island overlay.
    ToggleVisibility,
    /// Toggle live synced lyrics on or off.
    ToggleLyrics,
    /// Toggle Windows sign-in auto-start registration.
    ToggleStartup,
    /// Open the rotating logs directory in the OS file explorer.
    OpenLogs,
    /// Gracefully exit Fidra Island.
    Quit,
}

/// System clock and battery telemetry snapshot.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SystemStatus {
    /// Local wall-clock time formatted as `HH:mm`.
    pub time_str: String,
    /// Battery charge percentage (`0..=100`).
    pub battery_pct: u8,
    /// Whether AC power is connected.
    pub is_charging: bool,
}

static IS_EXPANDED: AtomicBool = AtomicBool::new(false);
static IS_HOVERED: AtomicBool = AtomicBool::new(false);
static SHOW_LYRICS: AtomicBool = AtomicBool::new(false);
static LYRICS_ENABLED: AtomicBool = AtomicBool::new(false);
static IS_HIDDEN: AtomicBool = AtomicBool::new(false);
static LAUNCH_AT_STARTUP: AtomicBool = AtomicBool::new(false);
static CURRENT_LYRIC_WIDTH: AtomicI32 = AtomicI32::new(COMPACT_WIDTH);

/// Updates whether the island is currently expanded (used by hit-testing).
pub fn set_expanded(v: bool) {
    IS_EXPANDED.store(v, Ordering::Relaxed);
}

/// Updates whether the pointer is currently hovering the island.
pub fn set_hovered(v: bool) {
    IS_HOVERED.store(v, Ordering::Relaxed);
}

/// Updates whether compact mode is actively showing a synced lyric line.
pub fn set_show_lyrics(v: bool) {
    SHOW_LYRICS.store(v, Ordering::Relaxed);
}

/// Updates whether synced lyrics are enabled (used by hit-testing & tray menu checkmarks).
pub fn set_lyrics_enabled(v: bool) {
    LYRICS_ENABLED.store(v, Ordering::Relaxed);
}

/// Updates whether the island overlay is currently hidden (used by tray menu labels).
pub fn set_hidden_state(v: bool) {
    IS_HIDDEN.store(v, Ordering::Relaxed);
}

/// Updates whether auto-start is enabled (used by tray menu checkmarks).
pub fn set_startup_state(v: bool) {
    LAUNCH_AT_STARTUP.store(v, Ordering::Relaxed);
}

/// Updates the dynamic compact lyric width in logical pixels.
pub fn set_current_lyric_width(w: i32) {
    CURRENT_LYRIC_WIDTH.store(w, Ordering::Relaxed);
}

#[cfg(target_os = "windows")]
pub(crate) fn hit_test_flags() -> (bool, bool, bool, bool, i32) {
    (
        IS_EXPANDED.load(Ordering::Relaxed),
        IS_HOVERED.load(Ordering::Relaxed),
        SHOW_LYRICS.load(Ordering::Relaxed),
        LYRICS_ENABLED.load(Ordering::Relaxed),
        CURRENT_LYRIC_WIDTH.load(Ordering::Relaxed),
    )
}

#[cfg(target_os = "windows")]
pub(crate) fn tray_menu_flags() -> (bool, bool, bool) {
    (
        IS_HIDDEN.load(Ordering::Relaxed),
        LYRICS_ENABLED.load(Ordering::Relaxed),
        LAUNCH_AT_STARTUP.load(Ordering::Relaxed),
    )
}

/// Builds an SVG arc path for the 16x16 circular battery progress ring (`pct` in `0.0..=1.0`).
pub fn get_battery_arc_path(pct: f32) -> String {
    let pct = pct.clamp(0.01, 1.0);
    if pct >= 0.999 {
        return "M 8 2 A 6 6 0 1 1 7.99 2 Z".to_string();
    }
    let angle = pct * 2.0 * std::f32::consts::PI;
    let x = 8.0 + 6.0 * angle.sin();
    let y = 8.0 - 6.0 * angle.cos();
    let large_arc = if pct > 0.5 { 1 } else { 0 };
    format!("M 8 2 A 6 6 0 {} 1 {:.2} {:.2}", large_arc, x, y)
}

#[cfg(target_os = "windows")]
mod windows;
#[cfg(target_os = "windows")]
pub use windows::{
    open_logs_dir, position_overlay, query_system_status, remove_tray_icon, set_launch_at_startup,
    set_overlay_visible, setup_window_and_tray, ComGuard,
};

#[cfg(not(target_os = "windows"))]
mod stub;
#[cfg(not(target_os = "windows"))]
pub use stub::{
    open_logs_dir, position_overlay, query_system_status, remove_tray_icon, set_launch_at_startup,
    set_overlay_visible, setup_window_and_tray, ComGuard,
};

/// Convenience helper for opening the application's log directory.
pub fn open_log_folder(path: &Path) {
    open_logs_dir(path);
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn battery_arc_path_handles_full_and_partial_levels() {
        assert_eq!(get_battery_arc_path(1.0), "M 8 2 A 6 6 0 1 1 7.99 2 Z");
        assert_eq!(get_battery_arc_path(1.5), "M 8 2 A 6 6 0 1 1 7.99 2 Z");

        let half = get_battery_arc_path(0.5);
        assert!(half.starts_with("M 8 2 A 6 6 0 0 1 "));

        let three_quarter = get_battery_arc_path(0.75);
        assert!(three_quarter.starts_with("M 8 2 A 6 6 0 1 1 "));
    }
}
