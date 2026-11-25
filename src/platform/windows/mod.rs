//! Windows platform implementation.
//!
//! Subdivided into modular units:
//! - [`hit_test`]: Win32 window procedure, DPI scaling, and display change re-centering.
//! - [`tray`]: System Tray notification icon and Win32 context popup menu.

pub mod hit_test;
pub mod tray;

use super::{SystemStatus, TrayAction, WINDOW_WIDTH};
use crate::logging;
use std::path::Path;
use std::sync::atomic::{AtomicIsize, Ordering};
use std::sync::mpsc;

use windows_sys::Win32::{
    Foundation::{HWND, SYSTEMTIME, S_FALSE, S_OK},
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
        Shell::ShellExecuteW,
        WindowsAndMessaging::{
            EnumWindows, GetSystemMetrics, GetWindowLongPtrW, SetWindowLongPtrW, SetWindowPos,
            ShowWindow, GWLP_WNDPROC, GWL_EXSTYLE, HWND_TOPMOST, SM_CXSCREEN, SWP_FRAMECHANGED,
            SWP_NOACTIVATE, SWP_NOMOVE, SWP_NOSIZE, SW_HIDE, SW_SHOWNOACTIVATE, SW_SHOWNORMAL,
            WS_EX_APPWINDOW, WS_EX_TOOLWINDOW,
        },
    },
};

static MAIN_HWND: AtomicIsize = AtomicIsize::new(0);

/// RAII guard that initializes Windows COM MTA and cleanly uninitializes it on drop.
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
    let subkey = tray::to_wide("Software\\Microsoft\\Windows\\CurrentVersion\\Run");
    let value_name = tray::to_wide("FidraIsland");
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
                let wide = tray::to_wide(&cmd);
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
    let verb = tray::to_wide("open");
    let target = tray::to_wide(&path.to_string_lossy());
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
    tray::set_tray_sender(tray_tx);

    let mut lookup = hit_test::WindowLookup {
        process_id: std::process::id(),
        hwnd: std::ptr::null_mut(),
    };
    unsafe {
        EnumWindows(
            Some(hit_test::find_own_window),
            &mut lookup as *mut hit_test::WindowLookup as isize,
        );
    }
    let hwnd = lookup.hwnd;
    if hwnd.is_null() {
        logging::warn("platform::windows", "Could not locate overlay HWND");
        return;
    }

    MAIN_HWND.store(hwnd as isize, Ordering::Relaxed);

    unsafe {
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
        hit_test::PREV_WNDPROC.store(prev, Ordering::Relaxed);
        SetWindowLongPtrW(
            hwnd,
            GWLP_WNDPROC,
            hit_test::window_proc as *const () as isize,
        );
        ShowWindow(hwnd, SW_SHOWNOACTIVATE);

        tray::add_tray_icon(hwnd);
    }
}

/// Removes the notification area icon on application shutdown so no ghost icon lingers.
pub fn remove_tray_icon() {
    let raw = MAIN_HWND.load(Ordering::Relaxed);
    if raw == 0 {
        return;
    }
    unsafe {
        tray::remove_tray_icon(raw as HWND);
    }
}
