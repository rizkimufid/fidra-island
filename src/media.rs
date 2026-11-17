#[derive(Clone, Debug)]
pub struct MediaSnapshot {
    pub title: String,
    pub artist: String,
    pub album: String,
    pub elapsed_seconds: f32,
    pub duration_seconds: f32,
    pub is_playing: bool,
    pub thumbnail_data: Option<Vec<u8>>,
}

impl Default for MediaSnapshot {
    fn default() -> Self {
        Self {
            title: "No Media Playing".into(),
            artist: "FIRA Ambient".into(),
            album: String::new(),
            elapsed_seconds: 0.0,
            duration_seconds: 0.0,
            is_playing: false,
            thumbnail_data: None,
        }
    }
}

#[derive(Debug)]
pub enum MediaCommand {
    TogglePlayback,
    Next,
    Previous,
    SeekFraction(f32),
    SeekSeconds(f64),
    ToggleLyrics,
}

#[cfg(target_os = "windows")]
mod windows_media {
    use super::MediaSnapshot;
    use windows::Media::Control::{
        GlobalSystemMediaTransportControlsSession,
        GlobalSystemMediaTransportControlsSessionManager,
        GlobalSystemMediaTransportControlsSessionPlaybackStatus,
    };
    use windows::Storage::Streams::DataReader;

    pub struct MediaController {
        manager: Option<GlobalSystemMediaTransportControlsSessionManager>,
        session: Option<GlobalSystemMediaTransportControlsSession>,
        last_thumb_title: String,
        cached_thumbnail: Option<Vec<u8>>,
    }

    impl MediaController {
        pub fn new() -> Self {
            let manager = GlobalSystemMediaTransportControlsSessionManager::RequestAsync()
                .ok()
                .and_then(|op| op.get().ok());
            Self {
                manager,
                session: None,
                last_thumb_title: String::new(),
                cached_thumbnail: None,
            }
        }

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
                None => return Ok(MediaSnapshot::default()),
            };

            let mut snapshot = MediaSnapshot::default();

            if let Ok(info) = session.GetPlaybackInfo() {
                if let Ok(status) = info.PlaybackStatus() {
                    snapshot.is_playing =
                        status == GlobalSystemMediaTransportControlsSessionPlaybackStatus::Playing;
                }
            }

            if let Ok(timeline) = session.GetTimelineProperties() {
                let mut reported_pos = 0.0f32;
                if let Ok(pos) = timeline.Position() {
                    reported_pos = (pos.Duration as f32) / 10_000_000.0;
                }
                if let Ok(end) = timeline.EndTime() {
                    snapshot.duration_seconds = (end.Duration as f32) / 10_000_000.0;
                }

                // Calculate real-time elapsed position using LastUpdatedTime
                if snapshot.is_playing {
                    if let Ok(last_updated) = timeline.LastUpdatedTime() {
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
                        let updated_ticks = last_updated.UniversalTime as u64;
                        if now_ticks > updated_ticks {
                            let age_seconds = ((now_ticks - updated_ticks) as f32) / 10_000_000.0;
                            if age_seconds >= 0.0 && age_seconds < 86400.0 {
                                reported_pos += age_seconds;
                            }
                        }
                    }
                }

                snapshot.elapsed_seconds = reported_pos;
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

                    // Check if song changed to invalidate thumbnail cache
                    let song_key = format!("{} - {}", snapshot.title, snapshot.artist);
                    if song_key != self.last_thumb_title {
                        self.last_thumb_title = song_key;
                        self.cached_thumbnail = None;

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
                                                        self.cached_thumbnail = Some(buf);
                                                    }
                                                }
                                            }
                                        }
                                    }
                                }
                            }
                        }
                    }

                    snapshot.thumbnail_data = self.cached_thumbnail.clone();
                }
            }

            Ok(snapshot)
        }

        pub fn toggle_playback(&self) {
            if let Some(session) = &self.session {
                let _ = session.TryTogglePlayPauseAsync();
            }
        }

        pub fn skip_next(&self) {
            if let Some(session) = &self.session {
                let _ = session.TrySkipNextAsync();
            }
        }

        pub fn skip_previous(&self) {
            if let Some(session) = &self.session {
                let _ = session.TrySkipPreviousAsync();
            }
        }

        pub fn seek_fraction(&self, fraction: f32) {
            if let Some(session) = &self.session {
                if let Ok(timeline) = session.GetTimelineProperties() {
                    if let Ok(end) = timeline.EndTime() {
                        let target_ticks = (end.Duration as f64 * fraction as f64) as i64;
                        let _ = session.TryChangePlaybackPositionAsync(target_ticks);
                    }
                }
            }
        }

        pub fn seek_seconds(&self, seconds: f64) {
            if let Some(session) = &self.session {
                let target_ticks = (seconds * 10_000_000.0) as i64;
                let _ = session.TryChangePlaybackPositionAsync(target_ticks);
            }
        }
    }
}

#[cfg(not(target_os = "windows"))]
mod stub_media {
    use super::MediaSnapshot;

    pub struct MediaController;

    impl MediaController {
        pub fn new() -> Self {
            Self
        }
        pub fn poll(&mut self) -> Result<MediaSnapshot, ()> {
            Ok(MediaSnapshot::default())
        }
        pub fn toggle_playback(&self) {}
        pub fn skip_next(&self) {}
        pub fn skip_previous(&self) {}
        pub fn seek_fraction(&self, _fraction: f32) {}
        pub fn seek_seconds(&self, _seconds: f64) {}
    }
}

#[cfg(target_os = "windows")]
pub use windows_media::MediaController;

#[cfg(not(target_os = "windows"))]
pub use stub_media::MediaController;
