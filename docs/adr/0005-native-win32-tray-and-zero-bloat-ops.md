# ADR-0005: Native Win32 System Tray, Atomic Config, and Zero-Bloat File Logging

- **Status:** Accepted
- **Date:** 2026-11-25

## Context

Phase 3 production hardening requires a System Tray icon with a Quit/Settings menu, Windows sign-in auto-start, persistent user configuration, and rotating file logs for diagnosing issues in a `windows_subsystem = "windows"` binary. Pulling in multiple third-party crates for tray, logging, and registry access would increase compile times, dependency surface area, and binary size.

## Decision

1. **System Tray & Auto-Start:** Implement directly on top of the existing `windows-sys` 0.59 dependency (`Win32_UI_Shell`, `Win32_System_Registry`, `Win32_System_Com`) inside `src/platform/windows.rs`, with no-op stubs in `src/platform/stub.rs`.
2. **Persistent Configuration:** Store typed `AppConfig` as atomic JSON (`write .tmp + rename`) in `%APPDATA%\FidraIsland\config.json` using the existing `serde` + `serde_json` dependencies.
3. **Structured File Logging:** Implement a lightweight thread-safe rotating file logger (`src/logging.rs`, `2 MiB` rotation threshold) writing UTC ISO-8601 records to `%LOCALAPPDATA%\FidraIsland\logs\fidra-island.log`.
4. **COM Initialization:** Wrap `CoInitializeEx(COINIT_MULTITHREADED)` / `CoUninitialize` in an RAII `ComGuard` on the media worker thread.

## Consequences

### Positive
- Zero new external crate dependencies added to `Cargo.lock`.
- Minimal binary size and fast offline builds.
- Clean separation between platform Win32 FFI (`src/platform/`) and application orchestration (`src/app.rs`).

### Trade-offs & Risks
- Win32 tray menu uses the default application icon (`IDI_APPLICATION`) until a custom embedded `.ico` resource is bundled in Phase 4.
- Log formatting is intentionally simple plain-text structured lines rather than full `tracing` span trees.
