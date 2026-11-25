# ADR-0003: Fixed Transparent Viewport with DPI-Aware Win32 `WM_NCHITTEST`

- **Status:** Accepted
- **Date:** 2026-11-25

## Context

Animating OS window dimensions (`SetWindowPos`) per frame on Windows causes visible DWM composition stutter. Conversely, keeping a large fixed transparent window (`680px x 220px`) blocks mouse clicks to underlying desktop windows when the island is collapsed to its compact notch (`290px x 38px`).

## Decision

1. Lock the root Slint `Window` to `680px x 220px` logical pixels (`no-frame: true`, `background: transparent`, `always-on-top: true`) and animate only the inner `island-wrapper` / `island` container.
2. Subclass the Win32 `HWND` (`GWLP_WNDPROC`) to intercept `WM_NCHITTEST`, convert physical client coordinates to logical pixels using live `GetClientRect` DPI scaling, and return `HTCLIENT` inside the active island contour (including `14px` ear flares) and `HTTRANSPARENT` everywhere else.
3. Wrap the `WM_NCHITTEST` logic in `std::panic::catch_unwind` to prevent undefined behavior across the FFI boundary.

## Consequences

### Positive
- Butter-smooth `60fps+` expand/collapse morphing with zero DWM resize stutter.
- Accurate click-through on `100%`, `125%`, `150%`, and `200%` Windows DPI scales.

### Trade-offs & Risks
- Island geometry constants in `src/main.rs` MUST stay synchronized with `ui/main.slint`.
- Multi-monitor / dynamic `WM_DISPLAYCHANGE` repositioning remains a Phase 3 follow-up.
