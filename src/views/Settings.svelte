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
  import { invoke } from '@tauri-apps/api/core'
  import { openUrl } from '@tauri-apps/plugin-opener'
  import { connect, finish, lastfm, refresh } from '../lib/lastfm.svelte'
  import { connect as connectSpotify, disconnect, setClient, spotify } from '../lib/spotify.svelte'

  let clientId = $state('')
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

  export async function pickFolder() {
    const dir = await open({ directory: true, multiple: false, title: 'Add a music or video folder' })
    if (typeof dir === 'string') await addFolder(dir)
  }
</script>

<h1 class="t-title">Settings</h1>

<section>
  <h2 class="t-h2">Library</h2>
  <p class="t-sub">Sonora watches these folders and picks up new or changed files automatically. {plural(lib.tracks.length, 'file')} in your library.</p>
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
    <Toggle bind:checked={settings.bitPerfect} label="Bit-Perfect mode" hint="Exclusive access to the output device. Other apps can't play sound while Sonora plays." />
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
    <p class="t-sub">Approve Sonora on Last.fm in your browser, then come back and finish.</p>
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
  <h2 class="t-h2">Spotify</h2>
  {#if spotify.user}
    <div class="line group">
      <span>Connected as <b>{spotify.user}</b></span>
      <span class="actions tight">
        <button class="btn press" onclick={() => nav.root({ name: 'spotify' })}>Open Spotify</button>
        <button class="btn press" onclick={() => disconnect()}>Disconnect</button>
      </span>
    </div>
    <p class="t-sub hint">Disconnecting deletes Sonora's access. To remove it on Spotify's side too, use
      <button class="link" onclick={() => openUrl('https://www.spotify.com/account/apps/')}>spotify.com/account/apps</button>.</p>
  {:else if spotify.configured}
    <p class="t-sub">Approve Sonora in the browser window that opens. Sonora never sees your Spotify password.</p>
    <div class="actions">
      <button class="btn primary press" onclick={connectSpotify} disabled={spotify.waiting}>{spotify.waiting ? 'Waiting for your browser…' : 'Connect to Spotify…'}</button>
      <button class="btn press" onclick={() => disconnect(true)}>Use a Different Client ID</button>
    </div>
  {:else}
    <p class="t-sub">Import your Spotify playlists as playlists of your own files, and control the Spotify app from Sonora. Spotify needs your own free developer app (your account must have Premium):</p>
    <ol class="steps t-sub">
      <li>Open the <button class="link" onclick={() => openUrl('https://developer.spotify.com/dashboard')}>Spotify Developer Dashboard</button> and choose <b>Create app</b>.</li>
      <li>Name it Sonora, add the Redirect URI <code>http://127.0.0.1/callback</code> and tick <b>Web API</b>.</li>
      <li>Copy the app's <b>Client ID</b> and paste it below. No client secret is needed.</li>
    </ol>
    <div class="keys one">
      <input bind:value={clientId} placeholder="Client ID" aria-label="Spotify Client ID" spellcheck="false" />
      <button class="btn press" onclick={() => setClient(clientId.trim())} disabled={!clientId.trim()}>Save</button>
    </div>
  {/if}
  {#if spotify.error}<p class="err">{spotify.error}</p>{/if}
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
  .keys.one { grid-template-columns: 1fr auto; max-width: 480px; }
  .actions.tight { margin-top: 0; }
  .hint { margin-top: 8px; }
  .steps { margin: 0 0 12px; padding-left: 20px; line-height: 1.7; }
  code { font-size: 12px; padding: 1px 5px; border-radius: 4px; background: var(--fill-2); user-select: text; }
</style>
