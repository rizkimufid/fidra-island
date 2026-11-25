# ADR-0001: Use Rust + Slint for Native Desktop Island UI

- **Status:** Accepted
- **Date:** 2026-11-25

## Context

Fidra Island needs to run continuously in the background on Windows as a top-edge Dynamic Island overlay with smooth spring animations, minimal RAM/CPU overhead, direct Win32/WinRT interoperability, and zero webview/Electron memory footprint.

## Decision

Build Fidra Island as a standalone native Rust executable using **Slint 1** (`slint` + `slint-build`) for declarative UI and animations, compiling SVG icons (`assets/icons/*.svg`) and UI definitions (`ui/main.slint`) directly into the binary.

## Consequences

### Positive
- Low memory footprint and native startup speed without a bundled browser engine.
- Compile-time checked UI bindings between Rust and Slint.
- Direct access to `windows` (WinRT GSMTC) and `windows-sys` (Win32 `WM_NCHITTEST`, system telemetry).

### Trade-offs & Risks
- Platform-specific windowing polish (click-through, taskbar hiding, DPI hit-testing) requires small `unsafe` Win32 FFI bridges.
- Non-Windows targets require stub implementations (`src/media.rs`) to preserve cross-platform compilation.
