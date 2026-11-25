# Changelog

All notable changes to **Fidra Island** will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.1.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [Unreleased]

### Added
- Persistent lyrics disk cache (`%LOCALAPPDATA%\FidraIsland\cache\lyrics\`) with positive (14-day) and negative (2-day) TTL for instant offline playback.
- Inno Setup installer script (`packaging/installer.iss`) for single-click Windows per-user installation with desktop shortcut and uninstaller.
- Automated GitHub Actions release pipeline (`.github/workflows/release.yml`) building portable ZIP, installer setup, and `SHA256SUMS.txt` on version tags.
- Native Windows System Tray icon and context menu (*Show/Hide*, *Synced Lyrics*, *Launch at Startup*, *Open Logs Folder*, *Quit*) with clean `NIM_DELETE` teardown.
- Atomic persistent JSON configuration (`%APPDATA%\FidraIsland\config.json`) preserving `lyrics_enabled`, `is_pinned`, and `launch_at_startup`.
- Windows Registry sign-in auto-start support (`HKCU\Software\Microsoft\Windows\CurrentVersion\Run`).
- Rotating structured file logger (`%LOCALAPPDATA%\FidraIsland\logs\fidra-island.log`, `2 MiB` rotation).
- Automatic overlay re-centering on `WM_DISPLAYCHANGE` and `WM_DPICHANGED`.
- Explicit RAII COM MTA initialization (`ComGuard`) on the media worker thread.
- Modularized Rust codebase (`src/app.rs`, `src/config.rs`, `src/logging.rs`, `src/platform/`) and Slint UI components (`ui/components/{battery_ring,equalizer,controls}.slint`).
- Production documentation suite (`AGENTS.md`, `CONTRIBUTING.md`, `CHANGELOG.md`, `docs/architecture/`, `docs/design/`, `docs/adr/`, `docs/guides/`, `docs/roadmap.md`).
- GitHub Actions CI workflow (`.github/workflows/ci.yml`) running `cargo fmt`, `cargo clippy`, `cargo test`, and `cargo doc`.
- Unit test suites for LRC parsing, word-sync tag stripping, `[offset:]` support, metadata sanitization, lyric context lookup, time formatting, and battery arc geometry.
- Bounded in-memory lyric cache (`64` tracks) and channel request deduplication on rapid track skips.
- Graceful shutdown signaling for background worker threads on event loop exit.
- `[profile.release]` optimizations (`opt-level = "s"`, `lto = "thin"`, `codegen-units = 1`, `strip = true`).

### Changed
- Unified product and crate naming to **Fidra Island** (`fidra-island`).
- Removed `docs` and `Cargo.lock` from `.gitignore` so documentation and lockfile are version-controlled.
- Replaced per-poll thumbnail cloning with incremental `ThumbnailUpdate` deltas.
- Pre-compiled metadata cleanup regexes and shared a pooled `ureq::Agent` with a compliant `User-Agent` header.
- Deduplicated system clock and battery updates (5-second refresh, mutating Slint only on value change).
- Stopped equalizer timer completely when playback is paused for true zero-CPU idle.

### Removed
- Removed unused mockup audio icons (`volume-*.svg`, `mic.svg`) from `assets/icons/`.

### Fixed
- Fixed DPI scaling mismatch in `position_overlay()` and Win32 `WM_NCHITTEST` hit-testing on >100% Windows displays.
- Fixed `always-on-top` dropping `HWND_TOPMOST` when unpinning the island.
- Added `std::panic::catch_unwind` FFI panic boundary inside Win32 `window_proc`.
- Fixed raw `<mm:ss.xx>` word-sync tags leaking into `LyricLine.text`.
- Fixed overlapping lyric line transition timers using a generation counter.
- Synchronized Win32 hit-test island widths with `ui/main.slint` clamp and hover dimensions.

## [0.1.0] - 2026-11-25

### Added
- Initial Rust + Slint Dynamic Island prototype with Windows GSMTC media control, LRCLIB synced lyrics, Win32 click-through, and compact/expanded morphing UI.
