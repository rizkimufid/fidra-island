# Changelog

All notable changes to **Fidra Island** will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.1.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [Unreleased]

### Added
- **Jadwal Sholat 2-Page Carousel Slider (`ui/components/prayer_card.slint`):** Full 6-prayer schedule card (Subuh, Terbit, Dzuhur, Ashar, Maghrib, Isya) accessible as Page 2 in expanded mode with smooth horizontal slide animation (`cubic-bezier(0.22, 1.0, 0.36, 1.0)`), clickable pagination dots, top toolbar moon icon toggle, next-prayer glowing cards, and DPI-aware hit-test synchronization.
- **Jadwal Sholat (Prayer Times Countdown):** Built-in offline astronomical solar calculation (`src/prayer.rs`) aligning with Indonesian Kemenag (Subuh 20°, Isya 18°, Ihtiyat +2m) and global standards. Features an ambient countdown pill (`assets/icons/moon.svg`) in the compact island showing the next prayer (e.g. *Ashar 45m*), with emergency pulse alert during the final 2 minutes before adhan. Configurable coordinates (defaults to Jakarta UTC+7) persistent in `%APPDATA%\FidraIsland\config.json`.
- **CJK Transliteration (Romaji/Pinyin/Hangul Latin):** Added automatic romanization for East Asian lyrics (Japanese via `kakasi`, Chinese via `pinyin` with tone marks, and Korean via Revised Romanization algorithmic mapping in `src/lyrics/romanize.rs`). Subtitles render cleanly below active CJK lyric lines in Plus Jakarta Sans. Includes an on/off toggle button (`assets/icons/languages.svg`) in the top-right toolbar with state saved to `%APPDATA%\FidraIsland\config.json` (defaults to enabled).
- **Bundled typography:** Plus Jakarta Sans (Regular/Bold) is now the single UI typeface; lyric lines containing Japanese script (hiragana, katakana, CJK) render in LINE Seed JP (Regular/Bold). Fonts live in `assets/fonts/` with their SIL OFL 1.1 licenses and are embedded in the binary (~7.7 MB total).
- `FIDRA_SYNC_DEBUG=1` environment variable enables verbose GSMTC timeline logging (`media::sync`) to diagnose browser-specific lyric desync.
- **Dynamic Cover Art Ambient Glow:** Fast downsampled (24x24) dominant color extractor (`src/media/color.rs`) scoring vibrant saturation to pass dynamic `ambient-color` into Slint's `@radial-gradient` and progress bar accent.
- **Specular Hardware Top Rim:** 1px glass specular highlight across the island top container for physical bezel realism.
- **Kinetic Waterfall Lyric Animation:** Smooth continuous vertical waterfall stream in `ui/components/lyrics_column.slint`, interpolating line positions and typography scale (12px to 15.5px) with spring deceleration curves (`cubic-bezier(0.16, 1.0, 0.3, 1.0)`).
- **Strict < 500 LOC Modular Architecture:** Decomposed monolithic UI and application logic to adhere strictly to the rule:
  - `src/app/{mod.rs, callbacks.rs}`
  - `src/platform/windows/{mod.rs, hit_test.rs, tray.rs}`
  - `src/media/{mod.rs, color.rs, gsmtc.rs, stub.rs}`
  - `src/lyrics/{mod.rs, parser.rs, cache.rs, client.rs}`
  - `ui/components/{compact_layer.slint, player_column.slint, lyrics_column.slint, ear_flares.slint}`
- **Companion Widget Roadmap:** Expanded `docs/roadmap.md` with detailed architecture and milestones for Hydration / Water Reminder, Prayer Times (Jadwal Sholat), and Daily Task Companion.
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
