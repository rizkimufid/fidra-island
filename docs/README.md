# Fidra Island Documentation

Welcome to the living documentation hub for **Fidra Island**. Every document in this folder is version-controlled alongside the codebase and governed by the sync rules in [`AGENTS.md`](../AGENTS.md).

## Architecture

- [System Architecture Overview](architecture/overview.md) — Threading model, message channels, GSMTC media pipeline, and LRCLIB lyrics flow.
- [Windows Platform Integration](architecture/windows-integration.md) — DPI scaling, `WM_NCHITTEST` desktop click-through, `WS_EX_TOOLWINDOW`, and WinRT GSMTC behavior.

## Design System

- [UI & Motion Specification](design/ui-motion-spec.md) — Single source of truth for dimensions, obsidian palette, typography, spring curves, and Slint component rules.

## Architecture Decision Records (ADRs)

- [ADR Index](adr/README.md) — Numbered records of significant technical decisions and trade-offs.

## Guides & Operations

- [Development Guide](guides/development.md) — Local environment, debugging Win32/Slint, unit testing, and profiling idle CPU/GPU.
- [Release & Packaging Guide](guides/release-packaging.md) — Optimized release builds, Windows distribution, and troubleshooting.
- [Production Roadmap](roadmap.md) — Prioritized milestones toward v1.0.

## API Reference (Generated)

Generate and open up-to-date `rustdoc` module documentation directly from source:

```powershell
cargo doc --no-deps --open
```
