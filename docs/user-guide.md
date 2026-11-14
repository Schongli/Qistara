# Qistara user guide

- [Getting started](#getting-started)
- [Library](#library)
- [Playing music](#playing-music)
- [Bit-perfect audio and the Signal Path](#bit-perfect-audio-and-the-signal-path)
- [Equalizer](#equalizer)
- [Lyrics](#lyrics)
- [Video](#video)
- [Playlists](#playlists)
- [Sources and accounts](#sources-and-accounts)
- [Downloads](#downloads)
- [Appearance](#appearance)
- [Keyboard shortcuts](#keyboard-shortcuts)
- [Where your data lives](#where-your-data-lives)
- [Troubleshooting](#troubleshooting)

## Getting started

1. Download `Qistara_x.y.z_x64-setup.exe` from [Releases](https://github.com/Schongli/Qistara/releases) and run it. The installer isn't signed yet, so
   Windows may say "Windows protected your PC": choose **More info → Run anyway**.
2. On the welcome screen, choose **Add Music Folder…** and pick the folder that holds your music and videos.
   You can add more folders later in **Settings › General › Library**.
3. Qistara scans the folder and keeps watching it, so new and changed files show up on their own.

Coming from Confluence Player or Sonora? On its first start Qistara copies your old library, playlists, settings and
saved sign-ins. The old app's folder is left untouched, so you can uninstall the old app once everything looks right.

## Library

- **Views:** Home, Albums, Artists, Genres, Composers, Songs, Videos, History and Your Top (most played, by period).
  Choose which ones the sidebar shows in **Settings › Appearance › Sidebar items**.
- **Search:** press **Ctrl+F** anywhere.
- **Pins:** right-click an album (or use its **⋯** button) › **Pin to Sidebar**.
- **Badges:** albums and songs show **Lossless** and **Hi-Res** badges from the files' real format.
- **Get Info:** right-click songs › **Get Info…** (or press **Ctrl+I** for the playing song) to edit tags and artwork. You can look up
  tags on MusicBrainz and artwork on the Cover Art Archive. Qistara writes tags to a copy of the file first, reads the
  copy back to check it, and only then replaces the original.
- **Duplicates** finds the same song stored more than once.
- **Back and Forward** (the arrows at the top, **Alt+←**, or the mouse's back button) return you to where you were,
  scroll position included.

## Playing music

- **Up Next:** right-click songs for **Play Next** or **Play Later**. Open the queue with the Up Next button in the
  player bar; drag rows to reorder.
- **Autoplay** (∞ in Up Next) keeps playing similar songs when the queue ends.
- **Gapless:** albums play without gaps between tracks.
- **Now Playing:** click the artwork in the player bar for full-screen Now Playing with lyrics.
- **MiniPlayer:** **Ctrl+Shift+M** shrinks Qistara to a small window that stays on top.
- **Sleep timer** (in Now Playing) stops playback after a number of minutes or at the end of the current song.
- **Media keys** on your keyboard and the Windows media overlay control playback.
- **Volume:** scroll the mouse wheel over a slider for fine steps.

## Bit-perfect audio and the Signal Path

Bit-perfect mode is on by default (**Settings › Playback › Bit-Perfect mode**). Qistara then opens your output device
in WASAPI exclusive mode at each file's own sample rate and bit depth, so the samples in the file reach your DAC
unchanged. While it is on:

- Other apps can't play sound while Qistara plays.
- The volume slider controls the device's own volume, not a software volume.
- Sound Check (ReplayGain) is off, because it changes levels.

Click the **format badge** in the player bar (for example **Hi-Res Lossless**) to open the **Signal Path**. It shows
each step from the file to your device. When the whole chain is untouched, the badge is cyan and the Signal Path says
**Bit-perfect**. If something changes the
sound, the Signal Path says what:

| Message | What it means |
|---|---|
| Shared mode: Windows mixer may resample | The device isn't open in exclusive mode (see [Troubleshooting](#troubleshooting)). |
| Resampled 96 kHz → 48 kHz | The device doesn't accept the file's sample rate. |
| Downmixed 6ch → 2ch | The file has more channels than the device. (Mono sent to both speakers stays bit-perfect.) |
| Processed (EQ) | The equalizer is on. |
| Sound Check changes level | ReplayGain is on. |
| Software volume | Volume isn't at 100% in shared mode. |
| Playback speed changed | Speed isn't 1×. |
| Truncated to 16-bit | The device takes fewer bits than the file has. |

**Bitstream to AV receiver** (Settings › Playback) sends Dolby TrueHD, Atmos, DTS-HD MA, AC-3 and DTS untouched over
HDMI or S/PDIF. Turn it on only if a receiver decodes them.

DSD files play, but are converted to PCM.

## Equalizer

Open it from **Settings › Playback › Open Equalizer** or the EQ button.

- Add up to 10 bands: Peak, Low Shelf, High Shelf, Low Cut or High Cut. Each band is coloured by the part of the
  spectrum it sits in.
- Drag a band's dot on the curve, or focus it and use the keys: **↑/↓** gain (Shift for 3 dB steps), **←/→** frequency.
- **Auto preamp** lowers the level by the curve's highest boost, so the EQ never clips.
- **Import AutoEQ…** reads an Equalizer APO / AutoEQ `ParametricEQ.txt` file. Save your own presets by name.

The EQ changes the sound, so with it on, playback is no longer bit-perfect.

## Lyrics

Open lyrics with the **Lyrics** button in the player bar (a pane beside the library) or in Now Playing. Lines scroll
with the song. Click a line to jump to it. With word-timed lyrics, each word lights up as it is sung.

**Where lyrics come from:** Qistara asks each source in your order until one has lyrics. Set the order in
**Settings › Lyrics**, by dragging a row or by focusing its grip and pressing ↑/↓.

| Source | What it is |
|---|---|
| Local files | A `.ttml`, `.lrc` or `.txt` file with the same name as the song, or lyrics embedded in its tags. |
| Navidrome / Subsonic | Your own server's lyrics, for songs played from it. |
| LRCLIB | A free, open lyrics database. |
| NetEase, Kugou | Large Chinese services with many Asian songs. Kugou often has word-by-word timing. Off by default. |

**Prefer word-synced lyrics** keeps asking further down the list for word-timed lyrics, and falls back to line-timed
ones. Online sources get only the title, artist, album and length, and what they find is saved on your PC.

**Translation** (Settings › Lyrics › Translate lyrics) shows a translation under each line, in your Windows language
or one you pick. Only the lyric lines are sent to Google Translate, and translations are cached.

**Learn mode** (the graduation-cap button above the lyrics) turns every word into a button: tap one for its meaning.
Japanese also gets furigana, romaji, dictionary forms and grammar notes, worked out offline. Saved words go to
**Vocabulary**, with the line they came from and a flashcard review (Enter shows the answer, then 1 = Again, 2 = Got It).

**Qur'an:** recitations are recognised from their titles (surah names, Juz 'Amma, Hafs or Shu'bah). Lyrics then show the
Uthmani text with word meanings, transliteration and a translation you choose (Kemenag, Saheeh International or The Clear Quran),
and keep the ayah being recited in view. If a recitation isn't recognised, use **Choose Surah…**.

## Video

Video plays in the same window, like in VLC: subtitles, audio tracks, chapters, speed, A-B loop, frame step,
screenshots and resume where you left off. See [Keyboard shortcuts](#keyboard-shortcuts) for the keys.
Screenshots are saved to `Pictures\Qistara`.

## Playlists

- **New playlist:** **New Playlist** in the sidebar, or select songs and right-click › **New Playlist from Selection**
  (or **Add to** an existing one).
- **Smart playlists** fill themselves from rules: Title, Artist, Album, Album Artist, Genre, Composer, Format, Year,
  Rating, Plays, Skips, Time, Sample Rate, Bit Depth, Bitrate, Date Added, Last Played, Loved and Lossless.
  For example, "Bit Depth is greater than 16" and "Last Played is not in the last 3 months".
- **Import and export** playlists as M3U / M3U8.
- **Ratings and Loved:** rate songs with stars and the heart; they feed smart playlists and Your Top.

## Sources and accounts

All of these are optional and live in **Settings › Providers** and **Settings › Integrations**.
Passwords, keys and sign-ins are kept in Windows Credential Manager.

### YouTube

Search, play and download through [yt-dlp](https://github.com/yt-dlp/yt-dlp), which Qistara downloads from yt-dlp's
GitHub release and checks against the release's SHA-256 list. You can use it as a guest, or sign in for your Liked
Music, age-restricted videos and, with Premium, 256 kbps audio:

- **Firefox:** sign in to YouTube in Firefox, then choose Firefox in Qistara. Your password never leaves the browser.
- **cookies.txt:** export your YouTube cookies to a file and choose it. Chrome and Edge lock their cookies on Windows,
  so they can't be read directly.

### Navidrome / Subsonic

Enter your server's address, user name and password. It works with Navidrome, Gonic, Airsonic, Ampache and other
servers that speak the Subsonic API. Type `https://` if your server uses TLS. FLAC streams as stored, so it stays
bit-perfect. The password is only sent to that server, as a salted hash.

### Spotify, Apple Music and Tidal

These import your playlists and match them to the files you already have. Their own streams are DRM-protected and
never play or download in Qistara. Each needs your own free developer app:

- **Spotify** (your account must have Premium): in the
  [Spotify Developer Dashboard](https://developer.spotify.com/dashboard) choose **Create app**, add the Redirect URI
  `http://127.0.0.1/callback`, tick **Web API**, and paste the app's **Client ID** into Qistara. You also get a
  Spotify Connect remote for the Spotify app.
- **Apple Music** needs a paid Apple Developer Program membership. Create a key with **Media Services (MusicKit)**,
  then enter your **Team ID**, the **Key ID** and the key's `.p8` file.
- **Tidal:** in the [TIDAL Developer Dashboard](https://developer.tidal.com/dashboard) create an app, add the Redirect
  URI that Qistara shows and the scopes `user.read`, `collection.read` and `playlists.read`, then paste the **Client ID**.

### Last.fm

Create a free API account at [last.fm/api](https://www.last.fm/api/account/create), paste the API key and shared
secret in **Settings › Integrations**, then connect. Each song is scrobbled once you've heard half of it (or 4 minutes).

### Radio

In **Radio & Streams**, search thousands of stations from [Radio Browser](https://www.radio-browser.info), or play
and save any stream address yourself.

## Downloads

Paste links to videos, songs, playlists or channels and choose a format:

| Format | What you get |
|---|---|
| Audio | The best audio as published (Opus or AAC), not re-encoded |
| MP3 | Converted to MP3 at the highest VBR quality (needs ffmpeg, which Qistara can fetch) |
| Video 1080p | Up to 1080p, with subtitles when ffmpeg is there |
| Video, best | The highest resolution published, 4K included |

The first time, choose **Get yt-dlp**: Qistara fetches the official build from its GitHub release and checks it
against the published SHA-256.

The folder, format and **Add downloads to the library** are remembered. The default folder is
`Music\Qistara Downloads`. Download only what you have the right to keep.

## Appearance

**Settings › Appearance** has:

- **Theme:** Light, Qistara Obsidian (dark) or Automatic, plus Catppuccin Frappé, Rosé Pine, Gruvbox Dark and Solarized Dark.
- **Accent:** nine colours, your own, or one taken from the playing song's cover.
- **Corners** from square to round, five **icon packs**, text size, compact rows and the font.
- **Window:** Mica blur behind the window, and blur in the interface, each on or off.

Settings has tabs and a search: press **Ctrl+F** on the Settings page.

## Keyboard shortcuts

| Keys | Action |
|---|---|
| Space | Play / pause |
| Ctrl+→ / Ctrl+← | Next / previous song |
| Ctrl+F | Search (on the Settings page: search settings) |
| Ctrl+O | Open files |
| Ctrl+I | Get Info for the playing song |
| Ctrl+Shift+M | MiniPlayer |
| Alt+← or mouse back button | Back |
| Esc | Close Now Playing or the side pane |

While a video plays:

| Keys | Action |
|---|---|
| ← / → | Back / forward 5 s (Shift: 30 s) |
| ↑ / ↓ | Volume |
| F or F11 | Full screen (Esc leaves it) |
| M | Mute |
| V / B | Next subtitle / audio track |
| [ / ] / = | Slower / faster / normal speed |
| , / . | One frame back / forward |
| L | A-B loop |
| I | File info |
| Ctrl+S | Screenshot |

## Where your data lives

Everything Qistara keeps is in `%LOCALAPPDATA%\com.qistara.app`:

| Path | Contents |
|---|---|
| `library.db` | The library, playlists, play counts, settings, EQ and queue |
| `art\` | Cached artwork |
| `lyrics\` | Lyrics found online, one file per song and source |
| `resume\` | Where each video was stopped |
| `bin\` | yt-dlp and ffmpeg, if you installed them from Qistara |

Sign-ins and keys are in Windows Credential Manager, under names starting with "Qistara".
To start over, quit Qistara and delete that folder. Your music files are never changed, except when you save tags
with Get Info.

## Troubleshooting

**The Signal Path says "Shared mode".** Another app may be using the device, or Windows doesn't let apps take it over.
Open **Settings › System › Sound**, choose your device, open **More sound settings** (or the device's properties) ›
**Advanced**, and tick **Allow applications to take exclusive control of this device**. Then play the song again.

**No sound from other apps while Qistara plays.** That's exclusive mode. Pause Qistara, or turn off
**Bit-Perfect mode** in Settings › Playback.

**A song is resampled.** Your device doesn't support that sample rate. Check its supported rates in the same Advanced
tab, or in your DAC's control panel.

**YouTube stopped working.** YouTube changes often. Open **Downloads › Tools** and choose **Check for Update** for yt-dlp.

**Windows warns about the installer.** The installer isn't signed yet: choose **More info → Run anyway**.

Still stuck? [Open an issue](https://github.com/Schongli/Qistara/issues) and say what you did, what you expected and what happened instead.
