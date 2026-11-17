# Signing and releases

A release has two signatures, and they do different jobs:

| Signature | What it's for | Key held by | Until it's set up |
|---|---|---|---|
| **Authenticode** (SignPath Foundation) | Windows shows "Qistara" as a verified publisher instead of "Windows protected your PC" | SignPath, on its own hardware | the installer is unsigned and SmartScreen warns |
| **Updater** (minisign) | Settings › General › Updates installs only what you signed, for the version it says | you, as a GitHub secret | "This build can't update itself" |

`.github/workflows/release.yml` does both when you push a tag. Each part switches on once its secrets are in place, so
you can release before either is set up.

## Releasing a version

1. Set the same version in `package.json`, `src-tauri/Cargo.toml` and `src-tauri/tauri.conf.json`, and add a
   `## X.Y.Z (date)` section to `CHANGELOG.md`. That section becomes the release notes, in the app too.
2. Commit, then tag and push the tag:

   ```bash
   git tag v0.4.0
   ```

   ```bash
   git push origin v0.4.0
   ```

3. The workflow checks that the tag matches `tauri.conf.json`, runs the Rust tests and builds the installer. It then
   has SignPath sign the installer (when set up), signs the final file for the updater, writes `latest.json` and
   publishes the GitHub release.

## One-time setup: updater key

1. Generate a key pair on your own computer, with a password, and keep the private key out of the repository:

   ```bash
   npx tauri signer generate -w ~/.tauri/qistara.key
   ```

2. Put the **public** key (the contents of `~/.tauri/qistara.key.pub`) in `src-tauri/tauri.conf.json` ›
   `plugins` › `updater` › `pubkey`, and commit it. The public key isn't secret.
3. In the repository, under **Settings › Secrets and variables › Actions**, add two secrets:
   - `TAURI_SIGNING_PRIVATE_KEY`: the contents of `~/.tauri/qistara.key`
   - `TAURI_SIGNING_PRIVATE_KEY_PASSWORD`: its password
4. Keep a backup of the private key. If it's lost, installed copies can't verify updates any more and have to be
   reinstalled by hand. If it leaks, anyone could sign an update, so generate a new pair, ship it, and tell users to
   reinstall.

The updater only accepts a file whose signature is bound to the version `latest.json` announces
(`requireSignedVersion`), so an edited `latest.json` can't point the app at an older release.

## One-time setup: SignPath Foundation

[SignPath Foundation](https://signpath.org) signs open-source Windows software for free, with its own certificate.
Qistara fits the usual conditions: an OSI licence (MIT), public source, and releases built on GitHub Actions from
that source rather than on someone's computer. Read the terms on their site before applying.

1. Apply on signpath.org with the repository URL. They review the project and may ask for changes, such as a
   code-signing policy page. This file can serve as the start of one.
2. Once accepted, in SignPath:
   - create a **project** for this repository
   - add an **artifact configuration** that signs the single `.exe` it's given
   - add a **signing policy** (for example `release-signing`)
   - link GitHub as the trusted build system
3. In the repository, under **Settings › Secrets and variables › Actions**:
   - secret `SIGNPATH_API_TOKEN`
   - variables `SIGNPATH_ORGANIZATION_ID`, `SIGNPATH_PROJECT_SLUG` and `SIGNPATH_POLICY_SLUG`

The signing step runs only when `SIGNPATH_ORGANIZATION_ID` is set. It signs the installer. The `Qistara.exe` inside it
isn't Authenticode-signed yet: that would take a `bundle.windows.signCommand` that sends each file to SignPath during
the build.

## Other release settings

- `QISTARA_DISCORD_ID` (variable): the Application ID of the "Qistara" app in the Discord Developer Portal, for
  Settings › Integrations › Discord. It isn't secret. Without it, the build says Discord isn't set up.
