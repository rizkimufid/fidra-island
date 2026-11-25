# ADR-0002: Isolate WinRT GSMTC & HTTP Lyrics on Dedicated Worker Threads

- **Status:** Accepted
- **Date:** 2026-11-25

## Context

Windows GSMTC WinRT APIs (`TryGetMediaPropertiesAsync`, `OpenReadAsync`) and remote HTTP calls to LRCLIB can block or stall for seconds. Executing WinRT `.get()` on Slint's main UI thread deadlocks with the Windows COM message pump, causing "Not Responding" freezes.

## Decision

Run two dedicated background worker threads communicating via `std::sync::mpsc` channels and `Weak::upgrade_in_event_loop`:
1. **Media Worker Thread:** Polls GSMTC (`250ms` when playing, `1000ms` when paused), extrapolates timeline position, emits incremental `ThumbnailUpdate` deltas, and executes non-blocking WinRT transport calls (`TryTogglePlayPauseAsync`, etc.).
2. **Lyric Fetcher Thread:** Handles all LRCLIB network I/O independently so slow network responses never delay playback progress or play/pause responsiveness.

## Consequences

### Positive
- Slint UI thread remains 100% non-blocking for hover, morphing, and click interactions.
- Eliminates COM message-pump deadlocks and UI freezes.
- Clean separation allows graceful shutdown via `Arc<AtomicBool>`.

### Trade-offs & Risks
- State synchronization requires careful ownership boundaries (`mpsc`, `Arc<Mutex<LyricContext>>`, atomics).
