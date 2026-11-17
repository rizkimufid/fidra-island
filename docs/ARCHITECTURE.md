# Architecture

## Scope boundary

This project owns only the music player/island prototype. It does not own the complete Fira application or non-music features.

## Initial layers

```text
Slint UI (Main Thread - Strict UI.md Architecture)
  -> fixed 480x410 transparent bounding viewport (zero OS window resize stutter)
  -> unified notch & wing wrapper (`island-wrapper`) with synchronized `current-island-width`
  -> left & right ear-flare Path elements locked to wrapper coordinate space (0ms lockstep)
  -> single unified morphing container (`island`) with `clip: true` and restrained elevation shadows
  -> persistent dual-layer mounting (no conditional `if` destruction)
  -> Nusa-style asymmetric spring easing (expand: cubic-bezier(0.34, 1.45, 0.64, 1.0), collapse: cubic-bezier(0.22, 1.0, 0.36, 1.0))
  -> staggered 3-tier entrance cascade (header 80ms, hero 120ms, volume 160ms)
  -> Lucide Icon vector system: discrete official SVG assets in `assets/icons/*.svg` compiled into binary
  -> tactile micro-interactions (press shrink -3px, hover pill expand +20px, hover track thickening 5px->8px)
  -> composite hover detection (`is-mouse-inside`) tracking all child elements & background

Rust Application & Windowing Layer
  -> top-edge screen overlay positioning (y=0, auto-centered at 480px width)
  -> Win32 dynamic hit-testing intercepting WM_NCHITTEST with HTTRANSPARENT for transparent bounds
  -> snappy responsive collapse lifecycle coordination (instant 40ms buffer on exit) and pin lock support
  -> event-driven "Doze Off" architecture (1000ms idle poll when paused, 300ms when playing)
  -> thread-safe state synchronization to Slint via upgrade_in_event_loop

Music Integration Boundary (Background Dedicated Worker Thread)
  -> non-blocking Windows GSMTC transport controls (TryTogglePlayPauseAsync, etc.)
  -> Windows GSMTC current-session metadata and timeline polling
  -> live media thumbnail streaming via WinRT IRandomAccessStreamReference and DataReader
  -> audio backend and operating-system media controls via mpsc channel
```

## Validation

The architecture remains provisional until a prototype validates Slint state binding, window behavior, rendering responsiveness, and playback integration.
