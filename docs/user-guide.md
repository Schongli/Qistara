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

1. Install Qistara from [Releases](https://github.com/Schongli/Qistara/releases):
   - **Windows 10 or 11:** run `Qistara_x.y.z_x64-setup.exe`. Until the installer is code-signed, Windows may say
     "Windows protected your PC": choose **More info → Run anyway**.
   - **macOS 13 or newer, Apple Silicon (M1 and later):** open `Qistara_x.y.z_aarch64.dmg` and drag Qistara to
     Applications. Qistara plays through mpv from [Homebrew](https://brew.sh): install Homebrew if you don't have it,
     then run `brew install mpv` in Terminal. Qistara isn't signed by Apple yet, so the first time you open it macOS
     says it can't check it: open **System Settings › Privacy & Security**, scroll down and choose **Open Anyway**.
   - **Linux (x86-64):** on Ubuntu 24.04 or newer, Debian 12 or newer, Mint and similar, install the `.deb`:
     `sudo apt install ./Qistara_x.y.z_amd64.deb` (it brings libmpv with it). Anywhere else, use the `.AppImage`:
     install your system's libmpv first (Fedora `sudo dnf install mpv-libs`, Arch `sudo pacman -S mpv`, openSUSE
     `sudo zypper install libmpv2`), make the file executable (`chmod +x Qistara_*.AppImage`) and run it.

   If playback isn't set up, Qistara says so when it starts and shows the command to copy.
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

- **Right-click anything you can play** (a song, an album, a pinned album, a playlist, an artist, a genre, a
  composer, a YT Music card) for **Play**, **Shuffle**, **Play Next** and **Play Later**, and what else fits it:
  pin, go to the artist, add to a playlist, export or delete a playlist, Get Info, Music Video.
- **Up Next:** open the queue with the Up Next button in the player bar; drag rows to reorder, right-click a row to
  play it or take it out.
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
  **Shift+drag** (or scroll on a dot) changes its width (Q), **double-click** the curve adds a band and **Delete**
  removes the selected one.
- The table under the curve lists every band: type, frequency, gain, Q and on/off, typed in directly. Values are
  kept in range (20 Hz to 20 kHz, ±24 dB, Q 0.1 to 18).
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
| YT Music | The lyrics YouTube shows, for YouTube and YT Music songs (it gets only the video id). |
| NetEase, Kugou | Large Chinese services with many Asian songs. Kugou often has word-by-word timing. Off by default. |

**Prefer word-synced lyrics** keeps asking further down the list for word-timed lyrics, and falls back to line-timed
ones. Online sources get only the title, artist, album and length, and what they find is saved on your PC.

**Translation** (Settings › Lyrics › Translate lyrics) shows a translation under each line, in your system language
or one you pick. Translations are cached, so a song is translated once. **Translate with** chooses how:

- **Google, then this PC when Google is busy** (the default): Google Translate gets only the lyric lines. Qistara asks
  one song at a time and waits when Google says it's busy, then tries again by itself; if you have the language pack,
  this PC translates instead.
- **Google Translate (online)** only.
- **This PC only (offline):** Firefox's own translator, with language packs from Mozilla (about 15–50 MB each,
  downloaded once and checked). About 100 pairs to and from English, Indonesian, Japanese, Arabic, Korean and Chinese
  included; other pairs go through English (Japanese → Indonesian uses Japanese → English and English → Indonesian).
  A song that needs a pack offers it above the lyrics; **Language packs** lists them with Remove. Qistara recognises the
  lyrics' language by itself, or choose it under **Lyrics are in**. Nothing is sent anywhere.

**Learn mode** (the graduation-cap button above the lyrics) turns every word into a button: tap one for its meaning.
Japanese also gets furigana, romaji, dictionary forms and grammar notes, worked out offline with a dictionary that
Learn mode offers to download the first time (Settings › Lyrics › Learn mode shows it and removes it). Saved words go to
**Vocabulary**, with the line they came from and a flashcard review (Enter shows the answer, then 1 = Again, 2 = Got It).

**Qur'an:** recitations are recognised from their titles (surah names, Juz 'Amma, Hafs or Shu'bah). Lyrics then show the
Uthmani text with word meanings, transliteration and a translation you choose (Kemenag, Saheeh International or The Clear Quran),
and keep the ayah being recited in view. If a recitation isn't recognised, use **Choose Surah…**.
Qur'an mode speaks English or Indonesian (Settings › Lyrics › Qur'an › Language; it follows your system by default),
including the surah names (Al-Fatihah, An-Naba', …) and the word meanings, and recognises Indonesian titles such as
"QS An-Naba", "Murotal Juz 30" or "Surat Asy-Syams".

## Video

On macOS and Linux, videos play their sound for now, and the video features below are Windows-only: showing the
picture inside Qistara's window there comes in a later version.

On Windows, video plays in the same window, like in VLC: subtitles, audio tracks, chapters, speed, A-B loop, frame step,
screenshots and resume where you left off. See [Keyboard shortcuts](#keyboard-shortcuts) for the keys.
Screenshots are saved to `Pictures\Qistara`.

**YouTube with its video:** right-click a YouTube or YT Music song › **Watch Video**, or press the **Video** button in
the player bar (Ctrl+Shift+V) while it plays: the same moment, now with the picture, up to 1080p. Press it again for
sound only.

**Music videos for your own songs:** right-click a song › **Add Music Video…**, then paste a YouTube link, choose
**Find on YouTube**, or pick a video from your library. When the song plays, its video shows over it, without the
video's own sound: what you hear is still your file, bit-perfect, counted in your play stats, with its lyrics. If the
video has an intro or starts late, use **Sync** in the video's controls (in tenths of a second); Qistara remembers it.
The **Video** button (Ctrl+Shift+V) hides or shows music videos.

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
Passwords, keys and sign-ins are kept in the OS secret store (Windows Credential Manager, the macOS Keychain, or
Secret Service on Linux).

### YT Music

**YT Music** in the sidebar browses YouTube Music: Home shelves, search by songs, albums, artists or playlists, and
album, artist and playlist pages. Signed in (below), Home is yours and **Library** shows Liked Music, your playlists,
saved albums and the artists you follow. **Start Radio** (in a YT Music or YouTube song's menu) keeps playing similar
songs, and Autoplay ∞ does the same when the queue ends. Covers and lyrics for these songs go to the streaming cache
(Settings › General › Storage). **Links & Videos** keeps the YouTube search, links and Liked Music.

**How songs play:** Qistara's own streaming engine asks YouTube for the song's sound and starts playing as the first
bytes arrive; it sends YouTube only the video id. The next songs in the queue, a song your pointer rests on and the
first song of an album or playlist you open are fetched ahead, so they start at once. A ring around Play shows while a
song opens. If the engine can't open a song (an age-restricted one, say), yt-dlp plays it instead, signed in if you
are. **Settings › Playback › Streaming › Test** shows how fast YouTube answers from your connection, and which way
the song came.

### YouTube

Search and download through [yt-dlp](https://github.com/yt-dlp/yt-dlp), which Qistara downloads from yt-dlp's
GitHub release (unpacked, so it starts quickly) and checks against the release's SHA-256 list. You can use YouTube
as a guest, or sign in for your YT Music library, Liked Music, age-restricted songs and, with Premium, 256 kbps audio.

**Sign In with Google** (Settings › Providers › YouTube) opens Google's own sign-in page in a small window of
Qistara's. Sign in as you would in a browser, two-step verification included; when YouTube has signed you in, the
window closes and Providers shows your name.
- **Your password** goes only to Google.
- **The window** is private and can only open Google's and YouTube's pages.
- **What Qistara keeps** is YouTube's sign-in cookies, encrypted in its data folder, with the key in the OS secret
  store. They go only to YouTube: to YT Music, and to yt-dlp when it plays or downloads.
- **Sign Out** deletes them.

If Google won't sign you in there ("This browser or app may not be secure"), use one of the **Other ways to sign in**:

- **Firefox:** sign in to YouTube in Firefox, then choose Firefox in Qistara. Your password never leaves the browser.
- **cookies.txt:** export your YouTube cookies to a file and choose it. Chrome and Edge lock their cookies on Windows,
  so they can't be read directly.

Your cookies reach yt-dlp only as a copy made for that one run and deleted afterwards (yt-dlp writes back to
the file it's given).

**Audio quality:** **Standard** plays through the streaming engine and starts fastest. **Premium** asks yt-dlp with
your account first, for YouTube Premium's 256 kbps audio when it's offered to you; songs take a moment longer to
start.

YouTube needs a JavaScript runtime for yt-dlp. If Node.js 20 or newer is installed, it's used; otherwise choose
**Get Deno** in Downloads › Tools, which fetches Deno from its GitHub release and checks its SHA-256.

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

### Discord

**Settings › Integrations › Discord › Show on Discord** (off by default) shows "Listening to Qistara" on your
Discord profile, with the title, artist, album and a time bar, while the Discord app runs on the same PC. It talks
to the Discord app directly, not over the internet. A cover is shown only when it's already a public picture (YT
Music, YouTube); **Artwork for local files** looks your album up on MusicBrainz (artist and album only) for a Cover
Art Archive picture. Your file names and folders are never sent.

## Listen together

Play music with friends: everyone hears the same song at the same moment, and there's a chat beside it.

1. **Host:** open the **Session** tab in the pane (the people button in the player bar), type your name and choose
   **Start a Session**. The first time, your system may ask whether Qistara can accept connections: allow it for
   private networks.
2. **Share the invite** (`qistara://session/…`, **Copy**). Guests on your Wi-Fi or a VPN such as Tailscale or ZeroTier
   can use the address shown; over the internet, forward the port (47800) on your router. The part after `#` is the
   key: share the invite only with people you'd let in.
3. **Join:** paste the invite into the Session tab and choose **Join**.

What you play is what they hear, and see: a YouTube song you watch plays as video for them, and a YouTube music
video you show over a song shows for them too. Your own files play for them as well (they're read from your PC while they play, never saved on
theirs), YT Music, YouTube and radio. Songs from your Navidrome/Subsonic server aren't shared, because their
addresses carry your sign-in. Guests can't pause or seek while they follow you. Remove a guest (the × by their name) to
disconnect them: the invite changes, so the old one stops working. Everything between you is encrypted.

## Downloads

Paste links to videos, songs, playlists or channels. With a single link, Qistara shows its title, length and the
highest quality published, and the size of each choice. Choose **Video** or **Audio**, then a quality:

| Choice | What you get |
|---|---|
| Video: Best, 2160p … 240p | Up to that height, as MKV with the best video and audio merged (without ffmpeg, the best single file) |
| Audio: Original | The best audio as published (Opus or AAC), not re-encoded |
| Audio: M4A | AAC in an M4A file, as published |
| Audio: MP3 320, 192, 128 | Converted to MP3 at that bitrate (needs ffmpeg, which Qistara can fetch) |

The first time, choose **Get yt-dlp**: Qistara fetches the official build from its GitHub release and checks it
against the published SHA-256.

The folder, the choice and **Add downloads to the library** are remembered, and quick downloads (a song's menu) use them. The default folder is
`Music\Qistara Downloads`. Download only what you have the right to keep.

## Appearance

**Settings › Appearance** has:

- **Theme:** Light, Qistara Obsidian (dark) or Automatic, plus 17 families, most with light and dark variants picked
  under the card: Pastel, Moonbloom, Timber, Sundial, Fjord, Nocturne, Neon City, Evergreen, Ukiyo, Slate, Andalus,
  Madinah, Sakura, Matcha, Pitch Black (for OLED screens) and WibuFest.
- **Adaptive theme:** the whole app takes its colours from the cover of what's playing, and changes with each song.
- **Accent:** nine colours, your own, or one taken from the playing song's cover.
- **Corners** from square to round, five **icon packs**, text size, compact rows and the font.
- **Window:** Mica blur behind the window, and blur in the interface, each on or off.

Settings has tabs and a search: press **Ctrl+F** on the Settings page.

The window can be as narrow as 360 px: the sidebar becomes icons, then a drawer (the menu button, top left), and
the pane becomes a sheet. On a touch screen, one tap plays a song and a long press opens its menu.

## Updates

**Settings › General › Updates** checks Qistara's GitHub releases once a day (turn off **Check for updates
automatically** to stop it) and says when a new version is out, with what's new. Nothing is downloaded until you
choose **Install and Restart**. An update installs only if it carries Qistara's release signature for that version.
This works for the Windows installer, the macOS app, the AppImage and the `.deb` (which asks for your password).

## Keyboard shortcuts

Press **Ctrl+/** (or **?**) anywhere for this list in the app. On macOS, Ctrl is ⌘. The video keys (Ctrl+Shift+V and
those for while a video plays) are Windows-only for now.

| Keys | Action |
|---|---|
| Space | Play / pause |
| Ctrl+→ | Next song |
| Ctrl+← | Previous song |
| Shift+→ | Forward 5 seconds |
| Shift+← | Back 5 seconds |
| Ctrl+↑ | Volume up |
| Ctrl+↓ | Volume down |
| Ctrl+S | Shuffle on / off |
| Ctrl+R | Repeat: off, all, one |
| Ctrl+L | Love the playing song |
| Ctrl+Shift+V | Show the playing song’s video |
| Ctrl+F | Search (in Settings: search settings) |
| Ctrl+N | New playlist |
| Ctrl+O | Open files |
| Ctrl+I | Get Info for the playing song |
| Alt+← | Back |
| Alt+→ | Forward |
| Ctrl+Shift+L | Lyrics pane |
| Ctrl+Shift+Q | Up Next pane |
| Ctrl+Shift+M | MiniPlayer |
| Ctrl+, | Settings |
| Ctrl+/ | Keyboard shortcuts |
| Esc | Close Now Playing, a sheet or the side pane |

While a video plays:

| Keys | Action |
|---|---|
| ← | Back 5 s (Shift: 30 s) |
| → | Forward 5 s (Shift: 30 s) |
| ↑ | Volume up |
| ↓ | Volume down |
| F | Full screen (F11 too) |
| M | Mute |
| V | Next subtitle |
| B | Next audio track |
| [ | Slower |
| ] | Faster |
| = | Normal speed |
| , | One frame back |
| . | One frame forward |
| L | A-B loop |
| I | File info |
| Ctrl+S | Screenshot |

## Where your data lives

Everything Qistara keeps is in its app data folder: `%LOCALAPPDATA%\com.qistara.app` on Windows,
`~/Library/Application Support/com.qistara.app` on macOS, `~/.local/share/com.qistara.app` on Linux.

| Path | Contents |
|---|---|
| `library.db` | The library, playlists, play counts, settings, EQ and queue |
| `art\` | Cached artwork |
| `lyrics\` | Lyrics found online, one file per song and source |
| `resume\` | Where each video was stopped |
| `bin\` | yt-dlp, ffmpeg and Deno, if you installed them from Qistara |
| `dict\` | The Japanese dictionary, if you installed it |
| `streaming\` | Covers and lyrics for your server and YT Music, yt-dlp's cache and download scratch space: Settings › General › Storage clears them. Also your YouTube sign-in, encrypted (`account.bin`; Sign Out deletes it) |

Qistara's log (what went wrong, for a bug report) is in `%LOCALAPPDATA%\com.qistara.app\logs` on Windows,
`~/Library/Logs/com.qistara.app` on macOS and `~/.local/share/com.qistara.app/logs` on Linux: **Settings › General ›
Updates › Show Log** opens it. It stays on your computer.

Sign-ins and keys are in the OS secret store, under names starting with "Qistara".
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

**YouTube stopped working.** YouTube changes often. Open **Downloads › Tools** and choose **Check for Update** for
yt-dlp, then try **Settings › Playback › Streaming › Test**: it says which way a song came, or why none worked.
Qistara's log has a line for each song with the time it took.

**"Google Translate is busy."** Qistara waits and tries again by itself. To stop depending on Google, add the language
pack in Settings › Lyrics › Translation › Language packs: with the default setting, this PC takes over when Google is
busy.

**"YouTube needs a JavaScript runtime."** Choose **Get Deno** in Downloads › Tools, or install Node.js 20 or newer.

**Friends can't join a session.** Check that your firewall lets Qistara accept connections on private networks, that
they use the address for the network you share (Wi-Fi or VPN), and that the invite is the current one (removing a
guest changes it).

**macOS: "Shared mode".** Bit-perfect uses hog mode, which takes the device for Qistara alone; another app holding it
prevents that. **Linux:** choose an `hw:` device in Settings › Playback › Output; `default` goes through the mixer.

**Windows warns about the installer.** Until the installer is code-signed: choose **More info → Run anyway**.

**"Install mpv to play music" (macOS) or "Install libmpv to play music" (Linux).** Run the command it shows (Copy, then
paste it into Terminal), then **Restart Qistara**. On macOS that needs Homebrew first ([brew.sh](https://brew.sh)).

**macOS: "Qistara can't be opened" or "Apple could not verify".** Qistara isn't signed by Apple yet. Open **System
Settings › Privacy & Security** and choose **Open Anyway** (once). After an update macOS may ask again for access to
your saved sign-ins in the Keychain: choose **Always Allow**.

**Linux: sign-ins aren't kept.** They go to the system keyring (Secret Service): GNOME and KDE have one. On other
desktops, install and start `gnome-keyring` or KeePassXC with Secret Service turned on.

**Linux: the window stays blank.** Some graphics drivers (NVIDIA's in particular) don't work with WebKitGTK's
renderer. Start Qistara with `WEBKIT_DISABLE_DMABUF_RENDERER=1 qistara`.

Still stuck? [Open an issue](https://github.com/Schongli/Qistara/issues) and say what you did, what you expected and what happened instead.
