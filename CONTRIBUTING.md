# Contributing to Fidra Island

Thank you for contributing to **Fidra Island**! Please read [`AGENTS.md`](AGENTS.md) and [`docs/README.md`](docs/README.md) before opening a pull request so your changes align with the project's architecture and quality bar.

## Development Setup

1. Install **Rust** (`1.75+`) via [rustup](https://rustup.rs/) with the `x86_64-pc-windows-msvc` toolchain.
2. Install Visual Studio Build Tools with the **Desktop development with C++** workload.
3. Clone the repository and verify your environment:

```powershell
cargo test
cargo clippy --all-targets -- -D warnings
cargo run
```

See [`docs/guides/development.md`](docs/guides/development.md) for debugging and profiling tips.

## Coding Standards

- Format all Rust code with `cargo fmt`.
- Zero Clippy warnings (`cargo clippy --all-targets -- -D warnings`).
- Add unit tests for all pure logic (parsers, formatters, sanitizers, math/geometry helpers).
- Document public modules, types, and functions with `rustdoc` comments (`//!` and `///`).
- Never suppress compiler/linter warnings with `#[allow(dead_code)]` unless strictly justified and commented.

## Commit Guidelines

Use clear, imperative commit messages (Conventional Commits style recommended):

- `feat(lyrics): support LRC offset tag parsing`
- `fix(window): scale WM_NCHITTEST coordinates by DPI`
- `perf(media): transmit album artwork as incremental delta`
- `docs(adr): record Win32 hit-testing architecture`

## Pull Request Checklist

- [ ] `cargo fmt --check` passes
- [ ] `cargo clippy --all-targets -- -D warnings` passes
- [ ] `cargo test` passes (with new tests for new/fixed behavior)
- [ ] `cargo doc --no-deps` passes without warnings
- [ ] Relevant files in `docs/` and `CHANGELOG.md` are updated
- [ ] Tested visually on Windows at `100%` and `>100%` DPI scaling (for UI/windowing changes)
