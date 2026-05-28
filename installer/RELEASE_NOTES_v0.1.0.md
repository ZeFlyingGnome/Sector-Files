# Controller Pack Installer v0.1.0 — first Tauri release

This is the first release of the new Rust + Tauri controller pack installer, replacing the previous Python `ControllerPackInstaller.exe` and `ProfileConfigurator.exe`.

## What's new

- **One binary instead of two.** Install/update + profile configuration are in a single app.
- **Persistent profile.** Your controller pack directory, CID, password, rating, and preferences are remembered between launches.
- **Direct GNG downloads.** Sign in to `files.aero-nav.com` once via the embedded browser; AIRAC packages download in the background after that.
- **AIRAC backup.** Old sector files are moved to `LFXX/Sectors/Backup/<FIR>-<cycle>.<ext>` automatically when a new cycle arrives.
- **Update detection.** A passive badge tells you when a new GitHub revision or AIRAC cycle is available.
- **Signed auto-updater.** Future installer versions install themselves with one click; no more batch-script self-update.

## One-time cutover from the old Python installer

If you're running the old `ControllerPackInstaller.exe`:

1. Download **`Controller Pack Installer_0.1.0_x64-setup.exe`** from this release.
2. Run it. It installs alongside (or replaces) the old binary.
3. On first launch, point it at your existing controller pack directory; it will detect your installed version and pick up where the old installer left off.
4. After this one-time download, all future updates install themselves automatically.

You can safely delete the old `ControllerPackInstaller.exe` and `ProfileConfigurator.exe` once the new app is running.

## Known limitations

- **GNG cookie capture and download URLs are subject to verification.** The first user to sign in is asked to confirm the session is captured correctly; if not, please open an issue with the WebView devtools output.
- **Windows code-signing is not yet applied.** SmartScreen will warn on first install. Click "More info" → "Run anyway". A code-signing cert is being investigated.
- **macOS and Linux builds are unsigned.** Windows is the primary supported target.

## For contributors

Source for the new app lives in [`installer/`](../installer). Pure-Rust core in [`installer/core/`](../installer/core); Tauri shell + commands in [`installer/src-tauri/`](../installer/src-tauri); React frontend in [`installer/src/`](../installer/src). Build instructions: `bun install && bun run tauri dev` from `installer/`.
