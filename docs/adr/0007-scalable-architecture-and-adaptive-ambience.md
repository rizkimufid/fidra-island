# ADR-0007: Scalable Multi-Widget Architecture, <500-Line Code Limit, and Adaptive Ambience

- **Status:** Accepted
- **Date:** 2026-11-25

## Context

Fidra Island is expanding from a dedicated music overlay into an **All-in-One Desktop Companion** with upcoming personal productivity and wellbeing widgets:
1. Drink Water Reminder (interval alerts, droplet pop banner, habit ring).
2. Prayer Times (offline calculation, countdown pill, adhan pulse).
3. Daily Task Checklist (lightweight local to-do checklist).

At the same time, maintaining maintainability requires a strict structural rule: source files must stay small, modular, and readable (strict **< 500 lines per file**, inspired by the Ponytail standard). Furthermore, the visual aesthetic needs to move away from a generic flat box into an organic, living hardware companion with adaptive cover art lighting and a physical waterfall lyric stream.

## Decision

1. **Strict File Size Constraint (< 500 LOC):**
   - No source file (`.rs`, `.slint`) may exceed 500 lines.
   - Modules approaching 400 lines are proactively split into submodules:
     - `src/lyrics/` -> `mod.rs`, `parser.rs`, `cache.rs`, `client.rs`.
     - `src/app/` -> `mod.rs`, `callbacks.rs`, `state.rs`.
     - `src/platform/windows/` -> `mod.rs`, `hit_test.rs`, `tray.rs`.
     - `ui/` -> decomposed into standalone components under `ui/components/`.
2. **Adaptive Cover Art Ambience:**
   - On thumbnail delta arrival in the media worker, extract the primary vibrant color (sampling downscaled pixels with optimal saturation/luminance thresholding) at 0% runtime CPU cost.
   - Bind `ambient-color` to `MainWindow` in Slint with smooth 600ms ease-in-out transitions across the island body and a 1px specular hardware rim highlight.
3. **Kinetic Waterfall Lyric Stream:**
   - Replace isolated single-line swaps with a continuous vertical stream: incoming line scales up (`12px -> 15.5px bold`, `opacity: 0.4 -> 1.0`), outgoing line scales down (`15.5px -> 12px`, `opacity: 1.0 -> 0.4`), driven by a `340ms` deceleration spring curve (`cubic-bezier(0.16, 1.0, 0.3, 1.0)`).
4. **Multi-Widget Architecture:**
   - Establish an extensible widget model where the compact notch supports dual habit rings, and the expanded view supports a segmented capsule switcher between Lyrics, Prayer Times, and Daily Tasks.

## Consequences

### Positive
- Codebase remains remarkably clean, modular, and easy to maintain without monolithic files.
- Visual presentation is vibrant and dynamic, adapting to album artwork like modern mobile operating systems.
- Seamless runway for adding Prayer Times, Hydration alerts, and Daily Tasks without rewriting the core layout.

### Trade-offs & Risks
- More files in directory tree; mitigated by clean folder organization and explicit module exports.
