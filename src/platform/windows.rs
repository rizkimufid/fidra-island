//! Windows platform implementation: Win32 DPI-aware `WM_NCHITTEST`,
//! `WM_DISPLAYCHANGE`/`WM_DPICHANGED` re-centering, native System Tray icon & menu,
//! Registry auto-start, RAII COM MTA initialization, and system telemetry.

use super::{
    hit_test_flags, tray_menu_flags, SystemStatus, TrayAction, COMPACT_HEIGHT, COMPACT_HOVER_WIDTH,
    COMPACT_LYRIC_MAX_WIDTH, COMPACT_LYRIC_MIN_WIDTH, COMPACT_WIDTH, EAR_FLARE_WIDTH,
    EXPANDED_HEIGHT, EXPANDED_LYRICS_WIDTH, EXPANDED_WIDTH, WINDOW_WIDTH,
};
use crate::logging;
use std::path::Path;
use std::sync::atomic::{AtomicIsize, Ordering};
use std::sync::{mpsc, Mutex, OnceLock};

use windows_sys::Win32::{
    Foundation::{HWND, LPARAM, LRESULT, POINT, RECT, SYSTEMTIME, S_FALSE, S_OK, WPARAM},
    Graphics::Gdi::ScreenToClient,
    System::{
        Com::{CoInitializeEx, CoUninitialize, COINIT_MULTITHREADED},
        Power::{GetSystemPowerStatus, SYSTEM_POWER_STATUS},
        Registry::{
            RegCloseKey, RegDeleteValueW, RegOpenKeyExW, RegSetValueExW, HKEY, HKEY_CURRENT_USER,
            KEY_SET_VALUE, REG_SZ,
        },
        SystemInformation::GetLocalTime,
    },
    UI::{
        Shell::{
            ShellExecuteW, Shell_NotifyIconW, NIF_ICON, NIF_MESSAGE, NIF_TIP, NIM_ADD, NIM_DELETE,
            NOTIFYICONDATAW,
        },
        WindowsAndMessaging::{
            AppendMenuW, CallWindowProcW, CreatePopupMenu, DefWindowProcW, DestroyMenu,
            EnumWindows, GetClientRect, GetCursorPos, GetSystemMetrics, GetWindowLongPtrW,
            GetWindowThreadProcessId, IsWindowVisible, LoadIconW, PostMessageW,
            SetForegroundWindow, SetWindowLongPtrW, SetWindowPos, ShowWindow, TrackPopupMenu,
            GWLP_WNDPROC, GWL_EXSTYLE, HTCLIENT, HTTRANSPARENT, HWND_TOPMOST, IDI_APPLICATION,
            MF_CHECKED, MF_SEPARATOR, MF_STRING, MF_UNCHECKED, SM_CXSCREEN, SWP_FRAMECHANGED,
            SWP_NOACTIVATE, SWP_NOMOVE, SWP_NOSIZE, SW_HIDE, SW_SHOWNOACTIVATE, SW_SHOWNORMAL,
            TPM_BOTTOMALIGN, TPM_RETURNCMD, TPM_RIGHTBUTTON, WM_APP, WM_CONTEXTMENU,
            WM_DISPLAYCHANGE, WM_DPICHANGED, WM_LBUTTONDBLCLK, WM_NCHITTEST, WM_NULL, WM_RBUTTONUP,
            WNDPROC, WS_EX_APPWINDOW, WS_EX_TOOLWINDOW,
        },
    },
};

const WM_TRAYICON: u32 = WM_APP + 1;
const TRAY_ICON_UID: u32 = 1;

const IDM_TOGGLE_VISIBILITY: usize = 1001;
const IDM_TOGGLE_LYRICS: usize = 1002;
const IDM_TOGGLE_STARTUP: usize = 1003;
const IDM_OPEN_LOGS: usize = 1004;
const IDM_QUIT: usize = 1005;

static PREV_WNDPROC: AtomicIsize = AtomicIsize::new(0);
static MAIN_HWND: AtomicIsize = AtomicIsize::new(0);
static TRAY_SENDER: OnceLock<Mutex<mpsc::Sender<TrayAction>>> = OnceLock::new();

/// RAII guard that initializes Windows COM in Multi-Threaded Apartment (MTA) mode
/// for the current worker thread and uninitializes it cleanly on drop.
pub struct ComGuard {
    should_uninit: bool,
}

impl ComGuard {
    /// Initializes COM MTA on the calling thread.
    pub fn init_mta() -> Self {
        let hr = unsafe { CoInitializeEx(std::ptr::null(), COINIT_MULTITHREADED as u32) };
        let should_uninit = hr == S_OK || hr == S_FALSE;
        if !should_uninit {
            logging::warn(
                "platform::com",
                format!("CoInitializeEx(MTA) returned HRESULT 0x{:08X}", hr as u32),
            );
        }
        Self { should_uninit }
    }
}

impl Drop for ComGuard {
    fn drop(&mut self) {
        if self.should_uninit {
            unsafe { CoUninitialize() };
        }
    }
}

/// Queries current local wall-clock time and system battery status via Win32.
pub fn query_system_status() -> SystemStatus {
    let mut st: SYSTEMTIME = unsafe { std::mem::zeroed() };
    unsafe { GetLocalTime(&mut st) };
    let time_str = format!("{:02}:{:02}", st.wHour, st.wMinute);

    let mut power: SYSTEM_POWER_STATUS = unsafe { std::mem::zeroed() };
    let ok = unsafe { GetSystemPowerStatus(&mut power) };
    let (battery_pct, is_charging) = if ok != 0 && power.BatteryLifePercent <= 100 {
        (power.BatteryLifePercent, power.ACLineStatus == 1)
    } else {
        (100, true)
    };

    SystemStatus {
        time_str,
        battery_pct,
        is_charging,
    }
}

/// Centers the overlay horizontally at the top edge (`y = 0`) using the window's DPI scale factor.
pub fn position_overlay(window: &slint::Window) {
    let scale = window.scale_factor().max(1.0);
    let physical_width = (WINDOW_WIDTH as f32 * scale).round() as i32;
    let screen_width = unsafe { GetSystemMetrics(SM_CXSCREEN) };
    let x = (screen_width - physical_width).max(0) / 2;
    window.set_position(slint::PhysicalPosition::new(x, 0));
}

/// Shows or hides the overlay window without stealing focus.
pub fn set_overlay_visible(visible: bool) {
    let raw = MAIN_HWND.load(Ordering::Relaxed);
    if raw != 0 {
        let hwnd = raw as HWND;
        unsafe {
            ShowWindow(hwnd, if visible { SW_SHOWNOACTIVATE } else { SW_HIDE });
        }
    }
}

/// Registers or removes Fidra Island from Windows user sign-in auto-start (`HKCU\...\Run`).
pub fn set_launch_at_startup(enabled: bool) -> Result<(), String> {
    let subkey = to_wide("Software\\Microsoft\\Windows\\CurrentVersion\\Run");
    let value_name = to_wide("FidraIsland");
    let mut hkey: HKEY = std::ptr::null_mut();

    let status = unsafe {
        RegOpenKeyExW(
            HKEY_CURRENT_USER,
            subkey.as_ptr(),
            0,
            KEY_SET_VALUE,
            &mut hkey,
        )
    };
    if status != 0 {
        return Err(format!("RegOpenKeyExW failed with code {status}"));
    }

    let result = if enabled {
        match std::env::current_exe() {
            Ok(exe_path) => {
                let cmd = format!("\"{}\"", exe_path.display());
                let wide = to_wide(&cmd);
                let byte_len = (wide.len() * std::mem::size_of::<u16>()) as u32;
                let res = unsafe {
                    RegSetValueExW(
                        hkey,
                        value_name.as_ptr(),
                        0,
                        REG_SZ,
                        wide.as_ptr() as *const u8,
                        byte_len,
                    )
                };
                if res == 0 {
                    Ok(())
                } else {
                    Err(format!("RegSetValueExW failed with code {res}"))
                }
            }
            Err(e) => Err(format!("Failed to resolve current_exe: {e}")),
        }
    } else {
        let res = unsafe { RegDeleteValueW(hkey, value_name.as_ptr()) };
        // ERROR_FILE_NOT_FOUND (2) is fine when removing an already-absent entry.
        if res == 0 || res == 2 {
            Ok(())
        } else {
            Err(format!("RegDeleteValueW failed with code {res}"))
        }
    };

    unsafe { RegCloseKey(hkey) };
    result
}

/// Opens the log directory in Windows Explorer.
pub fn open_logs_dir(path: &Path) {
    let _ = std::fs::create_dir_all(path);
    let verb = to_wide("open");
    let target = to_wide(&path.to_string_lossy());
    unsafe {
        ShellExecuteW(
            std::ptr::null_mut(),
            verb.as_ptr(),
            target.as_ptr(),
            std::ptr::null(),
            std::ptr::null(),
            SW_SHOWNORMAL,
        );
    }
}

/// Subclasses the overlay HWND for DPI-aware `WM_NCHITTEST`, display change
/// re-centering, taskbar exclusion (`WS_EX_TOOLWINDOW`), and registers the System Tray icon.
pub fn setup_window_and_tray(tray_tx: mpsc::Sender<TrayAction>) {
    let _ = TRAY_SENDER.set(Mutex::new(tray_tx));

    let mut lookup = WindowLookup {
        process_id: std::process::id(),
        hwnd: std::ptr::null_mut(),
    };
    unsafe {
        EnumWindows(
            Some(find_own_window),
            &mut lookup as *mut WindowLookup as isize,
        );
    }
    let hwnd = lookup.hwnd;
    if hwnd.is_null() {
        logging::warn("platform::windows", "Could not locate overlay HWND");
        return;
    }

    MAIN_HWND.store(hwnd as isize, Ordering::Relaxed);

    unsafe {
        // Hide briefly so Explorer re-evaluates the taskbar entry after applying tool-window style.
        ShowWindow(hwnd, SW_HIDE);
        let ex_style = GetWindowLongPtrW(hwnd, GWL_EXSTYLE);
        let ex_style = (ex_style | WS_EX_TOOLWINDOW as isize) & !(WS_EX_APPWINDOW as isize);
        SetWindowLongPtrW(hwnd, GWL_EXSTYLE, ex_style);
        SetWindowPos(
            hwnd,
            HWND_TOPMOST,
            0,
            0,
            0,
            0,
            SWP_NOMOVE | SWP_NOSIZE | SWP_NOACTIVATE | SWP_FRAMECHANGED,
        );

        let prev = GetWindowLongPtrW(hwnd, GWLP_WNDPROC);
        PREV_WNDPROC.store(prev, Ordering::Relaxed);
        SetWindowLongPtrW(hwnd, GWLP_WNDPROC, window_proc as *const () as isize);
        ShowWindow(hwnd, SW_SHOWNOACTIVATE);

        add_tray_icon(hwnd);
    }
}

/// Removes the notification area icon on application shutdown so no ghost icon lingers.
pub fn remove_tray_icon() {
    let raw = MAIN_HWND.load(Ordering::Relaxed);
    if raw == 0 {
        return;
    }
    unsafe {
        let mut nid: NOTIFYICONDATAW = std::mem::zeroed();
        nid.cbSize = std::mem::size_of::<NOTIFYICONDATAW>() as u32;
        nid.hWnd = raw as HWND;
        nid.uID = TRAY_ICON_UID;
        Shell_NotifyIconW(NIM_DELETE, &nid);
    }
}

unsafe fn add_tray_icon(hwnd: HWND) {
    let mut nid: NOTIFYICONDATAW = std::mem::zeroed();
    nid.cbSize = std::mem::size_of::<NOTIFYICONDATAW>() as u32;
    nid.hWnd = hwnd;
    nid.uID = TRAY_ICON_UID;
    nid.uFlags = NIF_MESSAGE | NIF_ICON | NIF_TIP;
    nid.uCallbackMessage = WM_TRAYICON;
    nid.hIcon = LoadIconW(std::ptr::null_mut(), IDI_APPLICATION);

    let tip = to_wide("Fidra Island — Dynamic Island Media Controller");
    let copy_len = tip.len().min(nid.szTip.len() - 1);
    nid.szTip[..copy_len].copy_from_slice(&tip[..copy_len]);

    if Shell_NotifyIconW(NIM_ADD, &nid) == 0 {
        logging::warn("platform::tray", "Shell_NotifyIconW(NIM_ADD) failed");
    }
}

unsafe fn reposition_hwnd(hwnd: HWND) {
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

unsafe fn show_tray_menu(hwnd: HWND) {
    let menu = CreatePopupMenu();
    if menu.is_null() {
        return;
    }

    let (is_hidden, lyrics_enabled, launch_at_startup) = tray_menu_flags();

    let vis_label = to_wide(if is_hidden {
        "Show Fidra Island"
    } else {
        "Hide Fidra Island"
    });
    let lyrics_label = to_wide("Synced Lyrics");
    let startup_label = to_wide("Launch at Startup");
    let logs_label = to_wide("Open Logs Folder");
    let quit_label = to_wide("Quit Fidra Island");

    AppendMenuW(menu, MF_STRING, IDM_TOGGLE_VISIBILITY, vis_label.as_ptr());
    AppendMenuW(
        menu,
        MF_STRING
            | if lyrics_enabled {
                MF_CHECKED
            } else {
                MF_UNCHECKED
            },
        IDM_TOGGLE_LYRICS,
        lyrics_label.as_ptr(),
    );
    AppendMenuW(
        menu,
        MF_STRING
            | if launch_at_startup {
                MF_CHECKED
            } else {
                MF_UNCHECKED
            },
        IDM_TOGGLE_STARTUP,
        startup_label.as_ptr(),
    );
    AppendMenuW(menu, MF_SEPARATOR, 0, std::ptr::null());
    AppendMenuW(menu, MF_STRING, IDM_OPEN_LOGS, logs_label.as_ptr());
    AppendMenuW(menu, MF_SEPARATOR, 0, std::ptr::null());
    AppendMenuW(menu, MF_STRING, IDM_QUIT, quit_label.as_ptr());

    let mut pt = POINT { x: 0, y: 0 };
    GetCursorPos(&mut pt);

    // Required by Win32 so the popup menu dismisses when clicking outside it.
    SetForegroundWindow(hwnd);
    let cmd = TrackPopupMenu(
        menu,
        TPM_RETURNCMD | TPM_RIGHTBUTTON | TPM_BOTTOMALIGN,
        pt.x,
        pt.y,
        0,
        hwnd,
        std::ptr::null(),
    ) as usize;
    PostMessageW(hwnd, WM_NULL, 0, 0);
    DestroyMenu(menu);

    let action = match cmd {
        IDM_TOGGLE_VISIBILITY => Some(TrayAction::ToggleVisibility),
        IDM_TOGGLE_LYRICS => Some(TrayAction::ToggleLyrics),
        IDM_TOGGLE_STARTUP => Some(TrayAction::ToggleStartup),
        IDM_OPEN_LOGS => Some(TrayAction::OpenLogs),
        IDM_QUIT => Some(TrayAction::Quit),
        _ => None,
    };

    if let Some(act) = action {
        dispatch_tray_action(act);
    }
}

fn dispatch_tray_action(action: TrayAction) {
    if let Some(lock) = TRAY_SENDER.get() {
        if let Ok(tx) = lock.lock() {
            let _ = tx.send(action);
        }
    }
}

unsafe extern "system" fn window_proc(
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

            let (is_exp, is_hov, show_lyrics, lyrics_enabled, lyric_w) = hit_test_flags();
            let island_w = if is_exp && lyrics_enabled {
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
                EXPANDED_HEIGHT
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

struct WindowLookup {
    process_id: u32,
    hwnd: HWND,
}

unsafe extern "system" fn find_own_window(
    hwnd: HWND,
    parameter: LPARAM,
) -> windows_sys::Win32::Foundation::BOOL {
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

fn to_wide(s: &str) -> Vec<u16> {
    s.encode_utf16().chain(std::iter::once(0)).collect()
}
