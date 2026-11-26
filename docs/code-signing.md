# Signing and releases

A release has two signatures, and they do different jobs:

| Signature | What it's for | Key held by | Until it's set up |
|---|---|---|---|
| **Authenticode** (SignPath Foundation) | Windows shows "Qistara" as a verified publisher instead of "Windows protected your PC" | SignPath, on its own hardware | the installer is unsigned and SmartScreen warns |
| **Updater** (minisign) | Settings › General › Updates installs only what you signed, for the version it says | you, as a GitHub secret | "This build can't update itself" |
| **Apple** (Developer ID, notarized) | macOS opens Qistara without "Open Anyway" | you, as GitHub secrets ($99/year Apple account) | ad-hoc signed: users choose Open Anyway once ([below](#5-apple-signing-later-optional)) |

`.github/workflows/release.yml` does both when you push a tag. Each part switches on once its secrets are in place, so
you can release before either is set up.

**Order that works:** updater key (1), Discord (2), first release (3), then SignPath (4), which wants a release to
exist already. Commands are given for PowerShell (VS Code's terminal) first, then Command Prompt where they differ.

## 1. Updater key (once, about 5 minutes)

Run these **inside your Qistara folder** (the one with `package.json`, for example `D:\Qistara`). `npx tauri` finds
the Tauri CLI in the project's `node_modules`. Run anywhere else, npx looks for an npm package named `tauri` instead
and fails with "could not determine executable to run".

1. Open a terminal in your Qistara folder:
   - **VS Code:** with the Qistara folder open, choose **Terminal › New Terminal**. It already starts there: the
     prompt reads `PS D:\Qistara>` (your own folder's path).
   - **Command Prompt:** go to the folder with `cd /d` and your folder's path. `/d` also switches drives; typing a
     path on its own isn't a command.

     ```bash
     cd /d D:\Qistara
     ```

   If there's no `node_modules` folder yet, install the project's packages once:

   ```bash
   npm install
   ```

2. Make a folder for the key, outside the project (if it already exists, that's fine):

   ```bash
   mkdir "$env:USERPROFILE\.tauri"
   ```

   In Command Prompt: `mkdir "%USERPROFILE%\.tauri"`.

3. Generate the key pair. It asks for a password, then again to confirm, and **nothing shows while you type**. Choose
   a strong one and keep it in a password manager.

   ```bash
   npx tauri signer generate -w "$env:USERPROFILE\.tauri\qistara.key"
   ```

   In Command Prompt the path is `"%USERPROFILE%\.tauri\qistara.key"`. From outside the project, this works too
   (the same CLI version the project uses):

   ```bash
   npx @tauri-apps/cli@2.12.4 signer generate -w "$env:USERPROFILE\.tauri\qistara.key"
   ```

   This makes two files:
   - `qistara.key`: the **private** key. Never commit it, paste it in a chat or send it to anyone.
   - `qistara.key.pub`: the public key. It isn't secret.

4. Put the public key in the app. Open it:

   ```bash
   notepad "$env:USERPROFILE\.tauri\qistara.key.pub"
   ```

   (Command Prompt: `notepad "%USERPROFILE%\.tauri\qistara.key.pub"`.) Copy the whole line (it starts with
   `dW50cnVzdGVk`) and paste it between the quotes of `"pubkey": ""` in `src-tauri\tauri.conf.json`. Commit and
   push that change.

5. Give the release workflow the private key. On GitHub, open the repository's **Settings › Secrets and variables ›
   Actions** and choose **New repository secret** twice:
   - Name `TAURI_SIGNING_PRIVATE_KEY`. For the value, open the key with
     `notepad "$env:USERPROFILE\.tauri\qistara.key"` (Command Prompt: `notepad "%USERPROFILE%\.tauri\qistara.key"`),
     select all and copy.
   - Name `TAURI_SIGNING_PRIVATE_KEY_PASSWORD`, with the password from step 3.

6. Back up `qistara.key` and its password, for example in a password manager or on a USB stick.
   - If they're lost, installed copies can't verify updates any more and have to be reinstalled by hand.
   - If they leak, anyone could sign an update. Generate a new pair, ship it, and ask users to reinstall.

The updater only accepts a file whose signature is bound to the version `latest.json` announces
(`requireSignedVersion`), so an edited `latest.json` can't point the app at an older release.

## 2. Discord (once, about 5 minutes)

1. Open the [Discord Developer Portal](https://discord.com/developers/applications), choose **New Application**,
   name it `Qistara`, accept the terms and create it.
2. On **General Information**, copy the **Application ID** (a long number). It isn't secret.
3. Under **Rich Presence › Art Assets**, choose **Add Image(s)** and upload `src-tauri\icons\icon.png` (512 × 512).
   Name it exactly `qistara`, then **Save Changes**. A new asset can take a few minutes to show.
4. On GitHub, open **Settings › Secrets and variables › Actions**, then the **Variables** tab (not Secrets), and choose
   **New repository variable**. Name it `QISTARA_DISCORD_ID`, with the Application ID as the value. Release builds now
   include it.
5. To try it in your own build, set it in the same window before starting the app (it's read when Rust compiles):

   ```bash
   $env:QISTARA_DISCORD_ID="your_application_id"
   ```

   ```bash
   npm run tauri dev
   ```

   In Command Prompt: `set QISTARA_DISCORD_ID=your_application_id`. Then turn on **Settings › Integrations ›
   Discord › Show on Discord**, with the Discord desktop app open and Discord's own activity sharing turned on
   (Discord's **User Settings › Activity Privacy**).

## 3. Releasing a version

1. Set the same version in `package.json`, `src-tauri/Cargo.toml` and `src-tauri/tauri.conf.json`, and add a
   `## X.Y.Z (date)` section to `CHANGELOG.md`. That section becomes the release notes, in the app too.
2. Merge into `main` and check that CI passes on the repository's **Actions** tab.
3. Tag the commit and push the tag:

   ```bash
   git checkout main
   ```

   ```bash
   git pull
   ```

   ```bash
   git tag v0.4.0
   ```

   ```bash
   git push origin v0.4.0
   ```

4. **Actions › Release** checks the tag matches `tauri.conf.json`, then builds three things side by side:
   - **Windows:** the installer; SignPath signs it, when set up (approve the request in SignPath while it waits)
   - **macOS (Apple Silicon):** the `.dmg`, and `Qistara.app.tar.gz`, which is what the updater installs
   - **Linux:** the `.deb` and the `.AppImage`

   Each is signed for the updater. The macOS and Linux packages are then started on a clean machine
   (`scripts/smoke.sh`), and must find mpv. Only when everything has passed does it publish one release with
   `latest.json`.

Copies of a version update themselves from the next release on: 0.4.0 offers 0.4.1 when it's out.

**A test release (beta)** for people trying a version first: give it a version like `0.6.0-beta.1` (in the same
three files) and tag `v0.6.0-beta.1`. The workflow builds every package and publishes a GitHub **pre-release**; its
Windows installer isn't sent to SignPath, so Windows shows the "unknown publisher" prompt for it. The app only reads
the latest full release, so nobody is offered a beta as an update. Its release notes come from the `## 0.6.0` section
of the changelog.

## 4. SignPath Foundation (once; their review takes a while)

[SignPath Foundation](https://signpath.org) signs open-source Windows software for free, with its own certificate.
What follows comes from their published terms and from other projects' guides. Check the application page for the
current rules before applying.

1. **Before applying**, the project needs:
   - a public repository and an OSI licence: Qistara is MIT, and has no proprietary parts
   - fully automated builds: `release.yml` on GitHub Actions
   - at least one earlier release (step 3)
   - two-factor authentication on GitHub, and later on SignPath, for everyone with release rights
   - a code signing policy: the [section below](#code-signing-policy)
2. **Apply** on signpath.org with the repository URL and a link to this page. They review the project by hand and may
   ask for changes.
3. **Once accepted,** in SignPath:
   - note the **organization ID**
   - create a **project** for this repository, which gives a project slug
   - keep the artifact configuration for a single `.exe`
   - add a **signing policy**, for example `release-signing`
   - link GitHub as the trusted build system
   - create an **API token** for CI
4. **On GitHub**, under **Settings › Secrets and variables › Actions**:
   - secret `SIGNPATH_API_TOKEN`
   - variables `SIGNPATH_ORGANIZATION_ID`, `SIGNPATH_PROJECT_SLUG` and `SIGNPATH_POLICY_SLUG`
5. **Each release:** open-source signing requests are approved by hand in SignPath. The workflow waits for it.

The signing step runs only when `SIGNPATH_ORGANIZATION_ID` is set. It signs the installer. The `Qistara.exe` inside it
isn't Authenticode-signed yet: that would take a `bundle.windows.signCommand` that sends each file to SignPath during
the build.

## 5. Apple signing (later, optional)

Without it, the macOS app is ad-hoc signed: it runs, but macOS asks people to choose **Open Anyway** once. To remove
that, you need an [Apple Developer](https://developer.apple.com/programs/) account ($99/year). Then:

1. In Xcode or on developer.apple.com, create a **Developer ID Application** certificate and export it as a `.p12`
   with a password.
2. Create an app-specific password for your Apple ID at [account.apple.com](https://account.apple.com).
3. Add GitHub secrets: `APPLE_CERTIFICATE` (the `.p12`, base64), `APPLE_CERTIFICATE_PASSWORD`,
   `APPLE_SIGNING_IDENTITY` (`Developer ID Application: Your Name (TEAMID)`), `APPLE_ID`, `APPLE_PASSWORD` (the
   app-specific one) and `APPLE_TEAM_ID`.
4. Ask for the release workflow to be switched over. That means:
   - passing those secrets to the macOS build, which then signs and notarizes;
   - dropping `signingIdentity: "-"` in `tauri.macos.conf.json`;
   - turning the hardened runtime back on, with the `com.apple.security.cs.disable-library-validation` entitlement:
     Qistara loads Homebrew's libmpv, which Apple didn't sign;
   - signing `src-tauri/lib/*.dylib` with the same identity before the build.

## Code signing policy

Free code signing provided by [SignPath.io](https://about.signpath.io), certificate by
[SignPath Foundation](https://signpath.org).

- **What's signed:** only the Windows installer built by `.github/workflows/release.yml` from a version tag in this
  repository. Nothing built on a personal computer is signed.
- **Roles:** the maintainer, [Schongli](https://github.com/Schongli), is the author, reviews every change and
  approves every signing request. Outside contributions arrive as pull requests and are reviewed before they're merged.
- **Privacy:** Qistara has no account, no telemetry and no servers of its own. It contacts only the services listed
  in the README's [Privacy](../README.md#privacy) section, and only for the features that need them.

## Sources

- [SignPath Foundation terms](https://signpath.org/terms)
- [SignPath for open source](https://about.signpath.io/product/open-source)
- [SignPath origin verification](https://docs.signpath.io/origin-verification)
