# Progress

## Milestones

| Milestone | Status | Acceptance criteria |
|---|---|---|
| Create isolated project | DONE | Sibling project exists and legacy `fira` remains untouched |
| Add Rust/Slint scaffold | DONE | `Cargo.toml` and initial UI entrypoint exist |
| Document architecture and decisions | DONE | Scope, boundaries, and validation criteria are recorded |
| Build static music island | DONE | Window renders track metadata and primary playback controls |
| Connect Rust state to Slint | DONE | Mock playback state updates the UI |
| Validate framework ergonomics | DONE | Findings cover authoring, state, window behavior, and integration |
| Connect Windows GSMTC media session | DONE | Current track metadata, playback status, polling, transport controls, and live thumbnail stream are wired |
| Top Dynamic Island & Hover Expansion | DONE | Automatic hover expansion with 80ms snappy debounce, 250ms exit collapse, pin toggle, and thumbnail art |
| Top-Edge Screen Notch Aesthetic | DONE | Flush top-edge placement with outward ear flares, amber dark gradient, and minimal centered typography matching reference design |
| Strict Morphing & Living Motion Architecture | DONE | Complies with `docs/UI.md`: persistent dual layers, fixed viewport, asymmetric spring physics, staggered cascade, dynamic ear flares |
| Zero-Lag Ear Flare Synchronizer (`island-wrapper`) | DONE | Rigidly locked ear fillets inside unified wrapper; eliminated double-animation cascading lag and excessive shadows |
| Seamless Subtle Inverted Fillets (14px) | DONE | Restrained tangential micro-fillets blending into top bezel with 0ms sync, zero top border, and tangential curves (`M 0 0 C 8 0 14 5 14 14 L 14 0 Z`) |
| Dynamic Content-Adaptive Island Width | DONE | Island width automatically shrinks to compact content (290px/310px compact, 620px expanded) with synced Win32 hit-testing |
| True Vertical Centering Architecture | DONE | Replaced rigid positioning with nested `VerticalLayout { alignment: center }`, eliminating top-edge sticking |
| Strict Slint UI, Layout & Motion Rules Overhaul | DONE | 680x220 root window, 290px->620x175 morphing island, 2-column split (250px player + Apple Music lyrics), 3-wave top-down waterfall drop, kinetic sweep & click-to-seek lyrics, full-bleed ambient glow |
| 100% Real-Time Media Timeline Engine | DONE | Implemented Windows GSMTC `LastUpdatedTime` age extrapolation via 100ns system ticks, ticking second-by-second at 4Hz |
| Zero-Clipping Compact Lyrics Metric | DONE | Safe dynamic glyph formula `(char_count * 6.8 + 68.0).clamp(290, 460)` preventing ellipsis truncation on sentences |
| Dynamic Spectrum Equalizer Animation | DONE | Multi-frequency organic dancing waveform (80ms keyframes + 90ms ease-in-out interpolation); rests at 3px when paused |
| Bouncy Dynamic Spring Lyric Expansion | DONE | Dynamic char-based width calculation with bouncy spring easing (`cubic-bezier(0.34, 1.45, 0.64, 1.0)`) |
| Win32 Transparent Hit-Testing (`WM_NCHITTEST`) | DONE | Full desktop click-through outside active island bounds via `HTTRANSPARENT`, dynamically tracking exact lyric & expanded dimensions (680px base) |
| Lucide Icon Vector System Integration | DONE | Replaced manual path data with official Lucide SVG vector icon assets (`assets/icons/*.svg`), compiled into binary with dynamic `colorize` |
| Native System Status in Compact Mode | DONE | Integrated Win32 `GetLocalTime` (HH:mm) and `GetSystemPowerStatus` with circular ring progress bar and charging indicators |
| Real-time Synced Lyrics Engine (LRCLIB) | DONE | Integrated `Fira`-compatible LRCLIB API pipeline in background worker with LRC parsing, metadata sanitization, and lyric toggle |
| Interactive Click-to-Seek Synced Lyrics | DONE | Wrapped lyric lines in TouchAreas with hover opacity brighten & nudge, triggering exact second media seek via `seek-to-lyric` callback |
| Non-blocking Media Worker Thread | DONE | Transport commands and WinRT polling moved to background worker thread, eliminating COM deadlocks and "Not Responding" freeze |
| Add real playback backend | IN PROGRESS | Play, pause, seek, and metadata reporting work with active sessions |
| Decide framework direction | TODO | Slint is confirmed or replaced based on prototype evidence |

## Current focus

**IN PROGRESS:** Real playback backend validation and live Windows media session refinement.

## Changelog

### Unreleased

- Fully applied **Strict Slint UI, Layout & Motion Rules**:
  - **Fixed Root Window & Island Dimensions**:
    - Root window locked to `width: 680px; height: 220px;` with frameless transparent configuration.
    - Morphing island animates between Compact (`290px` resting, `310px` on hover, `38px` height, `19px` border radius) and Expanded (`620px` width, `175px` height, `26px` border radius).
  - **Mandatory 2-Column Split (Expanded Mode)**:
    - Left Column (Music Player — `250px`):
      - Wave 1 (delay `60ms`, drop-in `y: -12px -> 0px`): `56px x 56px` album art with `12px` border radius and `14px` drop shadow, plus `15px` bold white track title and `12.5px` muted `#A1A1AA` artist.
      - Wave 2 (delay `110ms`, drop-in `y: -14px -> 0px`): Scrubbable progress bar with `10.5px` elapsed and remaining timestamps and a `4px` (expands to `6px` on hover) interactive track.
      - Wave 3 (delay `160ms`, drop-in `y: -16px -> 0px`): Centered media controls with Previous (`18px`), solid white `#FFFFFF` `42px` Play/Pause circle button with dark `#0B0C0E` vector icon, and Next (`18px`).
    - Right Column (Apple Music Synced Lyrics — `horizontal-stretch: 1`):
      - Always visible in Expanded Mode, blooming via combined horizontal unfold and top-down drop (`delay: 130ms`, `duration: 380ms`, `x: 18px -> 0px`, `y: -12px -> 0px`).
      - Staggered lyric lines: Previous (12px, #FFFFFF45, delay `150ms`), Active (16px, #FFFFFF, bold, delay `190ms`, with spring lift and kinetic sweep mask), Next (12px, #FFFFFF45, delay `230ms`).
      - Top and bottom `24px` vertical fade masks into `#0B0C0E`.
    - Top-Right Utility Controls: Pin and Collapse (▲) buttons neatly placed at `x: parent.width - self.width - 14px; y: 16px;` (`opacity: 0.55`, `1.0` on hover).
  - **Interactive Lyric Line Hover & Click-to-Seek**:
    - Inactive lines brighten (`0.35 -> 0.80`) and nudge right (`x: 0px -> 4px`) on hover.
    - Clicking a lyric line triggers `seek-to-lyric(direction)`, commanding `GlobalSystemMediaTransportControlsSession` to jump directly to that line's start timestamp.
  - **Seamless Tangential Ear Flares**:
    - Inverted fillets using `M 0 0 C 8 0 14 5 14 14 L 14 0 Z` (left) and `M 0 0 L 0 14 C 0 5 6 0 14 0 Z` (right) with `1px` overlap into the island, matching `#0B0C0E` fill, and zero top stroke along the screen bezel.
  - **Full-Bleed Ambient Glow**:
    - Replaced clipped circular radial gradient with seamless `@linear-gradient(125deg, #1E293B77 0%, #0B0C0E 55%, #0B0C0E 100%)`.
  - **Win32 Dynamic Hit-Testing**:
    - Synchronized `WM_NCHITTEST` coordinates to the new `680px` window base (`island_w = 620`, `island_h = 175` in expanded; `island_w = 290/310`, `island_h = 38` in compact).
