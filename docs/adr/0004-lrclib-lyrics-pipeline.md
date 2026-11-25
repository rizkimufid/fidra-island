# ADR-0004: Asynchronous LRCLIB Pipeline with Deduplication & In-Memory Cache

- **Status:** Accepted
- **Date:** 2026-11-25

## Context

Synced lyrics require querying an external API (`https://lrclib.net`). Raw GSMTC track metadata often contains noisy suffixes (`(Official Video)`, `[Remastered 2023]`, `(feat. ...)`) that reduce exact-match rates, and rapid track skipping can queue redundant HTTP requests.

## Decision

1. Sanitize track titles and artist names using pre-compiled `OnceLock<Regex>` patterns.
2. Share a single pooled `OnceLock<ureq::Agent>` with a `5s` timeout and descriptive `User-Agent` header.
3. Drain pending requests in the lyric worker channel before fetching so rapid track skips only query the latest song, and store results in a bounded `64`-entry in-memory cache.
4. Query `/api/get` first (exact match + duration), falling back to `/api/search` with `±5s` duration preference.
5. Parse standard LRC (`[mm:ss.xx]`), `[offset:+/-ms]`, and enhanced word-synced LRC (`<mm:ss.xx>`), stripping inline word tags from `LyricLine.text`.

## Consequences

### Positive
- Instant replay/skip-back via cache without redundant API calls.
- High match accuracy across YouTube Music, Spotify, and browser media players.
- Clean display text even when enhanced word-synced LRC is returned.

### Trade-offs & Risks
- Cache is volatile (cleared on app restart); persistent disk caching with TTL is planned for Phase 4.
