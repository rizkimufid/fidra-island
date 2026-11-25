//! Native Windows System Tray icon and context popup menu.

use super::super::{tray_menu_flags, TrayAction};
use crate::logging;
use std::sync::{mpsc, Mutex, OnceLock};
use windows_sys::Win32::{
    Foundation::{HWND, POINT},
    UI::{
        Shell::{Shell_NotifyIconW, NIF_ICON, NIF_MESSAGE, NIF_TIP, NIM_ADD, NIM_DELETE, NOTIFYICONDATAW},
        WindowsAndMessaging::{
            AppendMenuW, CreatePopupMenu, DestroyMenu, GetCursorPos, LoadIconW, PostMessageW,
            SetForegroundWindow, TrackPopupMenu, IDI_APPLICATION, MF_CHECKED, MF_SEPARATOR,
            MF_STRING, MF_UNCHECKED, TPM_BOTTOMALIGN, TPM_RETURNCMD, TPM_RIGHTBUTTON, WM_APP,
            WM_NULL,
        },
    },
};

pub const WM_TRAYICON: u32 = WM_APP + 1;
pub const TRAY_ICON_UID: u32 = 1;

const IDM_TOGGLE_VISIBILITY: usize = 1001;
const IDM_TOGGLE_LYRICS: usize = 1002;
const IDM_TOGGLE_STARTUP: usize = 1003;
const IDM_OPEN_LOGS: usize = 1004;
const IDM_QUIT: usize = 1005;

static TRAY_SENDER: OnceLock<Mutex<mpsc::Sender<TrayAction>>> = OnceLock::new();

pub fn set_tray_sender(tx: mpsc::Sender<TrayAction>) {
    let _ = TRAY_SENDER.set(Mutex::new(tx));
}

pub fn dispatch_tray_action(action: TrayAction) {
    if let Some(lock) = TRAY_SENDER.get() {
        if let Ok(tx) = lock.lock() {
            let _ = tx.send(action);
        }
    }
}

pub unsafe fn add_tray_icon(hwnd: HWND) {
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

pub unsafe fn remove_tray_icon(hwnd: HWND) {
    let mut nid: NOTIFYICONDATAW = std::mem::zeroed();
    nid.cbSize = std::mem::size_of::<NOTIFYICONDATAW>() as u32;
    nid.hWnd = hwnd;
    nid.uID = TRAY_ICON_UID;
    Shell_NotifyIconW(NIM_DELETE, &nid);
}

pub unsafe fn show_tray_menu(hwnd: HWND) {
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
        MF_STRING | if lyrics_enabled { MF_CHECKED } else { MF_UNCHECKED },
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

pub(crate) fn to_wide(s: &str) -> Vec<u16> {
    s.encode_utf16().chain(std::iter::once(0)).collect()
}
