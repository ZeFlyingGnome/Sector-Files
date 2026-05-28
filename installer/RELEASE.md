# Releasing the Controller Pack Installer

This document is for **maintainers** who cut signed Tauri releases. End users do not need to read it.

## One-time setup

### 1. Generate a Tauri signing keypair

```bash
bun x @tauri-apps/cli signer generate -w ~/.tauri/cofrance-installer.key
```

You'll be prompted for a passphrase — **use one** and store it in your password manager. The command emits:

- `~/.tauri/cofrance-installer.key` — the **private** key. Never commit this. Never email it. Never upload it anywhere except the GitHub repo's Actions secrets (next step).
- `~/.tauri/cofrance-installer.key.pub` — the **public** key. Embed this in `installer/src-tauri/tauri.conf.json` under `plugins.updater.pubkey`.

### 2. Put the private key in GitHub Actions secrets

Repo → Settings → Secrets and variables → Actions → New repository secret:

- `TAURI_SIGNING_PRIVATE_KEY` — paste the **entire** content of `~/.tauri/cofrance-installer.key` (the file, not the path).
- `TAURI_SIGNING_PRIVATE_KEY_PASSWORD` — the passphrase you chose.

### 3. Commit the public key

Edit `installer/src-tauri/tauri.conf.json`:

```json
"plugins": {
  "updater": {
    "pubkey": "<paste the content of ~/.tauri/cofrance-installer.key.pub here>"
  }
}
```

Commit and push. Every subsequent build will be signed by the matching private key and verified against the public key embedded in the binary.

> ⚠️ **If you ever lose the private key, every existing installation is permanently orphaned** — they will refuse any update because they only trust manifests signed by that exact key. Back the key up in *at least* two places (e.g. password manager + offline encrypted USB).

## Cutting a release

1. Bump the version in `installer/src-tauri/Cargo.toml` and `installer/src-tauri/tauri.conf.json` (keep them in sync). Update `installer/package.json` to match.
2. Commit: `git commit -m "installer: bump to v0.2.0"`.
3. Tag with the `installer-v` prefix: `git tag installer-v0.2.0 && git push origin installer-v0.2.0`.
4. Create a GitHub Release for that tag. The `.github/workflows/build-installer-tauri.yml` workflow will build Windows + Linux artifacts, sign them, generate `latest.json`, and attach everything to the release.
5. Verify the release contains: `Controller Pack Installer_<version>_x64-setup.exe`, `Controller Pack Installer_<version>_x64-setup.exe.sig`, `latest.json`.

End users with a previous Tauri installer (>= v0.1.0) will see the new version offered the next time they launch the app.

## Rolling back

If a release is broken in the wild:

1. Cut a follow-up release with a higher version that reverts the problematic change.
2. Do NOT delete the broken release — `latest.json` is updated on every release, so the broken version is naturally superseded. Deleting it would only matter for users intentionally pinning, which we don't support.

## Initial cutover from the legacy Python installer

The legacy Python `ControllerPackInstaller.exe` self-updated by hitting the same GitHub Releases stream. The first Tauri release (`installer-v0.1.0`) should be accompanied by a one-time message pointing existing Python users at the new download URL — see `openspec/changes/rust-tauri-installer/design.md` §"Migration Plan".
