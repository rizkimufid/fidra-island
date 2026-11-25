# ADR-0006: Persistent Lyrics Disk Cache with TTL and Release Distribution Pipeline

- **Status:** Accepted
- **Date:** 2026-11-25

## Context

Phase 4 targets production polish, offline resilience, and automated release packaging:
1. Re-fetching lyrics on every application restart or playback loop makes redundant network calls to LRCLIB and fails when offline.
2. Unused icon assets left over from early mockups (`mic.svg`, `volume-1.svg`, `volume-2.svg`, `volume-x.svg`) clutter the asset tree and violate the project's zero-dead-code standard.
3. Users need easy distribution options: a zero-install portable ZIP and a lightweight, standard Windows Setup installer with desktop shortcut and optional sign-in startup.

## Decision

1. **Persistent Disk Cache for Lyrics (`src/lyrics.rs`):**
   - Cache directory: `%LOCALAPPDATA%\FidraIsland\cache\lyrics\`.
   - File naming: Alphanumeric track slug combined with deterministic 64-bit FNV-1a hash (`<slug>_<hash>.json`) to prevent filesystem path collisions and invalid character errors.
   - TTL Policy:
     - Positive cache (`Some(LyricsData)`): 14 days (1,209,600 seconds).
     - Negative cache (`None` for untracked/instrumental songs): 2 days (172,800 seconds), preventing repeated 404 lookups.
   - Atomic writes (`.tmp` write followed by rename).
2. **Asset Tree Cleanup:**
   - Removed unused mockup SVGs (`mic.svg`, `volume-1.svg`, `volume-2.svg`, `volume-x.svg`) since volume is handled through OS hardware keys and tray.
3. **Distribution & Packaging (`packaging/installer.iss`, `.github/workflows/release.yml`):**
   - Authored an Inno Setup 6 script generating `FidraIsland-Setup-x64.exe` with user-level privilege support (`PrivilegesRequired=lowest`).
   - Created a GitHub Actions release workflow triggered on `v*` tags to produce both portable `.zip` and Inno Setup installer executable alongside `SHA256SUMS.txt`.

## Consequences

### Positive
- Instant offline lyrics rendering for previously played tracks.
- Significant reduction in network latency and external LRCLIB load.
- Single-click Windows installer with clean uninstallation and shortcut management.
- Zero dead assets in binary.

### Trade-offs & Risks
- Cached lyrics files persist until the TTL expires or the user manually clears the `%LOCALAPPDATA%\FidraIsland\cache\` folder.
