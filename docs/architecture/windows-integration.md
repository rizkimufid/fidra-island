# Windows Platform Integration

Fidra Island relies on targeted Win32 and WinRT APIs to behave like a native OS notch overlay. This document details the exact invariants required to keep Windows behavior stable across DPI scales and display setups.

---

## 1. Transparent Click-Through (`WM_NCHITTEST`)

Because resizing an OS window per frame causes visible DWM stutter, Fidra Island allocates a fixed transparent bounding window (`680px x 220px` logical) and intercepts `WM_NCHITTEST` via Win32 window subclassing (`GWLP_WNDPROC`).

### Coordinate Space & DPI Scaling

Win32 `WM_NCHITTEST` delivers screen coordinates in **physical pixels**, while Slint layout dimensions are authored in **logical pixels**.

1. Convert screen coordinates to client coordinates via `ScreenToClient(hwnd, &mut pt)`.
2. Compute the window's live DPI scale factor from `GetClientRect`:
   ```text
   scale = max((rect.right - rect.left) / WINDOW_WIDTH, 1.0)
   logical_x = round(pt.x / scale)
   logical_y = round(pt.y / scale)
   ```
3. Compute the active island contour in logical pixels from shared atomics (`IS_EXPANDED`, `IS_HOVERED`, `LYRICS_ENABLED`, `SHOW_LYRICS`, `CURRENT_LYRIC_WIDTH`):
   - Expanded + Lyrics: `610 x 175`
   - Expanded (Player only): `320 x 175`
   - Compact + Lyrics: `clamp(CURRENT_LYRIC_WIDTH, 280, 420) x 38`
   - Compact (Hovered): `310 x 38`
   - Compact (Resting): `290 x 38`
4. Include the `14px` tangential ear flares on both sides:
   - Return `HTCLIENT` when `logical_x ∈ [island_x - 14, island_x + island_w + 14]` and `logical_y ∈ [0, island_h]`.
   - Return `HTTRANSPARENT` everywhere else so clicks pass through to underlying apps.

### FFI Panic Safety

All Rust logic inside `unsafe extern "system" fn window_proc` MUST stay wrapped in `std::panic::catch_unwind`. Unwinding across a Win32 FFI boundary is Undefined Behavior and immediately aborts the process.

---

## 2. Overlay Positioning & Taskbar Exclusion

- **Top-Edge Centering (`position_overlay`)**:
  - Reads `window.window().scale_factor()` from Slint, multiplies `WINDOW_WIDTH` to obtain physical width, and centers against `GetSystemMetrics(SM_CXSCREEN)` at `y = 0`.
- **Tool Window Style (`setup_hit_test`)**:
  - Applies `WS_EX_TOOLWINDOW` and strips `WS_EX_APPWINDOW` (with a brief `SW_HIDE` -> `SW_SHOWNOACTIVATE` cycle) so Explorer excludes the overlay from the Taskbar and Alt+Tab switcher.
  - Enforces `HWND_TOPMOST` with `SWP_NOACTIVATE` so the island never steals focus from active applications.
  - `MainWindow` sets `always-on-top: true` permanently; `is-pinned` controls *stay-expanded* behavior only.

---

## 3. Windows GSMTC Media Control

- Uses `windows::Media::Control::GlobalSystemMediaTransportControlsSessionManager` exclusively on the background media worker thread.
- **Why off the main thread?** Calling WinRT `.get()` synchronously on Slint's UI thread deadlocks with the Windows COM message pump.
- **Timeline Extrapolation:** Many media players (Chrome, Spotify, Electron apps) update `TimelineProperties.Position` infrequently. Fidra Island compensates by reading `LastUpdatedTime` (`100ns` Windows FileTime ticks), adding elapsed wall-clock age while playing, and maintaining a monotonic `Instant` anchor that ignores stale re-reads.

---

## 4. Native System Telemetry

- **Clock:** `GetLocalTime` (`HH:mm`).
- **Battery:** `GetSystemPowerStatus` (`BatteryLifePercent`, `ACLineStatus == 1`).
- Updates run on a 5-second timer and are deduplicated against `SystemStatusCache` so Slint only redraws when the minute or battery percentage changes.

---

## 5. System Tray, Auto-Start & Display Events

- **Native Notification Area Icon (`Shell_NotifyIconW`):**
  - Registered in `setup_window_and_tray()` with `WM_TRAYICON = WM_APP + 1` and removed cleanly via `remove_tray_icon()` (`NIM_DELETE`) on shutdown.
  - Right-click opens a native Win32 popup menu (*Show/Hide*, *Synced Lyrics*, *Launch at Startup*, *Open Logs Folder*, *Quit*); double-click toggles overlay visibility.
  - Tray actions are dispatched to a zero-CPU blocking channel thread (`tray_rx.recv()`) which forwards UI updates via `upgrade_in_event_loop` or triggers `slint::quit_event_loop()`.
- **Windows Sign-In Auto-Start (`HKCU\Software\Microsoft\Windows\CurrentVersion\Run`):**
  - `set_launch_at_startup(bool)` writes or deletes the `FidraIsland` `REG_SZ` value pointing to the current executable.
- **Dynamic Display & DPI Changes:**
  - `window_proc` listens for `WM_DISPLAYCHANGE` and `WM_DPICHANGED` (wrapped in `catch_unwind`) and automatically re-centers the overlay at `y = 0` via `SetWindowPos`.
- **Explicit COM Initialization (`ComGuard`):**
  - The media worker thread initializes COM MTA (`CoInitializeEx(COINIT_MULTITHREADED)`) via an RAII `ComGuard` that calls `CoUninitialize()` on drop.

