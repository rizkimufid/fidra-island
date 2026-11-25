//! Main application wiring: Slint event loop, worker threads, persistent config,
//! zero-CPU idle timers, and system tray action handling.

use std::{
    cell::{Cell, RefCell},
    collections::HashMap,
    rc::Rc,
    sync::{
        atomic::{AtomicBool, Ordering},
        mpsc, Arc, Mutex,
    },
    thread,
    time::Duration,
};

use slint::{ComponentHandle, Timer, TimerMode};

use crate::{
    config::{self, AppConfig},
    logging,
    lyrics::{self, LyricContext},
    media::{MediaCommand, MediaController, MediaSnapshot, ThumbnailUpdate},
    platform::{self, TrayAction, COMPACT_LYRIC_MAX_WIDTH, COMPACT_LYRIC_MIN_WIDTH},
    MainWindow,
};

/// Maximum cached tracks held in the in-memory lyric lookup cache.
const LYRIC_CACHE_CAPACITY: usize = 64;

#[derive(Default)]
struct SystemStatusCache {
    time_str: String,
    battery_pct: Option<u8>,
    is_charging: Option<bool>,
}

/// Boots the Slint UI, spawns background workers, registers Win32 hit-test & tray,
/// and runs the main event loop until quit.
pub fn run() -> Result<(), slint::PlatformError> {
    logging::init();
    logging::info(
        "app",
        format!("Starting Fidra Island v{}", env!("CARGO_PKG_VERSION")),
    );

    let initial_cfg = config::load();
    let shared_cfg = Arc::new(Mutex::new(initial_cfg.clone()));

    let window = MainWindow::new()?;
    let shutdown = Arc::new(AtomicBool::new(false));

    // Apply persisted user preferences at startup.
    window.set_lyrics_enabled(initial_cfg.lyrics_enabled);
    window.set_is_pinned(initial_cfg.is_pinned);
    platform::set_lyrics_enabled(initial_cfg.lyrics_enabled);
    platform::set_startup_state(initial_cfg.launch_at_startup);

    // Shared lyric context for instant interactive seek-to-lyric callbacks.
    let lyric_context_shared = Arc::new(Mutex::new(LyricContext::default()));

    // Dedicated lyric fetcher thread with request deduplication and bounded cache.
    let (lyric_fetch_tx, lyric_fetch_rx) = mpsc::channel::<(String, String, String, f64)>();
    let (lyric_result_tx, lyric_result_rx) =
        mpsc::channel::<(String, Option<lyrics::LyricsData>)>();
    let lyric_shutdown = Arc::clone(&shutdown);
    thread::spawn(move || {
        let mut cache: HashMap<String, Option<lyrics::LyricsData>> = HashMap::new();

        while let Ok(mut req) = lyric_fetch_rx.recv() {
            if lyric_shutdown.load(Ordering::Relaxed) {
                break;
            }
            while let Ok(newer) = lyric_fetch_rx.try_recv() {
                req = newer;
            }

            let (song_key, title, artist, duration) = req;
            let result = if let Some(cached) = cache.get(&song_key) {
                cached.clone()
            } else {
                let fetched = lyrics::fetch_lyrics(&title, &artist, duration);
                if cache.len() >= LYRIC_CACHE_CAPACITY {
                    cache.clear();
                }
                cache.insert(song_key.clone(), fetched.clone());
                fetched
            };

            if lyric_result_tx.send((song_key, result)).is_err() {
                break;
            }
        }
    });

    // Channel for dispatching commands from UI thread to background media worker.
    let (cmd_tx, cmd_rx) = mpsc::channel::<MediaCommand>();
    if initial_cfg.lyrics_enabled {
        let _ = cmd_tx.send(MediaCommand::SetLyricsEnabled(true));
    }

    // Generation counter prevents stale lyric transition timers from cancelling newer lines.
    let lyric_transition_gen = Rc::new(Cell::new(0u64));

    // Live spectrum visualizer timer driving 4 animated bars (strictly stopped when paused).
    let eq_timer = Rc::new(Timer::default());
    let eq_running = Rc::new(Cell::new(false));
    let eq_tick = Rc::new(Cell::new(0.0f32));

    let worker_eq_timer_sync = window.as_weak();
    let eq_timer_for_ui = Rc::clone(&eq_timer);
    let eq_running_for_ui = Rc::clone(&eq_running);
    let eq_tick_for_ui = Rc::clone(&eq_tick);
    let transition_gen_for_ui = Rc::clone(&lyric_transition_gen);

    let sync_eq_state = Rc::new(move |is_playing: bool| {
        if is_playing && !eq_running_for_ui.get() {
            eq_running_for_ui.set(true);
            let eq_weak = worker_eq_timer_sync.clone();
            let tick = Rc::clone(&eq_tick_for_ui);
            eq_timer_for_ui.start(TimerMode::Repeated, Duration::from_millis(80), move || {
                if let Some(w) = eq_weak.upgrade() {
                    let t = tick.get() + 0.28;
                    tick.set(t);
                    let b1 =
                        3.2 + 5.0 * ((t * 1.6).sin().powi(2)) + 3.8 * ((t * 3.2).cos() * 0.5 + 0.5);
                    let b2 = 3.0
                        + 6.0 * ((t * 2.1 + 0.7).sin() * 0.5 + 0.5)
                        + 3.0 * ((t * 0.9).cos() * 0.5 + 0.5);
                    let b3 = 3.5
                        + 5.5 * ((t * 1.8 + 1.8).sin() * 0.5 + 0.5)
                        + 3.0 * ((t * 2.7).sin() * 0.5 + 0.5);
                    let b4 = 3.0
                        + 4.5 * ((t * 3.1 + 2.4).sin() * 0.5 + 0.5)
                        + 4.5 * ((t * 1.4).cos() * 0.5 + 0.5);

                    w.set_eq_bar1(b1.clamp(3.0, 12.0));
                    w.set_eq_bar2(b2.clamp(3.0, 12.0));
                    w.set_eq_bar3(b3.clamp(3.0, 12.0));
                    w.set_eq_bar4(b4.clamp(3.0, 12.0));
                }
            });
        } else if !is_playing && eq_running_for_ui.get() {
            eq_running_for_ui.set(false);
            eq_timer_for_ui.stop();
            if let Some(w) = worker_eq_timer_sync.upgrade() {
                w.set_eq_bar1(3.0);
                w.set_eq_bar2(3.0);
                w.set_eq_bar3(3.0);
                w.set_eq_bar4(3.0);
            }
        }
    });

    type EqSyncCallback = Rc<dyn Fn(bool)>;
    thread_local! {
        static SYNC_EQ_CB: RefCell<Option<EqSyncCallback>> = const { RefCell::new(None) };
        static TRANSITION_GEN: RefCell<Option<Rc<Cell<u64>>>> = const { RefCell::new(None) };
    }
    SYNC_EQ_CB.with(|slot| *slot.borrow_mut() = Some(sync_eq_state));
    TRANSITION_GEN.with(|slot| *slot.borrow_mut() = Some(transition_gen_for_ui));

    // Spawn background media controller worker thread with explicit COM MTA & "Doze Off" idle.
    let worker_window = window.as_weak();
    let worker_lyric_ctx = Arc::clone(&lyric_context_shared);
    let worker_shutdown = Arc::clone(&shutdown);
    thread::spawn(move || {
        let _com_guard = platform::ComGuard::init_mta();
        let mut controller = MediaController::new();
        let mut is_playing = false;
        let mut lyrics_enabled = initial_cfg.lyrics_enabled;
        let mut cached_lyrics: Option<lyrics::LyricsData> = None;
        let mut last_song_key = String::new();
        let mut last_lyric_line = String::new();

        while !worker_shutdown.load(Ordering::Relaxed) {
            while let Ok((song_key, result)) = lyric_result_rx.try_recv() {
                if song_key == last_song_key {
                    cached_lyrics = result;
                }
            }

            while let Ok(cmd) = cmd_rx.try_recv() {
                match cmd {
                    MediaCommand::TogglePlayback => controller.toggle_playback(),
                    MediaCommand::Next => controller.skip_next(),
                    MediaCommand::Previous => controller.skip_previous(),
                    MediaCommand::SeekFraction(fraction) => controller.seek_fraction(fraction),
                    MediaCommand::SeekSeconds(seconds) => controller.seek_seconds(seconds),
                    MediaCommand::SetLyricsEnabled(enabled) => lyrics_enabled = enabled,
                }
            }

            if let Ok(snapshot) = controller.poll() {
                is_playing = snapshot.is_playing;
                let current_song_key = format!("{} - {}", snapshot.title, snapshot.artist);

                let mut current_lyric_text = String::new();
                let mut prev_lyric_text = String::new();
                let mut next_lyric_text = String::new();
                let mut word_before = String::new();
                let mut word_current = String::new();
                let mut word_after = String::new();
                let mut has_word_sync = false;
                let mut has_lyrics = false;

                if lyrics_enabled && snapshot.title != "No Media Playing" {
                    if current_song_key != last_song_key {
                        last_song_key = current_song_key.clone();
                        cached_lyrics = None;
                        let _ = lyric_fetch_tx.send((
                            current_song_key.clone(),
                            snapshot.title.clone(),
                            snapshot.artist.clone(),
                            snapshot.duration_seconds as f64,
                        ));
                    }

                    if let Some(lyrics_data) = &cached_lyrics {
                        let ctx = lyrics_data
                            .get_lyrics_context_detailed(snapshot.elapsed_seconds as f64);
                        prev_lyric_text = ctx.prev_text.clone();
                        current_lyric_text = ctx.curr_text.clone();
                        next_lyric_text = ctx.next_text.clone();
                        word_before = ctx.word_before.clone();
                        word_current = ctx.word_current.clone();
                        word_after = ctx.word_after.clone();
                        has_word_sync = ctx.has_word_sync;
                        has_lyrics = !lyrics_data.lines.is_empty();

                        if let Ok(mut shared) = worker_lyric_ctx.lock() {
                            *shared = ctx;
                        }
                    }
                }

                let lyric_clone = current_lyric_text.clone();
                let prev_clone = prev_lyric_text.clone();
                let curr_clone = current_lyric_text.clone();
                let next_clone = next_lyric_text.clone();
                let word_before_clone = word_before.clone();
                let word_current_clone = word_current.clone();
                let word_after_clone = word_after.clone();
                let show_lyrics = lyrics_enabled && has_lyrics;
                let lyric_line_changed = !last_lyric_line.is_empty()
                    && !current_lyric_text.is_empty()
                    && current_lyric_text != last_lyric_line;
                let old_lyric_line = last_lyric_line.clone();
                last_lyric_line = current_lyric_text.clone();
                platform::set_show_lyrics(show_lyrics);

                let display_text = if !current_lyric_text.is_empty() {
                    &current_lyric_text
                } else {
                    &snapshot.title
                };
                let char_count = display_text.chars().count();
                let computed_lyric_w = (char_count as f32 * 6.8 + 68.0)
                    .clamp(COMPACT_LYRIC_MIN_WIDTH, COMPACT_LYRIC_MAX_WIDTH);
                platform::set_current_lyric_width(computed_lyric_w.round() as i32);

                let _ = worker_window.upgrade_in_event_loop(move |w| {
                    SYNC_EQ_CB.with(|slot| {
                        if let Some(cb) = slot.borrow().as_ref() {
                            cb(snapshot.is_playing);
                        }
                    });

                    if lyric_line_changed {
                        let next_gen = TRANSITION_GEN.with(|slot| {
                            slot.borrow()
                                .as_ref()
                                .map(|g| {
                                    let v = g.get().wrapping_add(1);
                                    g.set(v);
                                    v
                                })
                                .unwrap_or(0)
                        });
                        w.set_lyric_transition_old_line(old_lyric_line.into());
                        w.set_lyric_transition_active(true);
                        w.set_lyric_transition_started(false);
                        let weak = w.as_weak();
                        Timer::single_shot(Duration::from_millis(16), move || {
                            let is_current = TRANSITION_GEN.with(|slot| {
                                slot.borrow()
                                    .as_ref()
                                    .map(|g| g.get() == next_gen)
                                    .unwrap_or(false)
                            });
                            if !is_current {
                                return;
                            }
                            if let Some(w) = weak.upgrade() {
                                w.set_lyric_transition_started(true);
                                let weak = w.as_weak();
                                Timer::single_shot(Duration::from_millis(280), move || {
                                    let is_current = TRANSITION_GEN.with(|slot| {
                                        slot.borrow()
                                            .as_ref()
                                            .map(|g| g.get() == next_gen)
                                            .unwrap_or(false)
                                    });
                                    if !is_current {
                                        return;
                                    }
                                    if let Some(w) = weak.upgrade() {
                                        w.set_lyric_transition_active(false);
                                        w.set_lyric_transition_started(false);
                                        w.set_lyric_transition_old_line(String::new().into());
                                    }
                                });
                            }
                        });
                    }
                    apply_snapshot(&w, snapshot);
                    w.set_show_lyrics(show_lyrics);
                    w.set_current_lyric_line(lyric_clone.into());
                    w.set_lyric_width(computed_lyric_w);
                    w.set_lyric_prev(prev_clone.into());
                    w.set_lyric_curr(curr_clone.into());
                    w.set_lyric_next(next_clone.into());
                    w.set_lyric_word_before(word_before_clone.into());
                    w.set_lyric_word_current(word_current_clone.into());
                    w.set_lyric_word_after(word_after_clone.into());
                    w.set_has_word_sync(has_word_sync);
                });
            }

            let sleep_ms = if is_playing { 250 } else { 1000 };
            thread::sleep(Duration::from_millis(sleep_ms));
        }
    });

    // Zero-CPU blocking tray action dispatcher thread.
    let (tray_tx, tray_rx) = mpsc::channel::<TrayAction>();
    let tray_window = window.as_weak();
    let tray_cmd_tx = cmd_tx.clone();
    let tray_cfg = Arc::clone(&shared_cfg);
    let tray_shutdown = Arc::clone(&shutdown);
    thread::spawn(move || {
        let mut is_hidden = false;
        while let Ok(action) = tray_rx.recv() {
            match action {
                TrayAction::ToggleVisibility => {
                    is_hidden = !is_hidden;
                    platform::set_hidden_state(is_hidden);
                    platform::set_overlay_visible(!is_hidden);
                }
                TrayAction::ToggleLyrics => {
                    let tx = tray_cmd_tx.clone();
                    let cfg_ref = Arc::clone(&tray_cfg);
                    let _ = tray_window.upgrade_in_event_loop(move |w| {
                        let enabled = !w.get_lyrics_enabled();
                        w.set_lyrics_enabled(enabled);
                        platform::set_lyrics_enabled(enabled);
                        let _ = tx.send(MediaCommand::SetLyricsEnabled(enabled));
                        persist_config(&cfg_ref, |c| c.lyrics_enabled = enabled);
                    });
                }
                TrayAction::ToggleStartup => {
                    let next_enabled = tray_cfg
                        .lock()
                        .map(|c| !c.launch_at_startup)
                        .unwrap_or(false);
                    match platform::set_launch_at_startup(next_enabled) {
                        Ok(()) => {
                            platform::set_startup_state(next_enabled);
                            persist_config(&tray_cfg, |c| c.launch_at_startup = next_enabled);
                            logging::info(
                                "app::startup",
                                format!("Launch at startup set to {next_enabled}"),
                            );
                        }
                        Err(e) => {
                            logging::error(
                                "app::startup",
                                format!("Failed to update startup registry: {e}"),
                            );
                        }
                    }
                }
                TrayAction::OpenLogs => {
                    platform::open_log_folder(&logging::log_dir());
                }
                TrayAction::Quit => {
                    logging::info("app", "Quit requested from system tray");
                    tray_shutdown.store(true, Ordering::Relaxed);
                    let _ = slint::quit_event_loop();
                    break;
                }
            }
        }
    });

    wire_ui_callbacks(
        &window,
        cmd_tx,
        lyric_context_shared,
        Arc::clone(&shared_cfg),
    );

    // Initial system status update & deduplicated 5-second refresh (minute clock + battery).
    let sys_cache = Rc::new(RefCell::new(SystemStatusCache::default()));
    update_system_status(&window, &mut sys_cache.borrow_mut());
    let sys_window = window.as_weak();
    let sys_cache_timer = Rc::clone(&sys_cache);
    let sys_timer = Timer::default();
    sys_timer.start(TimerMode::Repeated, Duration::from_secs(5), move || {
        if let Some(w) = sys_window.upgrade() {
            update_system_status(&w, &mut sys_cache_timer.borrow_mut());
        }
    });

    window.show()?;
    platform::position_overlay(window.window());
    platform::setup_window_and_tray(tray_tx);

    let result = slint::run_event_loop();
    shutdown.store(true, Ordering::Relaxed);
    platform::remove_tray_icon();
    logging::info("app", "Fidra Island exited cleanly");
    result
}

fn wire_ui_callbacks(
    window: &MainWindow,
    cmd_tx: mpsc::Sender<MediaCommand>,
    lyric_context_shared: Arc<Mutex<LyricContext>>,
    shared_cfg: Arc<Mutex<AppConfig>>,
) {
    let tx_toggle = cmd_tx.clone();
    window.on_toggle_playback(move || {
        let _ = tx_toggle.send(MediaCommand::TogglePlayback);
    });

    let tx_next = cmd_tx.clone();
    window.on_skip_next(move || {
        let _ = tx_next.send(MediaCommand::Next);
    });

    let tx_prev = cmd_tx.clone();
    window.on_skip_previous(move || {
        let _ = tx_prev.send(MediaCommand::Previous);
    });

    let tx_seek = cmd_tx.clone();
    window.on_seek(move |fraction| {
        let _ = tx_seek.send(MediaCommand::SeekFraction(fraction));
    });

    let tx_lyric_seek = cmd_tx.clone();
    window.on_seek_to_lyric(move |direction| {
        if let Ok(ctx) = lyric_context_shared.lock() {
            let target_time = match direction {
                -1 => ctx.prev_time,
                0 => ctx.curr_time,
                1 => ctx.next_time,
                _ => None,
            };
            if let Some(t) = target_time {
                let _ = tx_lyric_seek.send(MediaCommand::SeekSeconds(t));
            }
        }
    });

    let lyrics_window = window.as_weak();
    let tx_lyrics = cmd_tx;
    let lyrics_cfg = Arc::clone(&shared_cfg);
    window.on_toggle_lyrics(move || {
        if let Some(w) = lyrics_window.upgrade() {
            let enabled = !w.get_lyrics_enabled();
            w.set_lyrics_enabled(enabled);
            platform::set_lyrics_enabled(enabled);
            let _ = tx_lyrics.send(MediaCommand::SetLyricsEnabled(enabled));
            persist_config(&lyrics_cfg, |c| c.lyrics_enabled = enabled);
        }
    });

    let pin_window = window.as_weak();
    let pin_cfg = shared_cfg;
    window.on_toggle_pinned(move || {
        if let Some(w) = pin_window.upgrade() {
            let is_pinned = !w.get_is_pinned();
            w.set_is_pinned(is_pinned);
            persist_config(&pin_cfg, |c| c.is_pinned = is_pinned);
        }
    });

    let hover_window = window.as_weak();
    let collapse_timer = Rc::new(RefCell::new(None::<Timer>));
    let expand_timer = Rc::new(RefCell::new(None::<Timer>));

    let exp_window = window.as_weak();
    let exp_timer = Rc::clone(&collapse_timer);
    window.on_toggle_expanded(move || {
        if let Some(w) = exp_window.upgrade() {
            let next_state = !w.get_is_expanded();
            w.set_is_expanded(next_state);
            platform::set_expanded(next_state);

            if !next_state {
                if let Some(timer) = exp_timer.borrow_mut().take() {
                    timer.stop();
                }
            }
        }
    });

    let h_timer = Rc::clone(&collapse_timer);
    let h_expand_timer = Rc::clone(&expand_timer);
    let h_window = hover_window;
    window.on_hover_changed(move |hovered| {
        platform::set_hovered(hovered);
        if let Some(w) = h_window.upgrade() {
            if hovered {
                if let Some(timer) = h_timer.borrow_mut().take() {
                    timer.stop();
                }
                if !w.get_is_expanded() {
                    let exp_win = h_window.clone();
                    let timer = Timer::default();
                    timer.start(
                        TimerMode::SingleShot,
                        Duration::from_millis(80),
                        move || {
                            if let Some(w) = exp_win.upgrade() {
                                w.set_is_expanded(true);
                                platform::set_expanded(true);
                            }
                        },
                    );
                    *h_expand_timer.borrow_mut() = Some(timer);
                }
            } else {
                if let Some(timer) = h_expand_timer.borrow_mut().take() {
                    timer.stop();
                }
                if w.get_is_expanded() && !w.get_is_pinned() {
                    let timer_window = h_window.clone();
                    let timer = Timer::default();
                    timer.start(
                        TimerMode::SingleShot,
                        Duration::from_millis(250),
                        move || {
                            if let Some(w) = timer_window.upgrade() {
                                if !w.get_is_pinned() {
                                    w.set_is_expanded(false);
                                    platform::set_expanded(false);
                                }
                            }
                        },
                    );
                    *h_timer.borrow_mut() = Some(timer);
                }
            }
        }
    });
}

fn persist_config(shared_cfg: &Arc<Mutex<AppConfig>>, update: impl FnOnce(&mut AppConfig)) {
    if let Ok(mut guard) = shared_cfg.lock() {
        update(&mut guard);
        if let Err(e) = config::save(&guard) {
            logging::warn("config", format!("Failed to save config: {e}"));
        }
    }
}

fn apply_snapshot(window: &MainWindow, snapshot: MediaSnapshot) {
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

fn update_system_status(window: &MainWindow, cache: &mut SystemStatusCache) {
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
