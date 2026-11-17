# Signing and releases

A release has two signatures, and they do different jobs:

| Signature | What it's for | Key held by | Until it's set up |
|---|---|---|---|
| **Authenticode** (SignPath Foundation) | Windows shows "Qistara" as a verified publisher instead of "Windows protected your PC" | SignPath, on its own hardware | the installer is unsigned and SmartScreen warns |
| **Updater** (minisign) | Settings › General › Updates installs only what you signed, for the version it says | you, as a GitHub secret | "This build can't update itself" |

`.github/workflows/release.yml` does both when you push a tag. Each part switches on once its secrets are in place, so
you can release before either is set up.

**Order that works:** updater key (1), Discord (2), first release (3), then SignPath (4), which wants a release to
exist already. The commands below are for Windows Command Prompt; PowerShell variants are given where they differ.

## 1. Updater key (once, about 5 minutes)

Run these **inside the Qistara folder** (the one with `package.json`), after `npm install`. `npx tauri` finds the
Tauri CLI in the project's `node_modules`. Run anywhere else, npx looks for an npm package named `tauri` instead and
fails with "could not determine executable to run".

1. Open Command Prompt in the project and install its packages:

   ```bash
   cd C:\path\to\Qistara
   ```

   ```bash
   npm install
   ```

2. Make a folder for the key, outside the project:

   ```bash
   mkdir "%USERPROFILE%\.tauri"
   ```

3. Generate the key pair. It asks for a password twice, and nothing shows while you type. Choose a strong one and
   keep it in a password manager.

   ```bash
   npx tauri signer generate -w "%USERPROFILE%\.tauri\qistara.key"
   ```

   In PowerShell the path is `"$env:USERPROFILE\.tauri\qistara.key"`. From outside the project, this works too
   (the same CLI version the project uses):

   ```bash
   npx @tauri-apps/cli@2.12.4 signer generate -w "%USERPROFILE%\.tauri\qistara.key"
   ```

   This makes two files:
   - `qistara.key`: the **private** key. Never commit it, paste it in a chat or send it to anyone.
   - `qistara.key.pub`: the public key. It isn't secret.

4. Put the public key in the app. Open it:

   ```bash
   notepad "%USERPROFILE%\.tauri\qistara.key.pub"
   ```

   Copy the whole line (it starts with `dW50cnVzdGVk`) and paste it between the quotes of `"pubkey": ""` in
   `src-tauri\tauri.conf.json`. Commit and push that change.

5. Give the release workflow the private key. On GitHub, open the repository's **Settings › Secrets and variables ›
   Actions** and choose **New repository secret** twice:
   - Name `TAURI_SIGNING_PRIVATE_KEY`. For the value, open the key with
     `notepad "%USERPROFILE%\.tauri\qistara.key"`, select all and copy.
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
   set QISTARA_DISCORD_ID=your_application_id
   ```

   ```bash
   npm run tauri dev
   ```

   In PowerShell: `$env:QISTARA_DISCORD_ID="your_application_id"`. Then turn on **Settings › Integrations ›
   Discord › Show on Discord**, with the Discord desktop app open and Discord's own activity sharing turned on
   (Discord's **User Settings › Activity Privacy**).

## 3. Releasing a version

1. Set the same version in `package.json`, `src-tauri/Cargo.toml` and `src-tauri/tauri.conf.json`, and add a
   `## X.Y.Z (date)` section to `CHANGELOG.md`. That section becomes the release notes, in the app too.
   (0.4.0 is already set.)
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

4. **Actions › Release** builds the installer and checks the tag matches `tauri.conf.json`. It then:
   - has SignPath sign the installer, when set up (approve the request in SignPath while it waits)
   - signs the final file for the updater
   - publishes the release with `latest.json`

Copies of a version update themselves from the next release on: 0.4.0 offers 0.4.1 when it's out.

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
