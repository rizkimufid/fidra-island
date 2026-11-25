//! Windows GSMTC media controller implementation.

use super::color::{extract_dominant_color, DEFAULT_AMBIENT_COLOR};
use super::{MediaSnapshot, ThumbnailUpdate};
use std::time::Instant;
use windows::Media::Control::{
    GlobalSystemMediaTransportControlsSession, GlobalSystemMediaTransportControlsSessionManager,
    GlobalSystemMediaTransportControlsSessionPlaybackStatus,
};
use windows::Storage::Streams::DataReader;

/// Windows GSMTC media controller bound to a background worker thread.
pub struct MediaController {
    manager: Option<GlobalSystemMediaTransportControlsSessionManager>,
    session: Option<GlobalSystemMediaTransportControlsSession>,
    last_thumb_key: String,
    has_sent_thumbnail: bool,
    last_song_key: String,
    position_anchor: Option<(f32, Instant)>,
    last_timeline_updated: Option<i64>,
    pending_seek: Option<(f32, Instant)>,
    last_title: String,
    last_artist: String,
    last_album: String,
    last_duration_seconds: f32,
    last_playing: bool,
    last_ambient_color: (u8, u8, u8),
}

impl MediaController {
    /// Creates a new controller and attempts to acquire the Windows GSMTC session manager.
    pub fn new() -> Self {
        let manager = GlobalSystemMediaTransportControlsSessionManager::RequestAsync()
            .ok()
            .and_then(|op| op.get().ok());
        Self {
            manager,
            session: None,
            last_thumb_key: String::new(),
            has_sent_thumbnail: false,
            last_song_key: String::new(),
            position_anchor: None,
            last_timeline_updated: None,
            pending_seek: None,
            last_title: String::new(),
            last_artist: String::new(),
            last_album: String::new(),
            last_duration_seconds: 0.0,
            last_playing: false,
            last_ambient_color: DEFAULT_AMBIENT_COLOR,
        }
    }

    /// Polls the current Windows media session and returns an extrapolated [`MediaSnapshot`].
    pub fn poll(&mut self) -> Result<MediaSnapshot, ()> {
        let manager = match &self.manager {
            Some(m) => m,
            None => {
                self.manager = GlobalSystemMediaTransportControlsSessionManager::RequestAsync()
                    .ok()
                    .and_then(|op| op.get().ok());
                self.manager.as_ref().ok_or(())?
            }
        };

        let session = manager.GetCurrentSession().ok();
        self.session = session;

        let session = match &self.session {
            Some(s) => s,
            None => {
                self.last_song_key.clear();
                self.last_thumb_key.clear();
                self.position_anchor = None;
                self.last_timeline_updated = None;
                self.pending_seek = None;
                self.last_title.clear();
                self.last_artist.clear();
                self.last_album.clear();
                self.last_duration_seconds = 0.0;
                self.last_playing = false;
                self.last_ambient_color = DEFAULT_AMBIENT_COLOR;

                let mut snapshot = MediaSnapshot::default();
                if self.has_sent_thumbnail {
                    self.has_sent_thumbnail = false;
                    snapshot.thumbnail = ThumbnailUpdate::Cleared;
                }
                return Ok(snapshot);
            }
        };

        let mut snapshot = MediaSnapshot::default();
        if !self.last_title.is_empty() {
            snapshot.title = self.last_title.clone();
        }
        if !self.last_artist.is_empty() {
            snapshot.artist = self.last_artist.clone();
        }
        snapshot.album = self.last_album.clone();
        snapshot.is_playing = self.last_playing;
        snapshot.ambient_color = self.last_ambient_color;

        if let Ok(info) = session.GetPlaybackInfo() {
            if let Ok(status) = info.PlaybackStatus() {
                self.last_playing =
                    status == GlobalSystemMediaTransportControlsSessionPlaybackStatus::Playing;
                snapshot.is_playing = self.last_playing;
            }
        }

        let timeline_read_at = Instant::now();
        let mut timeline_sample = None;
        if let Ok(timeline) = session.GetTimelineProperties() {
            if let Ok(end) = timeline.EndTime() {
                let duration = (end.Duration as f32) / 10_000_000.0;
                if duration.is_finite() && duration > 0.0 {
                    snapshot.duration_seconds = duration;
                }
            }

            if let Ok(pos) = timeline.Position() {
                let mut reported_pos = (pos.Duration as f32) / 10_000_000.0;
                let sample_updated = timeline
                    .LastUpdatedTime()
                    .ok()
                    .map(|updated| updated.UniversalTime);

                if snapshot.is_playing {
                    if let Some(updated_ticks) = sample_updated {
                        let mut ft = unsafe {
                            std::mem::zeroed::<windows_sys::Win32::Foundation::FILETIME>()
                        };
                        unsafe {
                            windows_sys::Win32::System::SystemInformation::GetSystemTimeAsFileTime(
                                &mut ft,
                            )
                        };
                        let now_ticks =
                            ((ft.dwHighDateTime as u64) << 32) | (ft.dwLowDateTime as u64);
                        let updated_ticks = updated_ticks as u64;
                        if now_ticks > updated_ticks {
                            let age_seconds =
                                ((now_ticks - updated_ticks) as f32) / 10_000_000.0;
                            if age_seconds < 86400.0 {
                                reported_pos += age_seconds;
                            }
                        }
                    }
                }
                if reported_pos.is_finite() && reported_pos >= 0.0 {
                    timeline_sample = Some((reported_pos, sample_updated));
                }
            }
        }

        if let Ok(media_props_op) = session.TryGetMediaPropertiesAsync() {
            if let Ok(props) = media_props_op.get() {
                if let Ok(title) = props.Title() {
                    let t = title.to_string();
                    if !t.is_empty() {
                        snapshot.title = t;
                    }
                }
                if let Ok(artist) = props.Artist() {
                    let a = artist.to_string();
                    if !a.is_empty() {
                        snapshot.artist = a;
                    }
                }
                if let Ok(album) = props.AlbumTitle() {
                    snapshot.album = album.to_string();
                }

                let song_key = format!("{} - {}", snapshot.title, snapshot.artist);
                if song_key != self.last_thumb_key {
                    self.last_thumb_key = song_key;
                    let mut loaded: Option<Vec<u8>> = None;

                    if let Ok(thumb_ref) = props.Thumbnail() {
                        if let Ok(stream_op) = thumb_ref.OpenReadAsync() {
                            if let Ok(stream) = stream_op.get() {
                                let size = stream.Size().unwrap_or(0) as usize;
                                if size > 0 && size < 5_000_000 {
                                    if let Ok(reader) = DataReader::CreateDataReader(&stream) {
                                        if let Ok(load_op) = reader.LoadAsync(size as u32) {
                                            if load_op.get().is_ok() {
                                                let mut buf = vec![0u8; size];
                                                if reader.ReadBytes(&mut buf).is_ok() {
                                                    loaded = Some(buf);
                                                }
                                            }
                                        }
                                    }
                                }
                            }
                        }
                    }

                    match loaded {
                        Some(bytes) => {
                            self.has_sent_thumbnail = true;
                            let color = extract_dominant_color(&bytes).unwrap_or(DEFAULT_AMBIENT_COLOR);
                            self.last_ambient_color = color;
                            snapshot.ambient_color = color;
                            snapshot.thumbnail = ThumbnailUpdate::Updated(bytes);
                        }
                        None => {
                            self.has_sent_thumbnail = false;
                            self.last_ambient_color = DEFAULT_AMBIENT_COLOR;
                            snapshot.ambient_color = DEFAULT_AMBIENT_COLOR;
                            snapshot.thumbnail = ThumbnailUpdate::Cleared;
                        }
                    }
                }
            }
        }

        let song_key = format!("{} - {}", snapshot.title, snapshot.artist);
        let track_changed =
            snapshot.title != "No Media Playing" && song_key != self.last_song_key;
        if track_changed {
            self.last_song_key = song_key;
            self.position_anchor = None;
            self.last_timeline_updated = None;
            self.pending_seek = None;
            self.last_duration_seconds = 0.0;
        }
        if snapshot.title != "No Media Playing" {
            self.last_title = snapshot.title.clone();
            self.last_artist = snapshot.artist.clone();
            self.last_album = snapshot.album.clone();
        }
        if snapshot.duration_seconds > 0.0 {
            self.last_duration_seconds = snapshot.duration_seconds;
        } else {
            snapshot.duration_seconds = self.last_duration_seconds;
        }

        let now = Instant::now();
        let fallback_position = self.position_anchor.map(|(position, updated)| {
            if snapshot.is_playing {
                position + now.duration_since(updated).as_secs_f32()
            } else {
                position
            }
        });
        let duration = snapshot.duration_seconds;
        let reported = timeline_sample.and_then(|(position, sample_updated)| {
            let seek_match = self
                .pending_seek
                .map(|(target, _)| (position - target).abs() <= 2.0)
                .unwrap_or(false);
            let is_new_sample = match (sample_updated, self.last_timeline_updated) {
                (Some(updated), Some(previous)) => updated > previous,
                (Some(_), None) => true,
                (None, _) => {
                    self.position_anchor.is_none()
                        || fallback_position
                            .map(|fallback| position > fallback + 0.25)
                            .unwrap_or(false)
                }
            };

            if seek_match {
                self.pending_seek = None;
            } else if let Some((target, seek_time)) = self.pending_seek {
                if now.duration_since(seek_time).as_secs_f32() < 2.0 {
                    return Some((fallback_position.unwrap_or(target), false));
                }
                self.pending_seek = None;
            }

            if is_new_sample || seek_match {
                if let Some(updated) = sample_updated {
                    self.last_timeline_updated = Some(updated);
                }
                Some((position, true))
            } else {
                fallback_position.map(|position| (position, false))
            }
        });
        let (mut position, anchored_from_sample) = reported
            .or_else(|| fallback_position.map(|position| (position, false)))
            .unwrap_or((0.0, false));
        if anchored_from_sample && snapshot.is_playing {
            position += now.duration_since(timeline_read_at).as_secs_f32();
        }
        let position = if duration > 0.0 {
            position.clamp(0.0, duration)
        } else {
            position.max(0.0)
        };
        self.position_anchor = Some((position, now));
        snapshot.elapsed_seconds = position;

        Ok(snapshot)
    }

    /// Toggles playback state without blocking on the WinRT operation.
    pub fn toggle_playback(&self) {
        if let Some(session) = &self.session {
            let _ = session.TryTogglePlayPauseAsync();
        }
    }

    /// Skips to the next track without blocking on the WinRT operation.
    pub fn skip_next(&self) {
        if let Some(session) = &self.session {
            let _ = session.TrySkipNextAsync();
        }
    }

    /// Skips to the previous track without blocking on the WinRT operation.
    pub fn skip_previous(&self) {
        if let Some(session) = &self.session {
            let _ = session.TrySkipPreviousAsync();
        }
    }

    /// Seeks to a normalized fraction (`0.0..=1.0`) of track duration.
    pub fn seek_fraction(&mut self, fraction: f32) {
        if let Some(session) = self.session.clone() {
            if let Ok(timeline) = session.GetTimelineProperties() {
                if let Ok(end) = timeline.EndTime() {
                    let target = (end.Duration as f64 / 10_000_000.0
                        * fraction.clamp(0.0, 1.0) as f64)
                        as f32;
                    self.set_seek_anchor(target);
                    let target_ticks = (target as f64 * 10_000_000.0) as i64;
                    let _ = session.TryChangePlaybackPositionAsync(target_ticks);
                }
            }
        }
    }

    /// Seeks directly to an absolute timestamp in seconds.
    pub fn seek_seconds(&mut self, seconds: f64) {
        if let Some(session) = self.session.clone() {
            let target = seconds.max(0.0) as f32;
            self.set_seek_anchor(target);
            let target_ticks = (target as f64 * 10_000_000.0) as i64;
            let _ = session.TryChangePlaybackPositionAsync(target_ticks);
        }
    }

    fn set_seek_anchor(&mut self, seconds: f32) {
        let target = seconds.max(0.0);
        let now = Instant::now();
        self.position_anchor = Some((target, now));
        self.pending_seek = Some((target, now));
    }
}
