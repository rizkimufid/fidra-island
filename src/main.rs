use std::{
    cell::{Cell, RefCell},
    rc::Rc,
    sync::{
        atomic::{AtomicBool, AtomicI32, AtomicIsize, Ordering},
        mpsc, Arc, Mutex,
    },
    thread,
    time::Duration,
};

use slint::{ComponentHandle, Timer, TimerMode};

mod lyrics;
mod media;

use lyrics::LyricContext;
use media::{MediaCommand, MediaController, MediaSnapshot};

slint::include_modules!();

const WINDOW_WIDTH: i32 = 680;
#[allow(dead_code)]
const WINDOW_HEIGHT: i32 = 220;

static IS_EXPANDED: AtomicBool = AtomicBool::new(false);
static SHOW_LYRICS: AtomicBool = AtomicBool::new(false);
static CURRENT_LYRIC_WIDTH: AtomicI32 = AtomicI32::new(290);
static PREV_WNDPROC: AtomicIsize = AtomicIsize::new(0);

fn main() -> Result<(), slint::PlatformError> {
    let window = MainWindow::new()?;

    // Shared lyric context for instant interactive seek-to-lyric callbacks
    let lyric_context_shared = Arc::new(Mutex::new(LyricContext::default()));

    // Channel for dispatching commands from UI thread to background media worker
    let (cmd_tx, cmd_rx) = mpsc::channel::<MediaCommand>();

    // Spawn background media controller worker thread with "Doze Off" idle architecture
    let worker_window = window.as_weak();
    let worker_lyric_ctx = Arc::clone(&lyric_context_shared);
    thread::spawn(move || {
        let mut controller = MediaController::new();
        let mut is_playing = false;
        let mut lyrics_enabled = true; // Always active in expanded mode per rule 1
        let mut cached_lyrics: Option<lyrics::LyricsData> = None;
        let mut last_song_key = String::new();

        loop {
            // Process any pending commands from UI
            while let Ok(cmd) = cmd_rx.try_recv() {
                match cmd {
                    MediaCommand::TogglePlayback => controller.toggle_playback(),
                    MediaCommand::Next => controller.skip_next(),
                    MediaCommand::Previous => controller.skip_previous(),
                    MediaCommand::SeekFraction(fraction) => controller.seek_fraction(fraction),
                    MediaCommand::SeekSeconds(seconds) => controller.seek_seconds(seconds),
                    MediaCommand::ToggleLyrics => {
                        lyrics_enabled = !lyrics_enabled;
                        let enabled = lyrics_enabled;
                        if !enabled {
                            SHOW_LYRICS.store(false, Ordering::Relaxed);
                        }
                        let _ = worker_window.upgrade_in_event_loop(move |w| {
                            w.set_lyrics_active(enabled);
                            if !enabled {
                                w.set_show_lyrics(false);
                                w.set_current_lyric_line("".into());
                            }
                        });
                    }
                }
            }

            // Poll media snapshot
            if let Ok(snapshot) = controller.poll() {
                is_playing = snapshot.is_playing;
                let current_song_key = format!("{} - {}", snapshot.title, snapshot.artist);

                let mut current_lyric_text = String::new();
                let mut prev_lyric_text = String::new();
                let mut next_lyric_text = String::new();
                let mut line_progress = 0.0f32;
                let mut has_lyrics = false;

                if lyrics_enabled && snapshot.title != "No Media Playing" {
                    if current_song_key != last_song_key {
                        last_song_key = current_song_key.clone();
                        cached_lyrics = lyrics::fetch_lyrics(
                            &snapshot.title,
                            &snapshot.artist,
                            snapshot.duration_seconds as f64,
                        );
                    }

                    if let Some(lyrics_data) = &cached_lyrics {
                        let ctx = lyrics_data
                            .get_lyrics_context_detailed(snapshot.elapsed_seconds as f64);
                        prev_lyric_text = ctx.prev_text.clone();
                        current_lyric_text = ctx.curr_text.clone();
                        next_lyric_text = ctx.next_text.clone();
                        line_progress = ctx.curr_progress;
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
                let show_lyrics = lyrics_enabled && has_lyrics;
                SHOW_LYRICS.store(show_lyrics, Ordering::Relaxed);

                // Dynamically compute island width to guarantee lyrics are never clipped
                let display_text = if !current_lyric_text.is_empty() {
                    &current_lyric_text
                } else {
                    &snapshot.title
                };
                let char_count = display_text.chars().count();
                let computed_lyric_w = (char_count as f32 * 6.8 + 68.0).clamp(290.0, 460.0);
                CURRENT_LYRIC_WIDTH.store(computed_lyric_w as i32, Ordering::Relaxed);

                let _ = worker_window.upgrade_in_event_loop(move |w| {
                    apply_snapshot(&w, snapshot);
                    w.set_show_lyrics(show_lyrics);
                    w.set_current_lyric_line(lyric_clone.into());
                    w.set_lyric_width(computed_lyric_w);
                    w.set_lyric_prev(prev_clone.into());
                    w.set_lyric_curr(curr_clone.into());
                    w.set_lyric_next(next_clone.into());
                    w.set_active_line_progress(line_progress);
                });
            }

            // Event-driven idle: doze off at 1000ms when paused, 250ms when playing
            let sleep_ms = if is_playing { 250 } else { 1000 };
            thread::sleep(Duration::from_millis(sleep_ms));
        }
    });

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
    let lyric_seek_ctx = Arc::clone(&lyric_context_shared);
    window.on_seek_to_lyric(move |direction| {
        if let Ok(ctx) = lyric_seek_ctx.lock() {
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

    let tx_lyrics = cmd_tx.clone();
    window.on_toggle_lyrics(move || {
        let _ = tx_lyrics.send(MediaCommand::ToggleLyrics);
    });

    let pin_window = window.as_weak();
    window.on_toggle_pinned(move || {
        if let Some(w) = pin_window.upgrade() {
            let is_pinned = w.get_is_pinned();
            w.set_is_pinned(!is_pinned);
        }
    });

    let hover_window = window.as_weak();
    let is_hovered = Rc::new(Cell::new(false));
    let collapse_timer = Rc::new(RefCell::new(None::<Timer>));
    let expand_timer = Rc::new(RefCell::new(None::<Timer>));

    let exp_window = window.as_weak();
    let exp_timer = Rc::clone(&collapse_timer);
    window.on_toggle_expanded(move || {
        if let Some(w) = exp_window.upgrade() {
            let next_state = !w.get_is_expanded();
            w.set_is_expanded(next_state);
            IS_EXPANDED.store(next_state, Ordering::Relaxed);

            if !next_state {
                if let Some(timer) = exp_timer.borrow_mut().take() {
                    timer.stop();
                }
            }
        }
    });

    let h_timer = Rc::clone(&collapse_timer);
    let h_expand_timer = Rc::clone(&expand_timer);
    let h_hovered = Rc::clone(&is_hovered);
    let h_window = hover_window.clone();
    window.on_hover_changed(move |hovered| {
        h_hovered.set(hovered);
        if let Some(w) = h_window.upgrade() {
            if hovered {
                // Cancel any pending collapse timer
                if let Some(timer) = h_timer.borrow_mut().take() {
                    timer.stop();
                }
                // Hover expansion with 80ms snappy debounce
                if !w.get_is_expanded() {
                    let exp_win = h_window.clone();
                    let timer = Timer::default();
                    timer.start(
                        TimerMode::SingleShot,
                        Duration::from_millis(80),
                        move || {
                            if let Some(w) = exp_win.upgrade() {
                                w.set_is_expanded(true);
                                IS_EXPANDED.store(true, Ordering::Relaxed);
                            }
                        },
                    );
                    *h_expand_timer.borrow_mut() = Some(timer);
                }
            } else {
                // Cancel pending hover expand timer if mouse exits early
                if let Some(timer) = h_expand_timer.borrow_mut().take() {
                    timer.stop();
                }
                // Grace period collapse when cursor leaves island
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
                                    IS_EXPANDED.store(false, Ordering::Relaxed);
                                }
                            }
                        },
                    );
                    *h_timer.borrow_mut() = Some(timer);
                }
            }
        }
    });

    let volume_window = window.as_weak();
    window.on_change_volume(move |volume| {
        if let Some(w) = volume_window.upgrade() {
            w.set_volume(volume.clamp(0.0, 1.0));
        }
    });

    // Initial system status update & recurring 1-second refresh
    update_system_status(&window);
    let sys_window = window.as_weak();
    let sys_timer = Timer::default();
    sys_timer.start(
        TimerMode::Repeated,
        Duration::from_millis(1000),
        move || {
            if let Some(w) = sys_window.upgrade() {
                update_system_status(&w);
            }
        },
    );

    // Live spectrum visualizer timer driving 4 animated bars (0% CPU when paused)
    let eq_timer = Timer::default();
    let mut eq_tick: f32 = 0.0;
    let eq_weak = window.as_weak();
    eq_timer.start(TimerMode::Repeated, Duration::from_millis(80), move || {
        if let Some(w) = eq_weak.upgrade() {
            if w.get_is_playing() {
                eq_tick += 0.28;
                let b1 = 3.2
                    + 5.0 * ((eq_tick * 1.6).sin().powi(2))
                    + 3.8 * ((eq_tick * 3.2).cos() * 0.5 + 0.5);
                let b2 = 3.0
                    + 6.0 * ((eq_tick * 2.1 + 0.7).sin() * 0.5 + 0.5)
                    + 3.0 * ((eq_tick * 0.9).cos() * 0.5 + 0.5);
                let b3 = 3.5
                    + 5.5 * ((eq_tick * 1.8 + 1.8).sin() * 0.5 + 0.5)
                    + 3.0 * ((eq_tick * 2.7).sin() * 0.5 + 0.5);
                let b4 = 3.0
                    + 4.5 * ((eq_tick * 3.1 + 2.4).sin() * 0.5 + 0.5)
                    + 4.5 * ((eq_tick * 1.4).cos() * 0.5 + 0.5);

                w.set_eq_bar1(b1.clamp(3.0, 12.0));
                w.set_eq_bar2(b2.clamp(3.0, 12.0));
                w.set_eq_bar3(b3.clamp(3.0, 12.0));
                w.set_eq_bar4(b4.clamp(3.0, 12.0));
            } else if w.get_eq_bar1() != 3.0 {
                w.set_eq_bar1(3.0);
                w.set_eq_bar2(3.0);
                w.set_eq_bar3(3.0);
                w.set_eq_bar4(3.0);
            }
        }
    });

    window.show()?;
    position_overlay(&window);

    #[cfg(target_os = "windows")]
    setup_hit_test();

    slint::run_event_loop()
}

fn apply_snapshot(window: &MainWindow, snapshot: MediaSnapshot) {
    let duration = snapshot.duration_seconds.max(0.0);
    let elapsed = snapshot.elapsed_seconds.clamp(0.0, duration.max(0.001));
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
    window.set_remaining_time(format!("-{}", format_time(remaining)).into());
    window.set_duration_seconds(duration);
    window.set_progress(progress);
    window.set_is_playing(snapshot.is_playing);

    if let Some(thumb_bytes) = &snapshot.thumbnail_data {
        if let Ok(img) = slint::Image::load_from_data(thumb_bytes, None) {
            window.set_album_art(img);
            window.set_has_album_art(true);
        } else {
            window.set_has_album_art(false);
        }
    } else {
        window.set_has_album_art(false);
    }
}

fn update_system_status(window: &MainWindow) {
    #[cfg(target_os = "windows")]
    {
        use windows_sys::Win32::{
            Foundation::SYSTEMTIME,
            System::{
                Power::{GetSystemPowerStatus, SYSTEM_POWER_STATUS},
                SystemInformation::GetLocalTime,
            },
        };

        let mut st: SYSTEMTIME = unsafe { std::mem::zeroed() };
        unsafe { GetLocalTime(&mut st) };
        let time_str = format!("{:02}:{:02}", st.wHour, st.wMinute);
        window.set_current_time_str(time_str.into());

        let mut power: SYSTEM_POWER_STATUS = unsafe { std::mem::zeroed() };
        let ok = unsafe { GetSystemPowerStatus(&mut power) };
        let (pct, charging) = if ok != 0 && power.BatteryLifePercent <= 100 {
            (
                power.BatteryLifePercent as f32 / 100.0,
                power.ACLineStatus == 1,
            )
        } else {
            (1.0, true)
        };
        window.set_battery_level(pct);
        window.set_is_charging(charging);
        window.set_battery_arc_path(get_battery_arc_path(pct).into());
    }

    #[cfg(not(target_os = "windows"))]
    {
        window.set_current_time_str("12:00".into());
        window.set_battery_level(1.0);
        window.set_is_charging(false);
        window.set_battery_arc_path("M 8 2 A 6 6 0 1 1 7.99 2 Z".into());
    }
}

fn get_battery_arc_path(pct: f32) -> String {
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
fn position_overlay(window: &MainWindow) {
    use windows_sys::Win32::UI::WindowsAndMessaging::{GetSystemMetrics, SM_CXSCREEN};

    let screen_width = unsafe { GetSystemMetrics(SM_CXSCREEN) };
    let x = (screen_width - WINDOW_WIDTH).max(0) / 2;
    window
        .window()
        .set_position(slint::PhysicalPosition::new(x, 0));
}

#[cfg(not(target_os = "windows"))]
fn position_overlay(window: &MainWindow) {
    window
        .window()
        .set_position(slint::PhysicalPosition::new(0, 0));
}

#[cfg(target_os = "windows")]
unsafe extern "system" fn window_proc(
    hwnd: windows_sys::Win32::Foundation::HWND,
    msg: u32,
    wparam: windows_sys::Win32::Foundation::WPARAM,
    lparam: windows_sys::Win32::Foundation::LPARAM,
) -> windows_sys::Win32::Foundation::LRESULT {
    use windows_sys::Win32::Foundation::POINT;
    use windows_sys::Win32::Graphics::Gdi::ScreenToClient;
    use windows_sys::Win32::UI::WindowsAndMessaging::{
        CallWindowProcW, HTCLIENT, HTTRANSPARENT, WM_NCHITTEST,
    };

    if msg == WM_NCHITTEST {
        let x = (lparam & 0xFFFF) as i16 as i32;
        let y = ((lparam >> 16) & 0xFFFF) as i16 as i32;
        let mut pt = POINT { x, y };
        ScreenToClient(hwnd, &mut pt);

        let is_exp = IS_EXPANDED.load(Ordering::Relaxed);
        let show_lyrics = SHOW_LYRICS.load(Ordering::Relaxed);
        let island_w = if is_exp {
            620
        } else if show_lyrics {
            CURRENT_LYRIC_WIDTH.load(Ordering::Relaxed).max(290)
        } else {
            290
        };
        let island_h = if is_exp { 175 } else { 38 };
        let island_x = (WINDOW_WIDTH - island_w) / 2;

        // Check if cursor falls inside active island bounds (plus 14px ear flare fillets)
        if pt.x >= island_x - 14
            && pt.x <= island_x + island_w + 14
            && pt.y >= 0
            && pt.y <= island_h
        {
            return HTCLIENT as _;
        } else {
            return HTTRANSPARENT as _;
        }
    }

    let prev = PREV_WNDPROC.load(Ordering::Relaxed);
    if prev != 0 {
        CallWindowProcW(Some(std::mem::transmute(prev)), hwnd, msg, wparam, lparam)
    } else {
        windows_sys::Win32::UI::WindowsAndMessaging::DefWindowProcW(hwnd, msg, wparam, lparam)
    }
}

#[cfg(target_os = "windows")]
fn setup_hit_test() {
    use windows_sys::Win32::UI::WindowsAndMessaging::{
        FindWindowW, GetWindowLongPtrW, SetWindowLongPtrW, GWLP_WNDPROC,
    };

    let title: Vec<u16> = "Fira Music Island\0".encode_utf16().collect();
    let hwnd = unsafe { FindWindowW(std::ptr::null(), title.as_ptr()) };
    if !hwnd.is_null() {
        unsafe {
            let prev = GetWindowLongPtrW(hwnd, GWLP_WNDPROC);
            PREV_WNDPROC.store(prev, Ordering::Relaxed);
            SetWindowLongPtrW(hwnd, GWLP_WNDPROC, window_proc as *const () as isize);
        }
    }
}

fn format_time(seconds: f32) -> String {
    let seconds = seconds.max(0.0).round() as u32;
    format!("{:02}:{:02}", seconds / 60, seconds % 60)
}
