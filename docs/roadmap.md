# Production Roadmap

This roadmap tracks the prioritized milestones to take **Fidra Island** from `v0.2.0` to a polished `v1.0.0` desktop release.

## Phase 1 — Repo Hygiene & Living Documentation ✅
- Track `docs/` and `Cargo.lock` in Git.
- Establish `AGENTS.md`, `CONTRIBUTING.md`, `CHANGELOG.md`, ADRs, Mermaid architecture docs, and UI/Motion SSOT.
- Add GitHub Actions CI (`fmt`, `clippy`, `test`, `rustdoc`).

## Phase 2 — Critical Stability & Performance Fixes ✅
- DPI-aware `position_overlay()` and Win32 `WM_NCHITTEST` click-through.
- Incremental `ThumbnailUpdate` deltas (eliminate 4x/sec album art re-decoding).
- Fix `always-on-top` vs `is-pinned` conflict.
- Add `catch_unwind` FFI panic boundary in `window_proc`.
- True zero-CPU idle (`eq_timer` stopped when paused; `sys_timer` deduplicated).
- Fix raw `<mm:ss.xx>` word-sync tag leak, add `[offset:]` support, cache regexes & HTTP agent, deduplicate rapid lyric fetches.
- Add initial unit test suite.

## Phase 3 — Desktop Production Hardening ✅
- [x] **System Tray & Quit Menu:** Native Windows notification area icon with *Show/Hide*, *Toggle Lyrics*, *Launch at Startup*, *Open Logs*, and *Quit*.
- [x] **Structured File Logging:** Rotating file logger under `%LOCALAPPDATA%\FidraIsland\logs\fidra-island.log` for post-mortem diagnostics.
- [x] **Persistent Configuration:** Save/load user preferences (`lyrics_enabled`, `is_pinned`, `launch_at_startup`) atomically in `%APPDATA%\FidraIsland\config.json`.
- [x] **Multi-Monitor & Dynamic Display Events:** Handle `WM_DISPLAYCHANGE` and `WM_DPICHANGED` to re-center automatically when monitors/DPI change.
- [x] **Explicit COM/WinRT Initialization:** RAII `ComGuard` initializing MTA explicitly on the media worker thread.
- [x] **Modular Refactoring:** Split `src/main.rs` into `app.rs`, `config.rs`, `logging.rs`, `platform/{mod,windows,stub}.rs` and extracted `ui/components/{battery_ring,equalizer,controls}.slint`.

## Phase 4 — Distribution & Polish (`v1.0.0`) ✅
- [x] **Asset & Media Control Hygiene:** Removed unused mockup audio icons (`volume-*.svg`, `mic.svg`) to preserve zero dead code standard.
- [x] **Disk Cache for Lyrics:** Persist LRCLIB responses locally in `%LOCALAPPDATA%\FidraIsland\cache\lyrics` with positive (14-day) and negative (2-day) TTL for offline playback.
- [x] **Automated Release Pipeline:** Inno Setup script (`packaging/installer.iss`) and GitHub Actions release workflow (`.github/workflows/release.yml`) building portable ZIP, installer setup executable, and SHA-256 checksums.

## Phase 5 — Living Companion & Multi-Widget Architecture (Active)
- [ ] **Adaptive Cover Art Ambience:** Dominant vibrant color extraction from thumbnail bytes in Rust, driving dynamic ambient surface lighting and specular hardware top-rim in Slint.
- [ ] **Kinetic Waterfall Lyric Stream:** Continuous vertical scrolling with incoming line scale-up (`12px -> 15.5px bold`, `opacity: 0.4 -> 1.0`) and outgoing line scale-down (`15.5px -> 12px`, `opacity: 1.0 -> 0.4`) on deceleration spring curve.
- [ ] **Strict < 500 Lines Modularization:** Decompose `src/lyrics.rs`, `src/app.rs`, `src/platform/windows.rs`, and `ui/main.slint` into modular submodules under 500 lines.
- [ ] **Hydration Widget (Reminder Minum):** Configurable interval timer (e.g. 45-60m), dynamic droplet alert banner pulse, and daily habit ring progress.
- [ ] **Prayer Times Widget (Jadwal Sholat):** Offline astronomical prayer calculation (Kemenag / MWL method), next-prayer pill countdown, and adhan temporary notification pulse.
- [ ] **Daily Task Checklist:** Lightweight local to-do checklist in expanded widget hub with persistent `%APPDATA%\FidraIsland\tasks.json`.
