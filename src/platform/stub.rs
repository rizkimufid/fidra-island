//! Non-Windows platform stubs so Fidra Island remains compilable cross-platform.

use super::{SystemStatus, TrayAction};
use std::path::Path;
use std::sync::mpsc;

/// No-op COM guard on non-Windows platforms.
pub struct ComGuard;

impl ComGuard {
    /// No-op initialization.
    pub fn init_mta() -> Self {
        Self
    }
}

/// Returns a static placeholder status on non-Windows targets.
pub fn query_system_status() -> SystemStatus {
    SystemStatus {
        time_str: "12:00".to_string(),
        battery_pct: 100,
        is_charging: false,
    }
}

/// Positions the window at `(0, 0)` on non-Windows targets.
pub fn position_overlay(window: &slint::Window) {
    window.set_position(slint::PhysicalPosition::new(0, 0));
}

/// No-op overlay visibility toggle on non-Windows targets.
pub fn set_overlay_visible(_visible: bool) {}

/// No-op startup registration on non-Windows targets.
pub fn set_launch_at_startup(_enabled: bool) -> Result<(), String> {
    Ok(())
}

/// No-op log folder opener on non-Windows targets.
pub fn open_logs_dir(_path: &Path) {}

/// No-op tray and window subclass setup on non-Windows targets.
pub fn setup_window_and_tray(_tray_tx: mpsc::Sender<TrayAction>) {}

/// No-op tray removal on non-Windows targets.
pub fn remove_tray_icon() {}
