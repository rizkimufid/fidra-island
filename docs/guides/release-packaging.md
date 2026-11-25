# Release & Packaging Guide

## Building an Optimized Release Binary

`Cargo.toml` defines a size- and speed-optimized `[profile.release]` (`opt-level = "s"`, `lto = "thin"`, `codegen-units = 1`, `strip = true`).

```powershell
# Run full pre-release verification gate
cargo fmt --check
cargo clippy --all-targets -- -D warnings
cargo test --all-targets
cargo doc --no-deps

# Build release executable
cargo build --release
```

Output binary: `target/release/fidra-island.exe`.

> [!NOTE]
> All Lucide SVG assets (`assets/icons/*.svg`) and Slint UI definitions (`ui/main.slint`) are compiled directly into the executable. The release `.exe` is fully self-contained and portable with zero external asset folder requirements.

## Packaging Targets

### 1. Portable ZIP Package
The release binary `fidra-island.exe` is completely self-contained. Packaging involves archiving `fidra-island.exe`, `README.md`, and `CHANGELOG.md` into a portable `.zip`.

### 2. Windows Setup Installer (Inno Setup)
An Inno Setup script is provided at [`packaging/installer.iss`](../../packaging/installer.iss). It produces `FidraIsland-Setup-x64.exe` with:
- Per-user installation without requiring UAC administrator elevation (`PrivilegesRequired=lowest`).
- Desktop and Start Menu shortcut creation.
- Optional Windows sign-in auto-start registration (`HKCU\...\Run`).
- Clean uninstallation via Windows Settings / Control Panel.

Compile locally with Inno Setup 6:
```powershell
iscc packaging\installer.iss
```
Output installer: `dist\FidraIsland-Setup-x64.exe`.

### 3. Automated GitHub Release Workflow
Tagging a commit with `v*` (e.g., `git tag v0.2.0 && git push origin v0.2.0`) automatically triggers [`.github/workflows/release.yml`](../../.github/workflows/release.yml), which:
1. Compiles the optimized release binary on Windows.
2. Packages `fidra-island-portable-windows-x64.zip`.
3. Compiles `FidraIsland-Setup-x64.exe`.
4. Computes SHA-256 hashes (`SHA256SUMS.txt`).
5. Publishes a new GitHub Release with all three artifacts attached.

## Release Checklist

1. Bump `version` in `Cargo.toml` following Semantic Versioning (`MAJOR.MINOR.PATCH`).
2. Move items from `[Unreleased]` to `[X.Y.Z] - YYYY-MM-DD` in `CHANGELOG.md`.
3. Verify `Cargo.lock` is updated and committed.
4. Run the verification gate above on a clean Windows machine.
5. Smoke-test `target/release/fidra-island.exe` at `100%` and `150%` DPI scaling with an active media player.
6. Tag the commit (`git tag vX.Y.Z`) and publish the portable `.exe` / `.zip` artifact.

## Troubleshooting

| Symptom | Likely Cause | Resolution |
|---|---|---|
| Island does not detect media | Media player does not expose Windows GSMTC | Verify Windows volume flyout shows track metadata; enable hardware media keys in browser/player settings. |
| Lyrics show "No Synced Lyrics" | Track not found on LRCLIB or offline | Check internet access to `https://lrclib.net`; obscure/remixed tracks may not have synced LRC entries. |
| How to close or hide the app | Overlay uses `WS_EX_TOOLWINDOW` (no taskbar icon) | Right-click the **Fidra Island** System Tray icon and select **Hide Fidra Island** or **Quit Fidra Island** (or double-click the tray icon to toggle visibility). |
