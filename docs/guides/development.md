# Development Guide

## Prerequisites

- **OS:** Windows 10 or Windows 11 (64-bit).
- **Rust:** Stable `1.75+` (`rustup default stable-x86_64-pc-windows-msvc`).
- **MSVC Build Tools:** Visual Studio 2022 Build Tools with "Desktop development with C++".

## Common Commands

```powershell
# Run in debug mode
cargo run

# Format code
cargo fmt

# Lint with strict warnings (matches CI)
cargo clippy --all-targets -- -D warnings

# Run all unit tests
cargo test

# Build and open local rustdoc API docs
cargo doc --no-deps --open
```

## Debugging Tips

### Viewing Console Output on Windows
`src/main.rs` sets `#![cfg_attr(target_os = "windows", windows_subsystem = "windows")]` so launching the binary does not flash a terminal window. When debugging locally with `println!` / `dbg!`, temporarily comment out line 6 of `src/main.rs` or inspect output in a debugger.

### Testing DPI Scaling
1. Open **Windows Settings → System → Display → Scale**.
2. Test at `100%`, `125%`, and `150%`.
3. Verify that:
   - The island stays centered horizontally at `y = 0`.
   - Clicking immediately outside the ear flares passes through to underlying windows (`HTTRANSPARENT`).
   - Clicking inside the compact pill, expanded controls, and lyric lines registers accurately (`HTCLIENT`).

### Verifying Zero-CPU Idle
1. Pause all media playback on Windows.
2. Open **Task Manager → Details / Processes** and observe `fidra-island.exe`.
3. CPU usage should settle to `~0.0%` because `eq_timer` is stopped when paused, `sys_timer` only updates on value change every `5s`, and the media worker sleeps `1000ms` between polls.

### Testing Synced Lyrics
- Play a track with known synced lyrics on Spotify, YouTube Music, or browser media.
- Click the Lyrics icon in the top-right utility bar of the expanded island.
- Test rapid track skipping (5+ tracks in a row) to confirm request deduplication and responsive playback timestamps.
