<script lang="ts">
  import { KEY_STORE, SYSTEM } from '../lib/os'
  import { tick } from 'svelte'
  import { open } from '@tauri-apps/plugin-dialog'
  import { invoke } from '@tauri-apps/api/core'
  import { openUrl } from '@tauri-apps/plugin-opener'
  import Icon from '../components/Icon.svelte'
  import { addFolder, lib, removeFolder, rescan } from '../lib/library.svelte'
  import { mpv, OS, set } from '../lib/player.svelte'
  import { pickable } from '../lib/signal'
  import { settings } from '../lib/settings.svelte'
  import { nav, ui } from '../lib/nav.svelte'
  import Toggle from '../components/Toggle.svelte'
  import LyricsSources from '../components/LyricsSources.svelte'
  import Providers from './Providers.svelte'
  import { NAV } from '../components/Sidebar.svelte'
  import { ACCENTS } from '../lib/appearance.svelte'
  import { FAMILIES, LIGHT, OBSIDIAN, presetOf, type Family, type Palette, type ThemeId } from '../lib/themes'
  import { connect, finish, lastfm, refresh } from '../lib/lastfm.svelte'
  import { defaultFolder } from '../lib/downloads.svelte'
  import { plural } from '../lib/format'
  import { LANGUAGES, languageName, targetFor } from '../lib/translate'
  import { installDictionary, learn } from '../lib/learn.svelte'
  import { forgetCovers } from '../lib/subsonic.svelte'
  import { usePack } from '../lib/icons.svelte'
  import { discord } from '../lib/discord.svelte'
  import { queue } from '../lib/queue.svelte'
  import Artwork from '../components/Artwork.svelte'
  import { mb } from '../lib/format'

  let { tab = 'general' }: { tab?: string } = $props()

  const TABS = [
    ['general', 'General', 'Gear'], ['appearance', 'Appearance', 'PaintBrush'], ['playback', 'Playback', 'SpeakerHigh'],
    ['lyrics', 'Lyrics', 'Quotes'], ['providers', 'Providers', 'PlugsConnected'], ['integrations', 'Integrations', 'LinkSimple'],
  ] as const
  const DENSITY = [['comfortable', 'Comfortable'], ['compact', 'Compact']] as const
  const PACKS = [['phosphor', 'Phosphor'], ['lucide', 'Lucide'], ['iconoir', 'Iconoir'], ['remix', 'Remix'], ['solar', 'Solar']] as const
  const SAMPLE: [string, string][] = [['House', 'regular'], ['Heart', 'fill'], ['Play', 'fill'], ['Shuffle', 'regular'], ['MagnifyingGlass', 'regular']]
  const CORNERS = [['square', 'Square', 0], ['subtle', 'Subtle', 0.5], ['rounded', 'Rounded', 1], ['round', 'Round', 1.75]] as const

  // ---- search: every tab's sections at once, with the rows that don't mention the query hidden ----
  let query = $state('')
  let page = $state<HTMLDivElement>()
  let field = $state<HTMLInputElement>()
  let empty = $state(false)
  const shown = (t: string) => !!query.trim() || tab === t
  const opened = ui.settingsSearch
  $effect(() => {
    if (ui.settingsSearch !== opened) field?.focus()
  })
  // shortcut: filters the rendered DOM by its text, which covers every row without a keyword list to maintain
  function filter(root: HTMLElement, q: string) {
    let any = false
    for (const sec of root.querySelectorAll<HTMLElement>('section')) {
      const title = sec.querySelector('h2')?.textContent?.toLowerCase() ?? ''
      const rows = [...sec.querySelectorAll<HTMLElement>('.row, .line, .src, .check, .theme, .corner, .pack, .prov')]
      const whole = !q || title.includes(q)
      let hit = whole
      for (const r of rows) {
        const on = whole || r.textContent!.toLowerCase().includes(q)
        r.hidden = !on
        hit ||= on
      }
      if (!rows.length) hit ||= sec.textContent!.toLowerCase().includes(q)
      sec.hidden = !hit
      any ||= hit
    }
    empty = !!q && !any
  }
  $effect(() => {
    const q = query.trim().toLowerCase()
    tab
    tick().then(() => page && filter(page, q))
  })
  const goTab = (t: string) => {
    query = ''
    nav.replace({ name: 'settings', tab: t })
  }

  // ---- appearance ----
  // the icon-pack previews need every pack, so they load once Appearance is open
  $effect(() => {
    if (shown('appearance')) for (const [p] of PACKS) usePack(p)
  })
  const preset = $derived(presetOf(settings.theme))
  const accent = $derived(settings.accent || preset?.accent || ACCENTS[0][1])
  // live while the picker is dragged; saved (and made readable) when it closes
  const preview = (hex: string) => document.documentElement.style.setProperty('--accent', hex)
  const toggleNav = (id: string) =>
    (settings.hiddenNav = settings.hiddenNav.includes(id) ? settings.hiddenNav.filter((x) => x !== id) : [...settings.hiddenNav, id])
  const builtIn: [ThemeId, string, Palette][] = [['light', 'Light', LIGHT], ['dark', 'Obsidian', OBSIDIAN]]
  // a family's card shows its chosen variant, or its first one
  const shown_ = (f: Family) => f.variants.find((v) => v.id === settings.theme) ?? f.variants[0]

  // ---- library and downloads ----
  export async function pickFolder() {
    const dir = await open({ directory: true, multiple: false, title: 'Add a music or video folder' })
    if (typeof dir === 'string') await addFolder(dir)
  }
  let fallback = $state('')
  defaultFolder().then((d) => (fallback = d))
  async function pickDownloads() {
    const d = await open({ directory: true, multiple: false, title: 'Save downloads in' })
    if (typeof d === 'string') settings.downloadFolder = d
  }

  // ---- storage: the library's own data apart from what streaming leaves behind (lib.rs storage_usage) ----
  type Usage = { libraryArt: number; lyrics: number; streaming: number; tools: number; dictionary: number }
  let usage = $state<Usage | null>(null)
  const refreshUsage = () => invoke<Usage>('storage_usage').then((u) => (usage = u), () => (usage = null))
  $effect(() => {
    if (shown('general')) refreshUsage()
  })
  async function clearStreaming() {
    await invoke('clear_streaming_cache')
    forgetCovers()
    refreshUsage()
  }
  async function clearLyrics() {
    await invoke('clear_lyrics_cache')
    refreshUsage()
  }

  // ---- Japanese dictionary (optional download, japanese.rs) ----
  let dict = $state<{ installed: boolean; bytes: number } | null>(null)
  const refreshDict = () => invoke<{ installed: boolean; bytes: number }>('japanese_status').then((d) => (dict = d), () => (dict = null))
  $effect(() => {
    learn.dict
    if (shown('lyrics')) refreshDict()
  })
  async function removeDict() {
    await invoke('japanese_remove').catch((e) => (learn.error = String(e)))
    refreshDict()
  }

  // ---- Last.fm ----
  let key = $state('')
  let secret = $state('')
  async function saveKeys() {
    lastfm.error = ''
    try {
      await invoke('lastfm_set_keys', { key, secret })
      key = secret = ''
      await refresh()
    } catch (e) {
      lastfm.error = String(e)
    }
  }
  async function logout() {
    await invoke('lastfm_logout')
    lastfm.user = null
    lastfm.configured = false
  }
</script>

{#snippet mini(p: Palette)}
  <span class="mini" style:--b={p.bg} style:--b2={p.bg2} style:--t={p.text} style:--t3={p.text3} style:--a={p.accent}>
    <i class="m-side"><b></b><b></b><b></b></i>
    <i class="m-main"><b class="m-h"></b><b class="m-tile"></b><b class="m-tile"></b><b class="m-tile acc"></b></i>
    <i class="m-bar"><b></b></i>
  </span>
{/snippet}

<div class="page" bind:this={page}>
  <label class="find">
    <Icon name="MagnifyingGlass" size={14} weight="bold" />
    <input bind:this={field} bind:value={query} type="search" placeholder="Search settings" aria-label="Search settings" spellcheck="false"
      onkeydown={(e) => e.key === 'Escape' && ((query = ''), field?.blur())} />
    <kbd>Ctrl F</kbd>
  </label>

  <nav class="tabs" aria-label="Settings sections">
    {#each TABS as [id, label, icon] (id)}
      <button class="press" aria-current={!query && tab === id ? 'page' : undefined} class:on={!query && tab === id} onclick={() => goTab(id)}><Icon name={icon} size={14} />{label}</button>
    {/each}
  </nav>

  {#if empty}<p class="t-sub none">No settings match “{query.trim()}”.</p>{/if}

  {#if shown('general')}
    <section>
      <h2 class="t-h2">Library</h2>
      <p class="t-sub">Qistara watches these folders and picks up new or changed files automatically. {plural(lib.tracks.length, 'file')} in your library.</p>
      <div class="group">
        {#each lib.folders as f (f)}
          <div class="line">
            <span class="ellipsis">{f}</span>
            <button class="icon-btn" aria-label="Remove {f}" title="Remove from library (files stay on disk)" onclick={() => removeFolder(f)}><Icon name="X" size={14} weight="bold" /></button>
          </div>
        {:else}
          <div class="line t-sub">No folders yet.</div>
        {/each}
      </div>
      <div class="actions">
        <button class="btn press" onclick={pickFolder}><Icon name="FolderSimplePlus" size={15} />Add Folder…</button>
        <button class="btn press" onclick={() => nav.root({ name: 'duplicates' })}>Find Duplicates…</button>
        <button class="btn press" onclick={rescan} disabled={!!lib.scan}><Icon name="ArrowsClockwise" size={15} />{lib.scan ? 'Scanning…' : 'Rescan'}</button>
      </div>
      {#if lib.error}<p class="err">{lib.error}</p>{/if}
    </section>

    <section>
      <h2 class="t-h2">Downloads</h2>
      <div class="group">
        <div class="line">
          <span class="txt"><span>Save downloads in</span><span class="hint ellipsis" title={settings.downloadFolder || fallback}>{settings.downloadFolder || fallback}</span></span>
          <span class="btns">
            {#if settings.downloadFolder}<button class="btn press" onclick={() => (settings.downloadFolder = '')}>Use Default</button>{/if}
            <button class="btn press" onclick={pickDownloads}><Icon name="FolderOpen" size={15} />Change…</button>
          </span>
        </div>
        <Toggle bind:checked={settings.downloadToLibrary} label="Add downloads to the library" hint="New downloads appear with the rest of your music." />
      </div>
    </section>

    <section>
      <h2 class="t-h2">Storage</h2>
      <p class="t-sub">What streaming leaves behind is kept apart from your library, so it can be cleared without touching your music, playlists or settings.</p>
      <div class="group">
        <div class="line">
          <span class="txt"><span>Streaming cache</span><span class="hint">Covers and lyrics from your server and YT Music{usage ? ` · ${mb(usage.streaming)}` : ''}</span></span>
          <button class="btn press" onclick={clearStreaming}>Clear</button>
        </div>
        <div class="line">
          <span class="txt"><span>Lyrics found online</span><span class="hint">For songs in your library; looked up again on the next play{usage ? ` · ${mb(usage.lyrics)}` : ''}</span></span>
          <button class="btn press" onclick={clearLyrics}>Clear</button>
        </div>
        <div class="line"><span class="txt"><span>Library artwork</span><span class="hint">Rebuilt from your files when needed{usage ? ` · ${mb(usage.libraryArt)}` : ''}</span></span></div>
        <div class="line"><span class="txt"><span>Tools and dictionary</span><span class="hint">yt-dlp, ffmpeg, Deno; the Japanese dictionary{usage ? ` · ${mb(usage.tools + usage.dictionary)}` : ''}</span></span></div>
      </div>
    </section>
  {/if}

  {#if shown('appearance')}
    <section>
      <h2 class="t-h2">Theme</h2>
      <div class="themes" role="radiogroup" aria-label="Theme">
        <button class="theme press" role="radio" aria-checked={settings.theme === 'auto'} onclick={() => (settings.theme = 'auto')}>
          <span class="shot">{@render mini(OBSIDIAN)}<span class="half">{@render mini(LIGHT)}</span></span>
          <span class="name">Automatic</span>
        </button>
        {#each builtIn as [id, label, p] (id)}
          <button class="theme press" role="radio" aria-checked={settings.theme === id} onclick={() => (settings.theme = id)}>
            <span class="shot">{@render mini(p)}</span>
            <span class="name">{label}</span>
          </button>
        {/each}
        {#each FAMILIES as f (f.id)}
          {@const cur = shown_(f)}
          {@const on = f.variants.some((v) => v.id === settings.theme)}
          <div class="theme fam">
            <button class="press pick" role="radio" aria-checked={on} aria-label="{f.label} {f.variants.length > 1 ? cur.label : ''}" title={f.note} onclick={() => (settings.theme = cur.id)}>
              <span class="shot">{@render mini(cur)}</span>
            </button>
            {#if f.variants.length > 1}
              <select class="variant" value={cur.id} aria-label="{f.label} variant" onchange={(e) => (settings.theme = e.currentTarget.value)}>
                {#each f.variants as v (v.id)}<option value={v.id}>{f.label} · {v.label}</option>{/each}
              </select>
            {:else}
              <span class="name">{f.label}</span>
            {/if}
          </div>
        {/each}
      </div>
      <div class="group">
        <div class="line">
          <span>Accent colour</span>
          <div class="swatches" role="radiogroup" aria-label="Accent colour">
            {#each ACCENTS as [name, hex], i (hex)}
              <button class="sw press" role="radio" style:--c={hex} aria-label={name} title={name} aria-checked={accent === hex} onclick={() => (settings.accent = i || preset ? hex : '')}></button>
            {/each}
            <label class="sw custom" title="Custom colour" class:on={!ACCENTS.some(([, h]) => h === accent) && accent !== preset?.accent}>
              <input type="color" value={accent} aria-label="Custom accent colour" oninput={(e) => preview(e.currentTarget.value)} onchange={(e) => (settings.accent = e.currentTarget.value)} />
            </label>
          </div>
        </div>
        {#if preset && settings.accent}<div class="line"><span>Accent</span><button class="btn press" onclick={() => (settings.accent = '')}>Use the Theme's Own</button></div>{/if}
        <Toggle bind:checked={settings.adaptiveTheme} label="Adaptive theme" hint="The whole window takes on the colours of what's playing, dark or light as your theme is. With nothing playing, or a grey cover, your theme comes back." />
        <Toggle bind:checked={settings.matchArtwork} disabled={settings.adaptiveTheme} label="Adaptive accent" hint={settings.adaptiveTheme ? 'Part of the adaptive theme while it is on.' : "Takes the accent from the cover of what's playing; grey covers keep your accent."} />
      </div>
    </section>

    <section>
      <h2 class="t-h2">Corners</h2>
      <div class="corners" role="radiogroup" aria-label="Corners">
        {#each CORNERS as [id, label, k] (id)}
          <button class="corner press" role="radio" aria-checked={settings.corners === id} onclick={() => (settings.corners = id)}>
            <span class="c-box"><i style:border-top-left-radius="{14 * k}px"></i></span>
            <span class="name">{label}</span>
          </button>
        {/each}
      </div>
    </section>

    <section>
      <h2 class="t-h2">Icon pack</h2>
      <div class="packs" role="radiogroup" aria-label="Icon pack">
        {#each PACKS as [id, label] (id)}
          <button class="pack press" role="radio" aria-checked={settings.iconPack === id} onclick={() => (settings.iconPack = id)}>
            <span class="p-box">{#each SAMPLE as [n, w] (n)}<Icon name={n} pack={id} weight={w} size={15} />{/each}</span>
            <span class="name">{label}</span>
          </button>
        {/each}
      </div>
    </section>

    <section>
      <h2 class="t-h2">Text</h2>
      <div class="group">
        <label class="line">
          <span>Font</span>
          <select bind:value={settings.font}>
            <option value="inter">Inter</option>
            <option value="system">{OS === 'windows' ? 'Segoe UI (Windows)' : 'System font'}</option>
          </select>
        </label>
        <label class="line">
          <span>Text size</span>
          <select bind:value={settings.zoom}>
            {#each [0.9, 1, 1.1, 1.25] as z (z)}<option value={z}>{Math.round(z * 100)}%</option>{/each}
          </select>
        </label>
        <div class="line">
          <span>Rows</span>
          <div class="seg" role="radiogroup" aria-label="Row density">
            {#each DENSITY as [v, label] (v)}
              <button role="radio" aria-checked={settings.density === v} class:on={settings.density === v} onclick={() => (settings.density = v)}>{label}</button>
            {/each}
          </div>
        </div>
      </div>
    </section>

    <section>
      <h2 class="t-h2">Window</h2>
      <div class="group">
        {#if OS === 'windows'}<Toggle bind:checked={() => !settings.solidChrome, (v) => (settings.solidChrome = !v)} label="Blur window" hint="Lets the Windows Mica material show through the sidebar and bars." />{/if}
        <Toggle bind:checked={settings.blurUi} label="Blur UI" hint="Frosted bars, menus and popovers. Off makes them solid." />
      </div>
    </section>

    <section>
      <h2 class="t-h2">Sidebar items</h2>
      <div class="group checks">
        {#each NAV.flatMap((g) => g.items) as it (it.id)}
          <label class="check"><input type="checkbox" checked={!settings.hiddenNav.includes(it.id)} onchange={() => toggleNav(it.id)} />{it.label}</label>
        {/each}
      </div>
    </section>
  {/if}

  {#if shown('playback')}
    <section>
      <h2 class="t-h2">Audio</h2>
      <p class="t-sub">Bit-perfect mode sends every sample to your DAC untouched: {OS === 'windows' ? 'WASAPI exclusive output' : OS === 'macos' ? 'CoreAudio hog mode' : 'ALSA straight to the device (pick a “hw:” device below)'} at the file's own sample rate and bit depth, with nothing in between. Volume then controls your device directly.</p>
      <div class="group">
        <Toggle bind:checked={settings.bitPerfect} label="Bit-Perfect mode" hint="Exclusive access to the output device. Other apps can't play sound while Qistara plays." />
        <Toggle bind:checked={settings.soundCheck} disabled={settings.bitPerfect} label="Sound Check" hint={settings.bitPerfect ? 'Changes levels, so it is off in Bit-Perfect mode.' : 'Plays songs at a similar loudness using ReplayGain tags.'} />
        <Toggle bind:checked={settings.passthrough} label="Bitstream to AV receiver" hint="Sends Dolby TrueHD, Atmos, DTS-HD MA, AC-3 and DTS untouched over HDMI or S/PDIF. Only turn on if a receiver decodes them." />
        <div class="line"><span>Equalizer</span><button class="btn press" onclick={() => nav.root({ name: 'eq' })}>Open Equalizer</button></div>
      </div>
    </section>

    <section>
      <h2 class="t-h2">Output</h2>
      <div class="group">
        <label class="line">
          <span>Audio device</span>
          <select value={mpv.device} onchange={(e) => set('audio-device', e.currentTarget.value)}>
            {#each mpv.devices.filter((d) => pickable(OS, d.name)) as d (d.name)}
              <option value={d.name}>{d.name === 'auto' ? 'System default' : d.description}</option>
            {/each}
          </select>
        </label>
      </div>
    </section>

    <section>
      <h2 class="t-h2">Video</h2>
      <div class="group">
        <Toggle bind:checked={settings.hwdec} label="Hardware decoding" hint="Decodes on your GPU. Output is identical; turn off only if a file shows glitches." />
      </div>
    </section>
  {/if}

  {#if shown('lyrics')}
    <section>
      <h2 class="t-h2">Lyrics sources</h2>
      <p class="t-sub">Asked from the top until one has lyrics. Online sources get only the title, artist, album and length; what they find is kept on this PC.</p>
      <div class="group">
        <Toggle bind:checked={settings.preferWordSync} label="Prefer word-synced lyrics" hint="Keeps asking further down the list for lyrics timed word by word, and falls back to line-synced ones." />
      </div>
      <p class="t-sub sub2">Priority · drag a row, or focus its grip and press ↑/↓</p>
      <div class="group"><LyricsSources /></div>
    </section>

    <section>
      <h2 class="t-h2">Translation</h2>
      <div class="group">
        <Toggle bind:checked={settings.translateLyrics} label="Translate lyrics" hint="Shows a translation under each line. Only the lyric lines are sent to Google Translate; translations are cached." />
        <label class="line">
          <span>Translate to</span>
          <select bind:value={settings.lyricsLanguage}>
            <option value="">{languageName(targetFor(navigator.language))} ({SYSTEM} language)</option>
            {#each LANGUAGES as [code, name] (code)}<option value={code}>{name}</option>{/each}
          </select>
        </label>
      </div>
    </section>

    <section>
      <h2 class="t-h2">Qur'an</h2>
      <div class="group">
        <label class="line">
          <span class="txt"><span>Language</span><span class="hint">Qur'an mode's labels, surah names (Kemenag spelling in Indonesian) and word meanings</span></span>
          <select bind:value={settings.quranLang}>
            <option value="auto">Automatic ({SYSTEM})</option>
            <option value="en">English</option>
            <option value="id">Bahasa Indonesia</option>
          </select>
        </label>
      </div>
    </section>

    <section>
      <h2 class="t-h2">Learn mode</h2>
      <div class="group">
        <div class="line">
          <span>Japanese dictionary <span class="t-sub">{learn.dict === 'installing' ? 'installing…' : dict?.installed ? `installed, ${mb(dict.bytes)}` : 'not installed: needed for readings and word forms'}</span></span>
          {#if dict?.installed}<button class="btn press" onclick={removeDict}>Remove</button>
          {:else}<button class="btn press" disabled={learn.dict === 'installing'} onclick={installDictionary}>Install (12 MB)</button>{/if}
        </div>
      </div>
    </section>
  {/if}

  {#if shown('providers')}
    <div class="providers"><Providers embedded /></div>
  {/if}

  {#if shown('integrations')}
    <section>
      <h2 class="t-h2">Discord</h2>
      <div class="dc-wrap">
        <div class="dc" aria-label="How it looks on Discord">
          <span class="dc-head">Listening to Qistara</span>
          <div class="dc-body">
            <Artwork art={discord.cover ?? queue.current?.art} size="76px" radius="8px" shadow={false} />
            <div class="dc-txt">
              <b class="ellipsis">{queue.current?.title ?? 'Song title'}</b>
              <span class="ellipsis">{queue.current ? `by ${queue.current.artist || 'Unknown artist'}` : 'by Artist'}</span>
              <span class="ellipsis">{queue.current?.album || 'Album'}</span>
            </div>
          </div>
          <span class="dc-btn">Get Qistara</span>
        </div>
      </div>
      <div class="group">
        <Toggle bind:checked={settings.discord} disabled={!discord.ready} label="Show on Discord" hint={discord.ready ? 'Your Discord profile says what you play, while it plays. Only the title, artist, album and times go to the Discord app on this PC.' : "Discord isn't set up in this build: release builds carry Qistara's Discord application."} />
      </div>
      <p class="t-sub sub2">Local files</p>
      <div class="group">
        <Toggle bind:checked={settings.discordLocalArt} disabled={!settings.discord} label="Artwork for local files" hint="Discord can't see your files, so this looks the album up on MusicBrainz (artist and album only) and shows its Cover Art Archive picture. Off: the Qistara logo." />
      </div>
    </section>

    <section>
      <h2 class="t-h2">Last.fm</h2>
      {#if lastfm.user}
        <div class="line group"><span>Scrobbling as <b>{lastfm.user}</b>{#if lastfm.pending} · {lastfm.pending} waiting to send{/if}</span><button class="btn press" onclick={logout}>Disconnect</button></div>
      {:else if lastfm.configured}
        <p class="t-sub">Approve Qistara on Last.fm in your browser, then come back and finish.</p>
        <div class="actions">
          <button class="btn primary press" onclick={connect}>Connect to Last.fm…</button>
          {#if lastfm.waiting}<button class="btn press" onclick={finish}>I've Approved It</button>{/if}
          <button class="btn press" onclick={logout}>Use Different Keys</button>
        </div>
      {:else}
        <p class="t-sub">Scrobbles each song once you've heard half of it (or 4 minutes). Uses your own free API account:
          <button class="link" onclick={() => openUrl('https://www.last.fm/api/account/create')}>create one at last.fm/api</button>, then paste the key and shared secret. They're stored in {KEY_STORE}.</p>
        <div class="keys">
          <input bind:value={key} placeholder="API key" aria-label="Last.fm API key" spellcheck="false" />
          <input bind:value={secret} type="password" placeholder="Shared secret" aria-label="Last.fm shared secret" spellcheck="false" />
          <button class="btn press" onclick={saveKeys} disabled={!key || !secret}>Save</button>
        </div>
      {/if}
      {#if lastfm.error}<p class="err">{lastfm.error}</p>{/if}
    </section>
  {/if}
</div>

<style>
  .page { max-width: 720px; margin: 0 auto; }
  .find {
    display: flex; align-items: center; gap: 8px; height: 34px; padding: 0 10px 0 12px; margin-bottom: 12px;
    border-radius: var(--r-lg); background: var(--fill-2); color: var(--text-2);
  }
  .find:focus-within { box-shadow: 0 0 0 2px var(--accent); background: var(--bg); }
  .find input { flex: 1; min-width: 0; height: 100%; border: 0; background: none; outline: none; color: var(--text); }
  .find input::-webkit-search-cancel-button { display: none; }
  .find:focus-within kbd { display: none; }
  kbd { font: 600 10px var(--font); color: var(--text-3); border: 1px solid var(--sep); border-radius: var(--r-xs); padding: 1px 4px; }
  .tabs { display: flex; flex-wrap: wrap; justify-content: center; gap: 2px; margin-bottom: 26px; }
  .tabs button {
    display: inline-flex; align-items: center; gap: 6px; height: 28px; padding: 0 10px; border-radius: var(--r);
    font-weight: 550; color: var(--text-2); transition: background-color 120ms, color 120ms;
  }
  .tabs button:hover { color: var(--text); background: var(--fill); }
  .tabs button.on { color: var(--text); background: var(--fill-2); }
  .none { text-align: center; margin: 24px 0; }
  section { margin-bottom: 32px; }
  h2 { margin: 0 0 4px; }
  p { margin: 0 0 12px; }
  h2 + .group, h2 + .themes, h2 + .corners { margin-top: 10px; }
  .group { border-radius: var(--r-lg); background: var(--fill); overflow: hidden; }
  .line { display: flex; align-items: center; justify-content: space-between; gap: 12px; min-height: 42px; padding: 4px 8px 4px 14px; border-bottom: 1px solid var(--sep); }
  .line:last-child { border-bottom: 0; }
  .txt { display: grid; gap: 2px; min-width: 0; }
  .hint { font-size: 12px; color: var(--text-2); }
  .btns { display: flex; gap: 8px; flex: none; }
  .actions { display: flex; gap: 10px; margin-top: 12px; }
  .btn:disabled { opacity: 0.6; }
  select { max-width: 60%; padding: 4px 8px; border-radius: var(--r-sm); border: 1px solid var(--sep); background: var(--bg-2); color: var(--text); }
  .err { color: var(--signal-veil); margin-top: 10px; }
  .keys { display: grid; grid-template-columns: 1fr 1fr auto; gap: 8px; }
  .keys input { height: 30px; padding: 0 10px; border-radius: var(--r); border: 1px solid var(--sep); background: var(--bg-2); color: var(--text); min-width: 0; }
  .link { color: var(--accent); text-decoration: underline; text-underline-offset: 2px; }
  .seg { display: flex; padding: 2px; border-radius: var(--r); background: var(--fill-2); }
  .seg button { height: 26px; padding: 0 12px; border-radius: var(--r-sm); font-size: 12px; font-weight: 550; color: var(--text-2); }
  .seg button.on { background: var(--bg); color: var(--text); box-shadow: 0 1px 2px rgba(0, 0, 0, 0.12); }
  .swatches { display: flex; flex-wrap: wrap; gap: 8px; padding: 6px 0; }
  .sw { width: 22px; height: 22px; border-radius: 50%; background: var(--c); box-shadow: inset 0 0 0 1px rgba(0, 0, 0, 0.12); }
  .sw[aria-checked='true'], .sw.custom.on { outline: 2px solid var(--c, var(--accent)); outline-offset: 2px; }
  .sw.custom { position: relative; overflow: hidden; background: conic-gradient(#ff4d4d, #ffd60a, #30d158, #0a84ff, #bf5af2, #ff4d4d); }
  .sw.custom input { position: absolute; inset: -6px; width: 34px; height: 34px; opacity: 0; cursor: default; }
  .sub2 { margin: 16px 0 6px; font-weight: 600; }
  .checks { display: grid; grid-template-columns: repeat(auto-fill, minmax(160px, 1fr)); gap: 2px 12px; padding: 10px 14px; }
  .check { display: flex; align-items: center; gap: 8px; height: 26px; }
  .check input { accent-color: var(--accent); }

  /* theme and corner cards: a little window in each theme */
  .packs { display: grid; grid-template-columns: repeat(auto-fill, minmax(118px, 1fr)); gap: 14px 12px; margin-top: 10px; }
  .pack { display: grid; gap: 7px; justify-items: center; }
  .p-box {
    display: flex; align-items: center; justify-content: center; gap: 8px; width: 100%; height: 44px; border-radius: var(--r-lg);
    background: var(--fill); color: var(--text); box-shadow: 0 0 0 1px var(--sep); transition: box-shadow 160ms var(--ease-out);
  }
  [aria-checked='true'] > .p-box { box-shadow: 0 0 0 2px var(--accent), 0 0 0 4px color-mix(in srgb, var(--accent) 25%, transparent); }
  .themes, .corners { display: grid; grid-template-columns: repeat(auto-fill, minmax(124px, 1fr)); gap: 14px 12px; margin-bottom: 14px; }
  .theme, .corner { display: grid; gap: 7px; justify-items: center; }
  .dc-wrap { display: grid; place-items: center; padding: 18px; border-radius: var(--r-lg); border: 1px solid var(--sep); margin: 10px 0 12px; }
  .dc { display: grid; gap: 8px; width: min(360px, 100%); padding: 12px; border-radius: 10px; background: #232428; color: #dbdee1; }
  .dc-head { font-size: 12px; color: #b5bac1; }
  .dc-body { display: flex; gap: 12px; align-items: center; min-width: 0; }
  .dc-txt { display: grid; gap: 2px; min-width: 0; font-size: 12.5px; color: #b5bac1; }
  .dc-txt b { color: #f2f3f5; font-size: 14px; }
  .dc-btn { display: grid; place-items: center; height: 32px; border-radius: 4px; background: #4e5058; color: #fff; font-size: 13px; font-weight: 600; }
  .pick { display: block; width: 100%; border-radius: var(--r); }
  .variant { max-width: 100%; height: 22px; padding: 0 4px; border: 0; border-radius: var(--r-sm); background: transparent; color: var(--text-2); font: inherit; font-size: 12px; text-align: center; cursor: pointer; }
  .variant:hover, .variant:focus-visible { background: var(--fill); color: var(--text); }
  .shot, .c-box {
    position: relative; display: block; width: 100%; aspect-ratio: 16 / 10; border-radius: var(--r-lg); overflow: hidden;
    box-shadow: 0 0 0 1px var(--sep); transition: box-shadow 160ms var(--ease-out);
  }
  [aria-checked='true'] > .shot, [aria-checked='true'] > .c-box { box-shadow: 0 0 0 2px var(--accent), 0 0 0 4px color-mix(in srgb, var(--accent) 25%, transparent); }
  .name { font-size: 12px; color: var(--text-2); }
  [aria-checked='true'] > .name { color: var(--text); font-weight: 600; }
  .half { position: absolute; inset: 0; clip-path: polygon(0 0, 100% 0, 0 100%); }
  .mini { position: absolute; inset: 0; display: grid; grid-template-columns: 26% 1fr; grid-template-rows: 1fr 18%; background: var(--b); }
  .mini i { display: flex; gap: 4px; padding: 6px; }
  .mini b { display: block; border-radius: 2px; background: var(--t3); opacity: 0.55; }
  .m-side { grid-row: 1 / 3; flex-direction: column; background: var(--b2); }
  .m-side b { height: 3px; width: 70%; }
  .m-main { flex-wrap: wrap; align-content: flex-start; }
  .m-h { width: 60%; height: 4px; flex: 0 0 60%; background: var(--t) !important; opacity: 0.8 !important; }
  .m-tile { width: 26%; aspect-ratio: 1; }
  .m-tile.acc { background: var(--a) !important; opacity: 1 !important; }
  .m-bar { grid-column: 2; align-items: center; background: var(--b2); }
  .m-bar b { height: 3px; width: 40%; margin: 0 auto; background: var(--a); opacity: 1; }
  .c-box { background: var(--fill); }
  .c-box i { position: absolute; left: 28%; top: 30%; right: -1px; bottom: -1px; border: 2px solid var(--text-2); border-right: 0; border-bottom: 0; }
  .providers { margin-bottom: 32px; }
</style>
