# Fidra Island

A lightweight, native Windows **Dynamic Island** media controller with real-time synced lyrics, built with **Rust** and **Slint**.

Fidra Island sits flush against the top edge of your primary display, providing glanceable playback status, native Windows system clock & battery telemetry, hover-to-expand playback controls, and Apple Music-style synced lyrics (with optional enhanced word-by-word sync and click-to-seek) — without stealing window focus or blocking clicks outside the active notch contour.

## Features

- **Native Windows GSMTC Integration**: Automatically tracks active media sessions (Spotify, YouTube Music, Apple Music, browser media, etc.) with real-time timeline extrapolation and non-blocking transport controls (Play/Pause, Next, Previous, Seek).
- **Synced & Word-Level Lyrics (LRCLIB)**: Asynchronous background lookup with metadata sanitization, `[offset:]` support, in-memory caching, request deduplication, 3-line display, and interactive click-to-seek.
- **Seamless Top-Edge Morphing UI**: Fixed `680x220` logical transparent viewport with asymmetric spring curves, tangential ear flares, and DPI-aware `WM_NCHITTEST` desktop click-through.
- **Zero-CPU Idle Architecture**: Equalizer timer stops completely when playback pauses, system status updates are deduplicated, and album artwork is decoded only on track change.
- **Native System Telemetry**: Compact mode displays local time (`HH:mm`) and a circular battery ring with charging state via Win32 APIs.

## Quick Start

### Prerequisites

- **Windows 10 / 11** (primary target; non-Windows targets compile with a no-op media stub).
- **Rust toolchain** (`1.75+`, MSVC target `x86_64-pc-windows-msvc`).
- Visual Studio Build Tools with C++ Desktop Development workload (Windows SDK).

### Build & Run

```powershell
# Run in development mode
cargo run

# Build optimized release binary
cargo build --release
```

The release executable is emitted to `target/release/fidra-island.exe`.

### Verify Locally

```powershell
cargo fmt --check
cargo clippy --all-targets -- -D warnings
cargo test
cargo doc --no-deps
```

## Documentation

Full living documentation lives in [`docs/`](docs/README.md):

| Document | Purpose |
|---|---|
| [Documentation Hub](docs/README.md) | Complete map of architecture, design, guides, and ADRs |
| [System Architecture](docs/architecture/overview.md) | Threading model, channels, media & lyrics data flow (Mermaid diagrams) |
| [Windows Integration](docs/architecture/windows-integration.md) | Win32 `WM_NCHITTEST`, DPI scaling, `WS_EX_TOOLWINDOW`, and GSMTC details |
| [UI & Motion Spec](docs/design/ui-motion-spec.md) | Single source of truth for dimensions, obsidian palette, and spring curves |
| [Architecture Decision Records](docs/adr/README.md) | Numbered ADRs capturing key technical decisions |
| [Development Guide](docs/guides/development.md) | Debugging, testing, and profiling CPU/GPU idle usage |
| [Release & Packaging Guide](docs/guides/release-packaging.md) | Release builds, distribution, and troubleshooting |
| [Production Roadmap](docs/roadmap.md) | Prioritized milestones toward full v1.0 release |
| [AGENTS.md](AGENTS.md) | Operating manual & invariants for AI coding agents |
| [CONTRIBUTING.md](CONTRIBUTING.md) | Contribution workflow, style rules, and PR checklist |
| [CHANGELOG.md](CHANGELOG.md) | Versioned release history |
