//! Win32 window procedure, DPI-aware hit testing, and display change events.

use super::super::{
    hit_test_flags, TrayAction, COMPACT_HEIGHT, COMPACT_HOVER_WIDTH, COMPACT_LYRIC_MAX_WIDTH,
    COMPACT_LYRIC_MIN_WIDTH, COMPACT_WIDTH, EAR_FLARE_WIDTH, EXPANDED_HEIGHT,
    EXPANDED_LYRICS_WIDTH, EXPANDED_WIDTH, WINDOW_WIDTH,
};
use super::tray::{dispatch_tray_action, show_tray_menu, WM_TRAYICON};
use crate::logging;
use std::sync::atomic::{AtomicIsize, Ordering};
use windows_sys::Win32::{
    Foundation::{BOOL, HWND, LPARAM, LRESULT, POINT, RECT, WPARAM},
    Graphics::Gdi::ScreenToClient,
    UI::WindowsAndMessaging::{
        CallWindowProcW, DefWindowProcW, GetClientRect, GetSystemMetrics, GetWindowThreadProcessId,
        IsWindowVisible, SetWindowPos, HTCLIENT, HTTRANSPARENT, HWND_TOPMOST, SM_CXSCREEN,
        SWP_NOACTIVATE, SWP_NOSIZE, WM_CONTEXTMENU, WM_DISPLAYCHANGE, WM_DPICHANGED,
        WM_LBUTTONDBLCLK, WM_NCHITTEST, WM_RBUTTONUP, WNDPROC,
    },
};

pub static PREV_WNDPROC: AtomicIsize = AtomicIsize::new(0);

pub struct WindowLookup {
    pub process_id: u32,
    pub hwnd: HWND,
}

pub unsafe extern "system" fn find_own_window(hwnd: HWND, parameter: LPARAM) -> BOOL {
    let lookup = &mut *(parameter as *mut WindowLookup);
    let mut process_id = 0;
    GetWindowThreadProcessId(hwnd, &mut process_id);
    if process_id == lookup.process_id && IsWindowVisible(hwnd) != 0 {
        lookup.hwnd = hwnd;
        0
    } else {
        1
    }
}

pub unsafe fn reposition_hwnd(hwnd: HWND) {
    let mut rect = RECT {
        left: 0,
        top: 0,
        right: 0,
        bottom: 0,
    };
    let phys_w = if GetClientRect(hwnd, &mut rect) != 0 && rect.right > rect.left {
        rect.right - rect.left
    } else {
        WINDOW_WIDTH
    };
    let screen_width = GetSystemMetrics(SM_CXSCREEN);
    let x = (screen_width - phys_w).max(0) / 2;
    SetWindowPos(hwnd, HWND_TOPMOST, x, 0, 0, 0, SWP_NOSIZE | SWP_NOACTIVATE);
}

pub unsafe extern "system" fn window_proc(
    hwnd: HWND,
    msg: u32,
    wparam: WPARAM,
    lparam: LPARAM,
) -> LRESULT {
    if msg == WM_NCHITTEST {
        let hit_result = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
            let x = (lparam & 0xFFFF) as i16 as i32;
            let y = ((lparam >> 16) & 0xFFFF) as i16 as i32;
            let mut pt = POINT { x, y };
            ScreenToClient(hwnd, &mut pt);

            let mut rect = RECT {
                left: 0,
                top: 0,
                right: 0,
                bottom: 0,
            };
            let scale = if GetClientRect(hwnd, &mut rect) != 0 && rect.right > rect.left {
                ((rect.right - rect.left) as f32 / WINDOW_WIDTH as f32).max(1.0)
            } else {
                1.0
            };
            let logical_x = (pt.x as f32 / scale).round() as i32;
            let logical_y = (pt.y as f32 / scale).round() as i32;

            let (is_exp, is_hov, show_lyrics, lyrics_enabled, lyric_w, exp_page) = hit_test_flags();
            let island_w = if is_exp && (exp_page == 1 || lyrics_enabled) {
                EXPANDED_LYRICS_WIDTH
            } else if is_exp {
                EXPANDED_WIDTH
            } else if show_lyrics {
                lyric_w.clamp(
                    COMPACT_LYRIC_MIN_WIDTH as i32,
                    COMPACT_LYRIC_MAX_WIDTH as i32,
                )
            } else if is_hov {
                COMPACT_HOVER_WIDTH
            } else {
                COMPACT_WIDTH
            };
            let island_h = if is_exp {
                EXPANDED_HEIGHT + 4
            } else {
                COMPACT_HEIGHT
            };
            let island_x = (WINDOW_WIDTH - island_w) / 2;

            if logical_x >= island_x - EAR_FLARE_WIDTH
                && logical_x <= island_x + island_w + EAR_FLARE_WIDTH
                && logical_y >= 0
                && logical_y <= island_h
            {
                HTCLIENT as _
            } else {
                HTTRANSPARENT as _
            }
        }));

        if let Ok(code) = hit_result {
            return code;
        }
    } else if msg == WM_TRAYICON {
        let _ = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
            let event = (lparam & 0xFFFF) as u32;
            if event == WM_RBUTTONUP || event == WM_CONTEXTMENU {
                show_tray_menu(hwnd);
            } else if event == WM_LBUTTONDBLCLK {
                dispatch_tray_action(TrayAction::ToggleVisibility);
            }
        }));
        return 0;
    } else if msg == WM_DISPLAYCHANGE || msg == WM_DPICHANGED {
        let _ = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
            logging::info(
                "platform::display",
                format!(
                    "Display/DPI change event received (msg=0x{msg:04X}); re-centering overlay"
                ),
            );
            reposition_hwnd(hwnd);
        }));
    }

    let prev = PREV_WNDPROC.load(Ordering::Relaxed);
    if prev != 0 {
        let prev_proc = std::mem::transmute::<isize, WNDPROC>(prev);
        CallWindowProcW(prev_proc, hwnd, msg, wparam, lparam)
    } else {
        DefWindowProcW(hwnd, msg, wparam, lparam)
    }
}
