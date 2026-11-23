# Security policy

## Reporting a vulnerability

Please don't open a public issue. Report it privately through
[GitHub's vulnerability reporting](https://github.com/Schongli/Qistara/security/advisories/new) (the **Security** tab › **Report a vulnerability**).

Include what an attacker could do, the steps to reproduce it and the version you tested. You'll get a reply within a
week, and credit in the release notes once it's fixed, if you'd like.

## Supported versions

Only the latest release gets security fixes.

## What's in scope

Qistara handles files, credentials and network data, so these matter most:

- reading or leaking secrets kept in the OS secret store (Last.fm keys, server password, provider tokens),
- personal data sent to a service that doesn't need it,
- a crafted media file, tag, lyrics file, playlist or server reply that runs code, damages files or escapes the
  checks in `mpv.rs`, `ytdlp.rs` or `tags.rs`,
- the yt-dlp, ffmpeg or Deno download skipping its SHA-256 check, the Japanese dictionary its md5 check, or a
  translation language pack its SHA-256 check or Mozilla's fixed storage,
- the OAuth loopback sign-in pages (Spotify, Apple Music, Tidal) answering anyone but the app,
- the listen-together server (`session.rs`): reading a file that wasn't shared, sharing a path or a server token,
  joining without the invite's key, or a guest's loopback proxy answering anything but this app,
- the updater installing anything that isn't signed with the release key for the version it claims,
- a YouTube sign-in's cookies outliving the run that needed them.

Bugs in mpv, FFmpeg, yt-dlp or WebView2 themselves belong to those projects, but tell us if Qistara needs to update one.
