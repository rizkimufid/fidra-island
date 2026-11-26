//! Main application wiring: Slint event loop, worker threads, persistent config,
//! zero-CPU idle timers, and system tray action handling.

mod callbacks;
mod state;

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
    app::{
        callbacks::{persist_config, wire_ui_callbacks},
        state::{apply_snapshot, update_system_status, SystemStatusCache},
    },
    config, logging,
    lyrics::{self, LyricContext},
    media::{MediaCommand, MediaController},
    platform::{self, TrayAction, COMPACT_LYRIC_MAX_WIDTH, COMPACT_LYRIC_MIN_WIDTH},
    LyricRow, MainWindow,
};

/// Maximum cached tracks held in the in-memory lyric lookup cache.
const LYRIC_CACHE_CAPACITY: usize = 64;

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
    window.set_romanize_enabled(initial_cfg.romanize_enabled);
    window.set_prayer_enabled(initial_cfg.prayer_enabled);
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

    // Live spectrum visualizer timer driving 4 animated bars (strictly stopped when paused).
    let eq_timer = Rc::new(Timer::default());
    let eq_running = Rc::new(Cell::new(false));
    let eq_tick = Rc::new(Cell::new(0.0f32));

    let worker_eq_timer_sync = window.as_weak();
    let eq_timer_for_ui = Rc::clone(&eq_timer);
    let eq_running_for_ui = Rc::clone(&eq_running);
    let eq_tick_for_ui = Rc::clone(&eq_tick);

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
    }
    SYNC_EQ_CB.with(|slot| *slot.borrow_mut() = Some(sync_eq_state));

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
        let mut lyrics_dirty = false;

        while !worker_shutdown.load(Ordering::Relaxed) {
            while let Ok((song_key, result)) = lyric_result_rx.try_recv() {
                if song_key == last_song_key {
                    cached_lyrics = result;
                    lyrics_dirty = true;
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
                let mut lyric_index: i32 = 0;
                let mut word_before = String::new();
                let mut word_current = String::new();
                let mut word_after = String::new();
                let mut has_word_sync = false;
                let mut has_lyrics = false;

                if lyrics_enabled && snapshot.title != "No Media Playing" {
                    if current_song_key != last_song_key {
                        last_song_key = current_song_key.clone();
                        cached_lyrics = None;
                        lyrics_dirty = true;
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
                        current_lyric_text = ctx.curr_text.clone();
                        lyric_index = ctx.curr_index as i32;
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
                let word_before_clone = word_before.clone();
                let word_current_clone = word_current.clone();
                let word_after_clone = word_after.clone();
                let show_lyrics = lyrics_enabled && has_lyrics;
                let lines_update = if lyrics_dirty {
                    lyrics_dirty = false;
                    Some(
                        cached_lyrics
                            .as_ref()
                            .map(|d| d.line_texts())
                            .unwrap_or_default(),
                    )
                } else {
                    None
                };
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

                    // Full lyric list is pushed only when the track/lyrics change; the
                    // ribbon then scrolls smoothly by animating `lyric-index`.
                    if let Some(lines) = lines_update {
                        let items: Vec<LyricRow> = lines
                            .into_iter()
                            .map(|text| {
                                let roman = lyrics::romanize_line(&text);
                                let has_roman = roman.is_some();
                                LyricRow {
                                    jp: lyrics::is_japanese(&text),
                                    text: text.into(),
                                    roman: roman.unwrap_or_default().into(),
                                    has_roman,
                                }
                            })
                            .collect();
                        w.set_lyric_lines(slint::ModelRc::new(slint::VecModel::from(items)));
                    }
                    w.set_lyric_index(lyric_index);
                    apply_snapshot(&w, snapshot);
                    w.set_show_lyrics(show_lyrics);
                    w.set_current_lyric_line(lyric_clone.into());
                    w.set_lyric_width(computed_lyric_w);
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
