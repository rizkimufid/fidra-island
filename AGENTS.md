# AGENTS.md — AI Coding Agent Operating Manual

This file is the authoritative instruction set for AI coding agents and automated contributors working in the **Fidra Island** (`fidra-island`) repository. Read this before modifying code or documentation.

---

## 1. Project Identity & Scope

- **Canonical Name:** Fidra Island (crate/binary: `fidra-island`). Do not reintroduce legacy names (`fidra-music`, `Fira Music Island`, `FIRA Ambient`).
- **Stack:** Rust (edition 2021, MSRV `1.75`) + Slint 1 (`slint`, `slint-build`) + Windows WinRT (`windows` 0.61) + Win32 (`windows-sys` 0.59) + HTTP (`ureq` 3).
- **Target Platform:** Windows 10/11 (primary). Non-Windows builds MUST remain compilable via `#[cfg(not(target_os = "windows"))]` stubs.
- **Scope Boundary:** Lightweight desktop Dynamic Island media controller & synced lyrics overlay. Keep dependencies minimal and binary footprint small.

---

## 2. Repository Map

| Path | Responsibility |
|---|---|
| `src/main.rs` | Thin binary entrypoint delegating lifecycle to `app::run()`. |
| `src/app.rs` | Slint UI bootstrap, event loop, timers, config persistence, tray action dispatcher, and worker thread orchestration. |
| `src/config.rs` | Atomic JSON configuration persistence (`%APPDATA%\FidraIsland\config.json`). |
| `src/logging.rs` | Rotating structured file logger (`%LOCALAPPDATA%\FidraIsland\logs\fidra-island.log`). |
| `src/platform/` | Cross-platform abstraction (`mod.rs`), Windows Win32 hit-testing, display/DPI events, System Tray, Registry auto-start, `ComGuard` (`windows.rs`), and non-Windows stubs (`stub.rs`). |
| `src/media.rs` | Windows GSMTC media polling, timeline extrapolation, delta thumbnail updates (`ThumbnailUpdate`), non-blocking transport commands, non-Windows stub. |
| `src/lyrics.rs` | LRCLIB HTTP client (`OnceLock<ureq::Agent>`), metadata sanitization (`OnceLock<Regex>`), LRC & enhanced `<mm:ss.xx>` word-sync parser, unit tests. |
| `ui/main.slint` | Declarative Slint UI (`MainWindow`), compact/expanded morphing layers, animations, Lucide SVG icon usage. |
| `ui/components/*.slint` | Reusable Slint UI components (`battery_ring.slint`, `equalizer.slint`, `controls.slint`). |
| `assets/icons/*.svg` | Official Lucide vector icons compiled directly into the binary via `@image-url(...)`. |
| `build.rs` | Compiles `ui/main.slint` at build time via `slint-build`. |
| `docs/` | Living documentation (architecture, Windows integration, UI/motion spec, ADRs, guides, roadmap). |
| `.github/workflows/ci.yml` | Automated formatting, linting, unit test, and `rustdoc` verification. |

---

## 3. Hard Architectural Invariants (DO NOT VIOLATE)

1. **Never Block the Slint UI Thread:**
   - All Windows GSMTC queries/transport calls MUST run on the dedicated media worker thread (`src/media.rs`).
   - All LRCLIB network requests MUST run on the dedicated lyric fetcher thread (`src/main.rs`).
   - Communication back to Slint MUST use `Weak::upgrade_in_event_loop`.
2. **Never Resize the Root OS Window Per Frame:**
   - `MainWindow` is fixed at `680px x 220px` logical pixels (`no-frame: true`, `background: transparent`, `always-on-top: true`).
   - Animate the inner `island-wrapper` / `island` container instead.
   - `is-pinned` controls *stay-expanded* behavior only; NEVER bind `always-on-top: is-pinned`.
3. **Keep Hit-Test & UI Dimensions Strictly Synchronized (DPI-Aware):**
   - Any change to island widths/heights/ear-flare width in `ui/main.slint` MUST be mirrored in the constants at the top of `src/platform/mod.rs` (`WINDOW_WIDTH`, `COMPACT_WIDTH`, `COMPACT_HOVER_WIDTH`, `COMPACT_LYRIC_MIN_WIDTH`, `COMPACT_LYRIC_MAX_WIDTH`, `EXPANDED_WIDTH`, `EXPANDED_LYRICS_WIDTH`, `COMPACT_HEIGHT`, `EXPANDED_HEIGHT`, `EAR_FLARE_WIDTH`).
   - All Win32 physical pixel coordinates in `window_proc` (`WM_NCHITTEST`) MUST be converted to Slint logical pixels before comparing against layout constants.
4. **Strict File Size Limit (< 500 Lines of Code):**
   - No source file (`.rs`, `.slint`) may exceed **500 lines of code**.
   - Proactively split modules into domain-focused submodules (e.g. `src/lyrics/{mod,parser,cache,client}.rs`, `ui/components/*.slint`) before a file approaches 400-450 lines.
5. **FFI Safety in Win32 Callbacks:**
   - Every `unsafe extern "system" fn` callback (such as `window_proc`) MUST wrap Rust logic in `std::panic::catch_unwind` so a panic never unwinds across an FFI boundary.
6. **Zero-CPU Idle & Zero Redundant Work:**
   - Animation timers (`eq_timer`) MUST be stopped when `is_playing == false`.
   - Never clone or re-decode album art when the track has not changed; always use `ThumbnailUpdate`.
   - Deduplicate system status updates (clock/battery) before calling Slint property setters.
   - Compile regexes and HTTP `ureq::Agent` once via `OnceLock`.
7. **Visual Standards:**
   - Never use OS Unicode emojis for UI icons; use Lucide SVG assets in `assets/icons/*.svg` with `colorize`.
   - Always clamp dynamic ratios (`progress`, `battery-level`) with `Math.clamp(val, 0.0, 1.0)`.

---

## 4. Required Verification Commands

Before considering any code change complete, run all four checks and ensure **zero errors and zero warnings**:

```powershell
cargo fmt --check
cargo clippy --all-targets -- -D warnings
cargo test
cargo doc --no-deps
```

---

## 5. Documentation Sync Rules

Documentation is a first-class production artifact and is tracked in Git.

- **Architecture / Threading / Channel Changes:** Update [`docs/architecture/overview.md`](docs/architecture/overview.md) (including Mermaid diagrams).
- **Win32 / DPI / Windowing / GSMTC Changes:** Update [`docs/architecture/windows-integration.md`](docs/architecture/windows-integration.md).
- **UI Dimensions / Colors / Motion Curves / Components:** Update [`docs/design/ui-motion-spec.md`](docs/design/ui-motion-spec.md).
- **Significant Technical Decisions:** Add a new numbered ADR under `docs/adr/` using [`docs/adr/0000-template.md`](docs/adr/0000-template.md) and register it in [`docs/adr/README.md`](docs/adr/README.md).
- **User-Facing Changes:** Record under `[Unreleased]` in [`CHANGELOG.md`](CHANGELOG.md).
- **Public Rust Modules / Structs / Functions:** Maintain accurate `//!` and `///` `rustdoc` comments so `cargo doc --no-deps` stays warning-free.
