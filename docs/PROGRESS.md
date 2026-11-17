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
| Top Dynamic Island & Hover Expansion | DONE | Screen top-docking (y=0), hover expansion with 800ms debounce, pin toggle, and thumbnail art |
| Top-Edge Screen Notch Aesthetic | DONE | Flush top-edge placement with outward ear flares, amber dark gradient, and minimal centered typography matching reference design |
| Strict Morphing & Living Motion Architecture | DONE | Complies with `docs/UI.md`: persistent dual layers, fixed 480x410 viewport, asymmetric spring physics, staggered cascade, dynamic ear flares |
| Zero-Lag Ear Flare Synchronizer (`island-wrapper`) | DONE | Rigidly locked ear fillets inside unified wrapper; eliminated double-animation cascading lag and excessive shadows |
| Seamless Subtle Inverted Fillets (7px) | DONE | Restrained micro-fillets blending into top bezel with 0ms sync and zero giant horns |
| Dynamic Content-Adaptive Island Width | DONE | Island width automatically shrinks to compact content (184px default, dynamic lyric-width for lyrics, 384px expanded) with synced Win32 hit-testing |
| True Vertical Centering Architecture | DONE | Replaced rigid positioning with nested `VerticalLayout { alignment: center }`, eliminating top-edge sticking |
| macOS Minimalist Compact Expanded Surface | DONE | Compact 176px single-surface design with dedicated lyrics toggle, integrated volume slider, and sleek controls |
| 100% Real-Time Media Timeline Engine | DONE | Implemented Windows GSMTC `LastUpdatedTime` age extrapolation via 100ns system ticks, ticking second-by-second at 4Hz |
| Pixel-Perfect Horizontal & Vertical Control Alignment | DONE | Aligned volume deck and transport buttons along a single shared center line, with live equalizer badge on album art |
| Dynamic Spectrum Equalizer Animation | DONE | Multi-frequency organic dancing waveform (80ms keyframes + 90ms ease-in-out interpolation); rests at 3px when paused |
| Bouncy Dynamic Spring Lyric Expansion | DONE | Dynamic char-based width calculation (`(char_count * 8.0 + 104).clamp(210, 420)`) with bouncy spring easing (`cubic-bezier(0.34, 1.45, 0.64, 1.0)`) |
| Snappy Instant Collapse (Zero Idle Delay) | DONE | Replaced 800ms sluggish debounce with instant response upon cursor exit, supported by unified composite hover tracking |
| Win32 Transparent Hit-Testing (`WM_NCHITTEST`) | DONE | Full desktop click-through outside active island bounds via `HTTRANSPARENT`, dynamically tracking exact lyric width |
| Lucide Icon Vector System Integration | DONE | Replaced manual path data with official Lucide SVG vector icon assets (`assets/icons/*.svg`), compiled into binary with dynamic `colorize` |
| Native System Status in Compact Mode | DONE | Integrated Win32 `GetLocalTime` (HH:mm) and `GetSystemPowerStatus` with circular ring progress bar and charging indicators |
| Real-time Synced Lyrics Engine (LRCLIB) | DONE | Integrated `Fira`-compatible LRCLIB API pipeline in background worker with LRC parsing, metadata sanitization, and lyric toggle |
| Dynamic Compact State Switching | DONE | Compact pill displays Clock & Battery ring by default; switches to Lucide Music Icon & Synced Lyric Line when lyrics are active |
| Non-blocking Media Worker Thread | DONE | Transport commands and WinRT polling moved to background worker thread, eliminating COM deadlocks and "Not Responding" freeze |
| Add real playback backend | IN PROGRESS | Play, pause, seek, and metadata reporting work with active sessions |
| Decide framework direction | TODO | Slint is confirmed or replaced based on prototype evidence |

## Current focus

**IN PROGRESS:** Real playback backend validation and live Windows media session refinement.

## Changelog

### Unreleased

- Created isolated music project direction.
- Added Rust package manifest and Slint UI directory.
- Added architecture, decisions, and progress tracking.
- Added compact/expanded island UI with progress, volume, queue, and mock controls.
- Connected playback, navigation, expansion, seek, and volume callbacks to Rust state.
- Converted the normal window into a frameless always-on-top overlay with compact capsule and expanded modes.
- Added Windows-native top-center positioning using `windows-sys` screen metrics.
- Added a Windows GSMTC controller for current media metadata, timeline polling, play/pause, previous, next, and seek.
- Refined compact/expanded layout sizing and spacing in `ui/main.slint` for more predictable rendering.
- Stabilized overlay centering to use target mode width during compact/expanded toggles.
- Fully overhauled UI and windowing to obey `docs/UI.md` strict motion and architecture rules:
  - Fixed root `Window` to `480px x 410px` to eliminate Win32 window resizing stutter.
  - Eliminated conditional `if` state recreation; both compact and expanded layers remain mounted inside a single morphing `island` container with opacity and vertical slide.
  - Implemented Nusa-style asymmetric spring easing: `cubic-bezier(0.34, 1.45, 0.64, 1.0)` for energetic expand with settle overshoot, and `cubic-bezier(0.22, 1.0, 0.36, 1.0)` for snappy retract.
  - Bound ear flare `Path` elements inside a unified `island-wrapper`, moving and scaling in 100% synchronous 0ms lockstep with the body.
  - Softened and restrained drop shadows across the UI (`drop-shadow-color: #00000040`, blur 6px - 14px) to prevent excessive dark smudging or heavy halos on desktop wallpapers.
  - Implemented composite hover detection in `ui/main.slint` (`is-mouse-inside`) covering all buttons, sliders, and the background, with an instant exit response in `src/main.rs`.
  - Intercepted Win32 `WM_NCHITTEST` on HWND to return `HTTRANSPARENT` for out-of-island coordinates, dynamically adapting to active island width (184px, dynamic lyric width, or 384px).
  - Integrated official **Lucide Icons** vector assets (`play.svg`, `pause.svg`, `skip-back.svg`, `skip-forward.svg`, `music.svg`, `pin.svg`, `chevron-up.svg`, `lyrics.svg`, `zap.svg`, `volume-2.svg`, `volume-1.svg`, `volume-x.svg`) in `assets/icons/`.
  - Implemented 100% real-time media playback timeline tracking:
    - Fixed static timestamp freezing: Windows GSMTC only reports the snapshot position at `LastUpdatedTime`. Added 100ns tick delta extrapolation via `GetSystemTimeAsFileTime`.
    - Polling interval set to 250ms (4Hz) when media is active, making the second counter and scrubber move in continuous real time.
  - Pixel-perfect layout alignment in Row 4:
    - Wrapped speaker icon, volume slider, and percent text in individual `VerticalLayout { alignment: center }` so they share the exact horizontal center line.
    - Wrapped Prev, Play (36px), and Next in `VerticalLayout { alignment: center }` so their vertical centers align with zero sagging.
  - Relocated live equalizer onto the album art as an overlay badge, giving the song title and artist full horizontal breathing room without awkward clipping or ellipsis dots.
  - **Dynamic Audio Spectrum Oscillation**:
    - Converted `EqualizerMini` bars to multi-frequency mathematical harmonics (Bass punch, Low-mid rhythm, High-mid melody, Treble shimmer) driven by an 80ms timer in Rust with 90ms ease-in-out GPU interpolation.
    - Bars dance dynamically and organically during playback across all views (compact ambient, compact lyrics, and expanded album badge), and gracefully rest at 3px when playback is paused.
  - **Dynamic Bouncy Lyric Island Width**:
    - Automatically calculates the island width from the active lyric line's character length: `(char_count * 8.0 + 104.0).clamp(210.0, 420.0)`.
    - Applied bouncy spring overshoot curve `cubic-bezier(0.34, 1.45, 0.64, 1.0)` to all compact width transitions. As each new lyric line appears, the island morphs and bounces smoothly into the new width without clipping text.
    - Synchronized `CURRENT_LYRIC_WIDTH` atomic with Win32 `WM_NCHITTEST` to preserve desktop transparency and click-through accuracy.
