//! UI callbacks and interaction wiring for Fidra Island.

use std::{
    cell::RefCell,
    rc::Rc,
    sync::{mpsc, Arc, Mutex},
    time::Duration,
};

use slint::{ComponentHandle, Timer, TimerMode};

use crate::{
    config::{self, AppConfig},
    logging,
    lyrics::LyricContext,
    media::MediaCommand,
    platform, MainWindow,
};

/// Persists an updated configuration via atomic file write.
pub(crate) fn persist_config(
    shared_cfg: &Arc<Mutex<AppConfig>>,
    update: impl FnOnce(&mut AppConfig),
) {
    if let Ok(mut guard) = shared_cfg.lock() {
        update(&mut guard);
        if let Err(e) = config::save(&guard) {
            logging::warn("config", format!("Failed to save config: {e}"));
        }
    }
}

/// Connects Slint UI callbacks to media commands, configuration persistence, and platform states.
pub(crate) fn wire_ui_callbacks(
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
