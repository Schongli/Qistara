<script lang="ts">
  import { open } from '@tauri-apps/plugin-dialog'
  import { revealItemInDir } from '@tauri-apps/plugin-opener'
  import DownloadSimple from 'phosphor-svelte/lib/DownloadSimple'
  import FolderOpen from 'phosphor-svelte/lib/FolderOpen'
  import X from 'phosphor-svelte/lib/X'
  import ArrowsClockwise from 'phosphor-svelte/lib/ArrowsClockwise'
  import CheckCircle from 'phosphor-svelte/lib/CheckCircle'
  import WarningCircle from 'phosphor-svelte/lib/WarningCircle'
  import Toggle from '../components/Toggle.svelte'
  import { PRESETS, add, cancel, clearFinished, defaultFolder, dl, install, refreshTools, updateYtdlp, type Preset } from '../lib/downloads.svelte'

  let text = $state('')
  let preset = $state<Preset>('audioOriginal')
  let dest = $state('')
  let toLibrary = $state(true)
  refreshTools()
  defaultFolder().then((d) => (dest ||= d))

  const urls = $derived(text.split(/\s+/).filter((u) => /^https?:\/\//i.test(u)))
  const ffmpeg = $derived(!!dl.tools?.ffmpeg)
  const needsFfmpeg = $derived(!!PRESETS.find((p) => p.id === preset)?.ffmpeg && !ffmpeg)
  const mb = (n: number) => `${(n / 1048576).toFixed(0)} MB`

  async function pick() {
    const d = await open({ directory: true, multiple: false, title: 'Save downloads in' })
    if (typeof d === 'string') dest = d
  }
  async function start() {
    await add(urls, preset, dest, toLibrary)
    if (!dl.error) text = ''
  }
</script>

<h1 class="t-title">Downloads</h1>
<p class="t-sub lede">Save videos, songs and whole playlists from YouTube, SoundCloud, Bandcamp and the many other sites yt-dlp supports. Download only what you have the right to keep. Music from DRM services (Spotify, Apple Music, Tidal, Deezer) can't be downloaded.</p>

{#if !dl.tools}
  <p class="t-sub">Checking for yt-dlp…</p>
{:else if !dl.tools.ytdlp}
  <section class="card">
    <h2 class="t-h2">Get yt-dlp</h2>
    <p class="t-sub">Downloads come through yt-dlp, a free open-source tool. Confluence Player fetches the official Windows build from its GitHub release (about 18 MB) and checks it against the release's published SHA-256 before using it.</p>
    <button class="btn primary press" onclick={() => install('ytdlp')} disabled={!!dl.installing}>
      <DownloadSimple size={15} weight="bold" />{dl.installing === 'ytdlp' ? `Downloading… ${dl.toolTotal ? Math.round((dl.toolDone / dl.toolTotal) * 100) : 0}%` : 'Get yt-dlp'}
    </button>
  </section>
{:else}
  <section class="card">
    <textarea bind:value={text} rows="4" placeholder="Paste links, one per line: videos, songs, playlists or channels" aria-label="Links to download" spellcheck="false"></textarea>
    <div class="presets" role="radiogroup" aria-label="Format">
      {#each PRESETS as p (p.id)}
        <button class="preset press" role="radio" aria-checked={preset === p.id} class:on={preset === p.id} title={p.hint} onclick={() => (preset = p.id)}>{p.label}</button>
      {/each}
    </div>
    <p class="t-sub hint">{PRESETS.find((p) => p.id === preset)?.hint}{#if !ffmpeg && preset !== 'audioOriginal'} Without ffmpeg, video is limited to files that already include sound, and nothing gets cover art or tags.{/if}</p>
    <div class="dest">
      <span class="ellipsis" title={dest}>Save in <b>{dest || '…'}</b></span>
      <button class="btn press" onclick={pick}><FolderOpen size={15} />Change…</button>
    </div>
    <Toggle bind:checked={toLibrary} label="Add to the library" hint="Adds this folder to your library, so finished downloads appear with the rest of your music." />
    <div class="go">
      <button class="btn primary press" onclick={start} disabled={!urls.length || !dest || needsFfmpeg}>
        <DownloadSimple size={15} weight="bold" />{urls.length > 1 ? `Download ${urls.length} Links` : 'Download'}
      </button>
      {#if needsFfmpeg}<span class="t-sub">MP3 needs ffmpeg (below).</span>{/if}
    </div>
  </section>

  {#if dl.jobs.length}
    <div class="row-head">
      <h2 class="t-h2">This session</h2>
      <button class="btn press" onclick={clearFinished}>Clear Finished</button>
    </div>
    <ul class="jobs">
      {#each dl.jobs as j (j.id)}
        <li class="job">
          <div class="jt">
            <span class="ellipsis"><b>{j.title || j.urls[0]}</b>{#if j.urls.length > 1} and {j.urls.length - 1} more{/if}</span>
            <span class="t-sub ellipsis">
              {#if j.status === 'queued'}Waiting
              {:else if j.status === 'running'}{j.percent !== null ? `${j.percent.toFixed(0)}%` : 'Starting'}{j.eta ? ` · ${j.eta} left` : ''}{j.files.length ? ` · ${j.files.length} saved` : ''}
              {:else if j.status === 'done'}Saved {j.files.length || ''} {j.files.length === 1 ? 'file' : 'files'}
              {:else if j.status === 'cancelled'}Cancelled
              {:else}{j.error}{/if}
            </span>
            {#if j.status === 'running'}<div class="bar"><div style:width="{j.percent ?? 0}%"></div></div>{/if}
          </div>
          {#if j.status === 'done'}<span class="ok"><CheckCircle size={18} weight="fill" /></span>{/if}
          {#if j.status === 'failed'}<span class="bad"><WarningCircle size={18} weight="fill" /></span>{/if}
          {#if j.files.length}<button class="icon-btn" aria-label="Show in File Explorer" title="Show in File Explorer" onclick={() => revealItemInDir(j.files[j.files.length - 1])}><FolderOpen size={16} /></button>{/if}
          {#if j.status === 'queued' || j.status === 'running'}<button class="icon-btn" aria-label="Cancel" title="Cancel" onclick={() => cancel(j.id)}><X size={15} weight="bold" /></button>{/if}
        </li>
      {/each}
    </ul>
  {/if}

  <section class="tools">
    <h2 class="t-h2">Tools</h2>
    <div class="line">
      <span>yt-dlp <span class="t-sub">{dl.tools.ytdlp}</span></span>
      <button class="btn press" onclick={updateYtdlp}><ArrowsClockwise size={14} />Check for Update</button>
    </div>
    <div class="line">
      <span>ffmpeg <span class="t-sub ellipsis">{dl.tools.ffmpeg ?? 'not found: needed for MP3, best video and cover art'}</span></span>
      {#if !ffmpeg}
        <button class="btn press" onclick={() => install('ffmpeg')} disabled={!!dl.installing}>
          {dl.installing === 'ffmpeg' ? `Downloading… ${dl.toolTotal ? `${mb(dl.toolDone)} of ${mb(dl.toolTotal)}` : ''}` : 'Get ffmpeg (about 200 MB)'}
        </button>
      {/if}
    </div>
  </section>
{/if}
{#if dl.error}<p class="err">{dl.error}</p>{/if}

<style>
  h1 { margin: 0 0 6px; }
  .lede { max-width: 720px; margin: 0 0 20px; line-height: 1.5; }
  .card { max-width: 760px; display: grid; gap: 12px; padding: 16px; border-radius: 12px; background: var(--fill); margin-bottom: 26px; }
  .card h2, .card p { margin: 0; }
  .card .btn { justify-self: start; }
  textarea {
    width: 100%; resize: vertical; padding: 10px 12px; border-radius: 8px; border: 1px solid var(--sep); background: var(--bg);
    color: var(--text); font: 12.5px/1.5 ui-monospace, 'Cascadia Mono', Consolas, monospace; user-select: text;
  }
  textarea:focus { outline: none; border-color: var(--accent); }
  .presets { display: flex; flex-wrap: wrap; gap: 6px; }
  .preset { height: 28px; padding: 0 12px; border-radius: 14px; background: var(--fill-2); font-weight: 550; font-size: 12px; }
  .preset.on { background: var(--accent); color: var(--on-accent); }
  .hint { line-height: 1.45; }
  .dest { display: flex; align-items: center; justify-content: space-between; gap: 12px; min-width: 0; }
  .go { display: flex; align-items: center; gap: 12px; }
  .btn:disabled { opacity: 0.5; }
  .row-head { display: flex; align-items: baseline; justify-content: space-between; max-width: 760px; margin-bottom: 8px; }
  .row-head h2 { margin: 0; }
  .jobs { list-style: none; margin: 0 0 26px; padding: 0; max-width: 760px; display: grid; gap: 6px; }
  .job { display: flex; align-items: center; gap: 8px; padding: 10px 10px 10px 14px; border-radius: 10px; background: var(--fill); }
  .jt { flex: 1; min-width: 0; display: grid; gap: 2px; }
  .bar { height: 3px; margin-top: 6px; border-radius: 2px; background: var(--fill-2); overflow: hidden; }
  .bar div { height: 100%; background: var(--accent); transition: width 300ms; }
  .ok { color: #30d158; display: grid; }
  .bad { color: #e5484d; display: grid; }
  .tools { max-width: 760px; }
  .tools h2 { margin: 0 0 8px; }
  .line { display: flex; align-items: center; justify-content: space-between; gap: 12px; min-height: 44px; padding: 4px 8px 4px 14px; border-radius: 10px; background: var(--fill); margin-bottom: 6px; min-width: 0; }
  .line > span { min-width: 0; display: flex; gap: 8px; align-items: baseline; }
  .err { color: #e5484d; max-width: 760px; }
</style>
