<script lang="ts">
  import { open } from '@tauri-apps/plugin-dialog'
  import FolderSimplePlus from 'phosphor-svelte/lib/FolderSimplePlus'
  import ArrowsClockwise from 'phosphor-svelte/lib/ArrowsClockwise'
  import X from 'phosphor-svelte/lib/X'
  import { addFolder, lib, removeFolder, rescan } from '../lib/library.svelte'
  import { mpv, set } from '../lib/player.svelte'
  import { settings } from '../lib/settings.svelte'
  import { nav } from '../lib/nav.svelte'
  import Toggle from '../components/Toggle.svelte'
  import { NAV } from '../components/Sidebar.svelte'
  import { ACCENTS } from '../lib/appearance.svelte'
  import { invoke } from '@tauri-apps/api/core'
  import { openUrl } from '@tauri-apps/plugin-opener'
  import { connect, finish, lastfm, refresh } from '../lib/lastfm.svelte'

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
  import { plural } from '../lib/format'
  import { LANGUAGES, languageName, targetFor } from '../lib/translate'

  const THEMES = [['auto', 'Automatic'], ['light', 'Light'], ['dark', 'Dark']] as const
  const DENSITY = [['comfortable', 'Comfortable'], ['compact', 'Compact']] as const
  const accent = $derived(settings.accent || ACCENTS[0][1])
  // live while the picker is dragged; saved (and made readable) when it closes
  const preview = (hex: string) => document.documentElement.style.setProperty('--accent', hex)
  const toggleNav = (id: string) =>
    (settings.hiddenNav = settings.hiddenNav.includes(id) ? settings.hiddenNav.filter((x) => x !== id) : [...settings.hiddenNav, id])

  export async function pickFolder() {
    const dir = await open({ directory: true, multiple: false, title: 'Add a music or video folder' })
    if (typeof dir === 'string') await addFolder(dir)
  }
</script>

<h1 class="t-title">Settings</h1>

<section>
  <h2 class="t-h2">Appearance</h2>
  <div class="group">
    <div class="line">
      <span>Theme</span>
      <div class="seg" role="radiogroup" aria-label="Theme">
        {#each THEMES as [v, label] (v)}
          <button role="radio" aria-checked={settings.theme === v} class:on={settings.theme === v} onclick={() => (settings.theme = v)}>{label}</button>
        {/each}
      </div>
    </div>
    <div class="line">
      <span>Accent colour</span>
      <div class="swatches" role="radiogroup" aria-label="Accent colour">
        {#each ACCENTS as [name, hex], i (hex)}
          <button class="sw press" role="radio" style:--c={hex} aria-label={name} title={name} aria-checked={accent === hex} onclick={() => (settings.accent = i ? hex : '')}></button>
        {/each}
        <label class="sw custom" title="Custom colour" class:on={!ACCENTS.some(([, h]) => h === accent)}>
          <input type="color" value={accent} aria-label="Custom accent colour" oninput={(e) => preview(e.currentTarget.value)} onchange={(e) => (settings.accent = e.currentTarget.value)} />
        </label>
      </div>
    </div>
    <Toggle bind:checked={settings.matchArtwork} label="Match the artwork" hint="The accent takes its colour from the cover of what's playing; grey covers keep your accent." />
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
    <label class="line">
      <span>Font</span>
      <select bind:value={settings.font}>
        <option value="inter">Inter</option>
        <option value="system">Segoe UI (Windows)</option>
      </select>
    </label>
    <Toggle bind:checked={settings.solidChrome} label="Solid window" hint="Turns off the Mica translucency behind the sidebar and bars." />
  </div>
  <p class="t-sub sub2">Sidebar items</p>
  <div class="group checks">
    {#each NAV.flatMap((g) => g.items) as it (it.id)}
      <label class="check"><input type="checkbox" checked={!settings.hiddenNav.includes(it.id)} onchange={() => toggleNav(it.id)} />{it.label}</label>
    {/each}
  </div>
</section>

<section>
  <h2 class="t-h2">Library</h2>
  <p class="t-sub">Confluence Player watches these folders and picks up new or changed files automatically. {plural(lib.tracks.length, 'file')} in your library.</p>
  <div class="group">
    {#each lib.folders as f (f)}
      <div class="line">
        <span class="ellipsis">{f}</span>
        <button class="icon-btn" aria-label="Remove {f}" title="Remove from library (files stay on disk)" onclick={() => removeFolder(f)}><X size={14} weight="bold" /></button>
      </div>
    {:else}
      <div class="line t-sub">No folders yet.</div>
    {/each}
  </div>
  <div class="actions">
    <button class="btn press" onclick={pickFolder}><FolderSimplePlus size={15} />Add Folder…</button>
    <button class="btn press" onclick={() => nav.root({ name: 'duplicates' })}>Find Duplicates…</button>
    <button class="btn press" onclick={rescan} disabled={!!lib.scan}><ArrowsClockwise size={15} />{lib.scan ? 'Scanning…' : 'Rescan'}</button>
  </div>
  {#if lib.error}<p class="err">{lib.error}</p>{/if}
</section>

<section>
  <h2 class="t-h2">Audio</h2>
  <p class="t-sub">Bit-perfect mode sends every sample to your DAC untouched: WASAPI exclusive output at the file's own sample rate and bit depth, with nothing in between. Volume then controls your device directly.</p>
  <div class="group">
    <Toggle bind:checked={settings.bitPerfect} label="Bit-Perfect mode" hint="Exclusive access to the output device. Other apps can't play sound while Confluence Player plays." />
    <Toggle bind:checked={settings.soundCheck} disabled={settings.bitPerfect} label="Sound Check" hint={settings.bitPerfect ? 'Changes levels, so it is off in Bit-Perfect mode.' : 'Plays songs at a similar loudness using ReplayGain tags.'} />
    <Toggle bind:checked={settings.passthrough} label="Bitstream to AV receiver" hint="Sends Dolby TrueHD, Atmos, DTS-HD MA, AC-3 and DTS untouched over HDMI or S/PDIF. Only turn on if a receiver decodes them." />
  </div>
</section>

<section>
  <h2 class="t-h2">Lyrics</h2>
  <div class="group">
    <Toggle bind:checked={settings.onlineLyrics} label="Find lyrics online" hint="When a song has no .lrc/.ttml file or embedded lyrics, look it up on LRCLIB (title, artist, album and length are sent). Results are cached." />
    <Toggle bind:checked={settings.translateLyrics} label="Translate lyrics" hint="Shows a translation under each line. Only the lyric lines are sent to Google Translate; translations are cached." />
    <label class="line">
      <span>Translate to</span>
      <select bind:value={settings.lyricsLanguage}>
        <option value="">{languageName(targetFor(navigator.language))} (Windows language)</option>
        {#each LANGUAGES as [code, name] (code)}<option value={code}>{name}</option>{/each}
      </select>
    </label>
  </div>
</section>

<section>
  <h2 class="t-h2">Last.fm</h2>
  {#if lastfm.user}
    <div class="line group"><span>Scrobbling as <b>{lastfm.user}</b>{#if lastfm.pending} · {lastfm.pending} waiting to send{/if}</span><button class="btn press" onclick={logout}>Disconnect</button></div>
  {:else if lastfm.configured}
    <p class="t-sub">Approve Confluence Player on Last.fm in your browser, then come back and finish.</p>
    <div class="actions">
      <button class="btn primary press" onclick={connect}>Connect to Last.fm…</button>
      {#if lastfm.waiting}<button class="btn press" onclick={finish}>I've Approved It</button>{/if}
      <button class="btn press" onclick={logout}>Use Different Keys</button>
    </div>
  {:else}
    <p class="t-sub">Scrobbles each song once you've heard half of it (or 4 minutes). Uses your own free API account:
      <button class="link" onclick={() => openUrl('https://www.last.fm/api/account/create')}>create one at last.fm/api</button>, then paste the key and shared secret. They're stored in Windows Credential Manager.</p>
    <div class="keys">
      <input bind:value={key} placeholder="API key" aria-label="Last.fm API key" spellcheck="false" />
      <input bind:value={secret} type="password" placeholder="Shared secret" aria-label="Last.fm shared secret" spellcheck="false" />
      <button class="btn press" onclick={saveKeys} disabled={!key || !secret}>Save</button>
    </div>
  {/if}
  {#if lastfm.error}<p class="err">{lastfm.error}</p>{/if}
</section>

<section>
  <h2 class="t-h2">Providers</h2>
  <p class="t-sub">YouTube, your music server, and Spotify, Apple Music and Tidal sign-in moved to their own page.</p>
  <button class="btn press" onclick={() => nav.root({ name: 'providers' })}>Open Providers</button>
</section>

<section>
  <h2 class="t-h2">Video</h2>
  <div class="group">
    <Toggle bind:checked={settings.hwdec} label="Hardware decoding" hint="Decodes on your GPU. Output is identical; turn off only if a file shows glitches." />
  </div>
</section>

<section>
  <h2 class="t-h2">Output</h2>
  <label class="line">
    <span>Audio device</span>
    <select value={mpv.device} onchange={(e) => set('audio-device', e.currentTarget.value)}>
      {#each mpv.devices.filter((d) => d.name === 'auto' || d.name.startsWith('wasapi/')) as d (d.name)}
        <option value={d.name}>{d.name === 'auto' ? 'System default' : d.description}</option>
      {/each}
    </select>
  </label>
</section>

<style>
  h1 { margin: 0 0 24px; }
  section { max-width: 720px; margin-bottom: 36px; }
  h2 { margin: 0 0 4px; }
  p { margin: 0 0 12px; }
  .group { border-radius: 10px; background: var(--fill); overflow: hidden; }
  .line { display: flex; align-items: center; justify-content: space-between; gap: 12px; min-height: 42px; padding: 4px 8px 4px 14px; border-bottom: 1px solid var(--sep); }
  .line:last-child { border-bottom: 0; }
  section > .line { border-radius: 10px; background: var(--fill); }
  .actions { display: flex; gap: 10px; margin-top: 12px; }
  .btn:disabled { opacity: 0.6; }
  select { max-width: 60%; padding: 4px 8px; border-radius: 6px; border: 1px solid var(--sep); background: var(--bg-2); color: var(--text); }
  .err { color: #e5484d; margin-top: 10px; }
  .keys { display: grid; grid-template-columns: 1fr 1fr auto; gap: 8px; }
  .keys input { height: 30px; padding: 0 10px; border-radius: 7px; border: 1px solid var(--sep); background: var(--bg-2); color: var(--text); min-width: 0; }
  .link { color: var(--accent); text-decoration: underline; text-underline-offset: 2px; }
  .seg { display: flex; padding: 2px; border-radius: 8px; background: var(--fill-2); }
  .seg button { height: 26px; padding: 0 12px; border-radius: 6px; font-size: 12px; font-weight: 550; color: var(--text-2); }
  .seg button.on { background: var(--bg); color: var(--text); box-shadow: 0 1px 2px rgba(0, 0, 0, 0.12); }
  .swatches { display: flex; flex-wrap: wrap; gap: 8px; padding: 6px 0; }
  .sw { width: 22px; height: 22px; border-radius: 50%; background: var(--c); box-shadow: inset 0 0 0 1px rgba(0, 0, 0, 0.12); }
  .sw[aria-checked='true'], .sw.custom.on { outline: 2px solid var(--c, var(--accent)); outline-offset: 2px; }
  .sw.custom { position: relative; overflow: hidden; background: conic-gradient(#ff4d4d, #ffd60a, #30d158, #0a84ff, #bf5af2, #ff4d4d); }
  .sw.custom input { position: absolute; inset: -6px; width: 34px; height: 34px; opacity: 0; cursor: default; }
  .sub2 { margin: 14px 0 6px; }
  .checks { display: grid; grid-template-columns: repeat(auto-fill, minmax(160px, 1fr)); gap: 2px 12px; padding: 10px 14px; }
  .check { display: flex; align-items: center; gap: 8px; height: 26px; }
  .check input { accent-color: var(--accent); }
</style>
