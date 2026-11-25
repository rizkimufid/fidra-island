# Architecture Decision Records (ADRs)

This directory captures significant architectural decisions made for **Fidra Island** along with their context, rationale, and consequences.

## Index

| ID | Title | Status |
|---|---|---|
| [0000](0000-template.md) | ADR Template | Template |
| [0001](0001-rust-slint-stack.md) | Use Rust + Slint for Native Desktop Island UI | Accepted |
| [0002](0002-background-workers.md) | Isolate WinRT GSMTC & HTTP Lyrics on Dedicated Worker Threads | Accepted |
| [0003](0003-win32-hit-testing.md) | Fixed Transparent Viewport with DPI-Aware Win32 `WM_NCHITTEST` | Accepted |
| [0004](0004-lrclib-lyrics-pipeline.md) | Asynchronous LRCLIB Pipeline with Deduplication & In-Memory Cache | Accepted |
| [0005](0005-native-win32-tray-and-zero-bloat-ops.md) | Native Win32 System Tray, Atomic Config, and Zero-Bloat File Logging | Accepted |
| [0006](0006-disk-cache-and-distribution.md) | Persistent Lyrics Disk Cache with TTL and Release Distribution Pipeline | Accepted |
| [0007](0007-scalable-architecture-and-adaptive-ambience.md) | Scalable Multi-Widget Architecture, <500-Line Code Limit, and Adaptive Ambience | Accepted |
