//! Operating-system media session integration and cover art color analysis.

pub mod color;
#[cfg(target_os = "windows")]
mod gsmtc;
#[cfg(not(target_os = "windows"))]
mod stub;

#[allow(unused_imports)]
pub use color::{extract_dominant_color, DEFAULT_AMBIENT_COLOR};
#[cfg(target_os = "windows")]
pub use gsmtc::MediaController;
#[cfg(not(target_os = "windows"))]
pub use stub::MediaController;

/// Incremental update state for album artwork bytes.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub enum ThumbnailUpdate {
    /// Album artwork has not changed since the previous poll.
    #[default]
    Unchanged,
    /// Album artwork was cleared (no active session or track has no thumbnail).
    Cleared,
    /// New encoded image bytes (PNG/JPEG) ready for decoding on the UI thread.
    Updated(Vec<u8>),
}

/// Point-in-time snapshot of the active system media session.
#[derive(Clone, Debug)]
pub struct MediaSnapshot {
    /// Current track title, or `"No Media Playing"` when idle.
    pub title: String,
    /// Current artist name.
    pub artist: String,
    /// Current album title (may be empty).
    pub album: String,
    /// Monotonically extrapolated playback position in seconds.
    pub elapsed_seconds: f32,
    /// Total track duration in seconds (`0.0` if unknown).
    pub duration_seconds: f32,
    /// Whether the active session is currently playing.
    pub is_playing: bool,
    /// Delta update for album artwork.
    pub thumbnail: ThumbnailUpdate,
    /// Dominant vibrant ambient RGB color extracted from artwork.
    pub ambient_color: (u8, u8, u8),
}

impl Default for MediaSnapshot {
    fn default() -> Self {
        Self {
            title: "No Media Playing".into(),
            artist: "Fidra Island".into(),
            album: String::new(),
            elapsed_seconds: 0.0,
            duration_seconds: 0.0,
            is_playing: false,
            thumbnail: ThumbnailUpdate::Unchanged,
            ambient_color: DEFAULT_AMBIENT_COLOR,
        }
    }
}

/// Commands dispatched from the Slint UI thread to the media worker thread.
#[derive(Debug)]
pub enum MediaCommand {
    /// Toggle play/pause on the active GSMTC session.
    TogglePlayback,
    /// Skip to the next track.
    Next,
    /// Skip to the previous track.
    Previous,
    /// Seek to a normalized fraction (`0.0..=1.0`) of track duration.
    SeekFraction(f32),
    /// Seek to an absolute playback timestamp in seconds.
    SeekSeconds(f64),
    /// Enable or disable live synced-lyrics lookup.
    SetLyricsEnabled(bool),
}
