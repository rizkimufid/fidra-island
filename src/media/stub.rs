//! Cross-platform no-op media controller stub for non-Windows builds.

use super::MediaSnapshot;

/// Cross-platform no-op media controller stub.
pub struct MediaController;

impl MediaController {
    /// Creates a new stub controller.
    pub fn new() -> Self {
        Self
    }
    /// Returns an idle [`MediaSnapshot`].
    pub fn poll(&mut self) -> Result<MediaSnapshot, ()> {
        Ok(MediaSnapshot::default())
    }
    /// No-op on non-Windows targets.
    pub fn toggle_playback(&self) {}
    /// No-op on non-Windows targets.
    pub fn skip_next(&self) {}
    /// No-op on non-Windows targets.
    pub fn skip_previous(&self) {}
    /// No-op on non-Windows targets.
    pub fn seek_fraction(&mut self, _fraction: f32) {}
    /// No-op on non-Windows targets.
    pub fn seek_seconds(&mut self, _seconds: f64) {}
}
