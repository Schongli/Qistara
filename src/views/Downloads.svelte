<script lang="ts">
  import { FILE_MANAGER, SYSTEM } from '../lib/os'
  import { open } from '@tauri-apps/plugin-dialog'
  import { revealItemInDir } from '@tauri-apps/plugin-opener'
  import Icon from '../components/Icon.svelte'
  import Toggle from '../components/Toggle.svelte'
  import {
    AUDIO_Q, VIDEO_Q, add, cancel, choiceLabel, clearFinished, defaultFolder, dl, install, needsFfmpeg, probe, refreshTools,
    updateYtdlp, videoLabel, type Choice, type Probe, type VideoQ,
  } from '../lib/downloads.svelte'
  import { settings } from '../lib/settings.svelte'
  import { mb, time } from '../lib/format'

  let text = $state('')
  // the folder, format, quality and library choice live in settings, so they stay as you left them
  let fallback = $state('')
  defaultFolder().then((d) => (fallback = d))
  const dest = $derived(settings.downloadFolder || fallback)
  refreshTools()

  const urls = $derived(text.split(/\s+/).filter((u) => /^https?:\/\//i.test(u)))
  const ffmpeg = $derived(!!dl.tools?.ffmpeg)

  // one video link: ask what it offers, so each quality shows its size (a playlist link has no single answer)
  let info = $state<Probe | null>(null)
  let looking = $state(false)
  let lookErr = $state('')
  let seq = 0
  $effect(() => {
    const u = urls.length === 1 ? urls[0] : ''
    const one = u && (!/[?&]list=/.test(u) || /[?&]v=/.test(u)) ? u : ''
    info = null
    lookErr = ''
    looking = false
    if (!one || !dl.tools?.ytdlp) return
    const mine = ++seq
    looking = true
    const t = setTimeout(() => {
      probe(one).then((p) => mine === seq && (info = p), (e) => mine === seq && (lookErr = String(e))).finally(() => mine === seq && (looking = false))
    }, 500)
    return () => clearTimeout(t)
  })

  // a height this video doesn't have isn't offered; the saved one comes back for the next link that has it
  const videoChoices = $derived<[VideoQ, number | null][]>(info ? info.video : VIDEO_Q.map((q) => [q, null]))
  const video = $derived(videoChoices.some(([q]) => q === settings.downloadVideo) ? settings.downloadVideo : 'best')
  const choice = $derived<Choice>(settings.downloadKind === 'video' ? { kind: 'video', quality: video } : { kind: 'audio', quality: settings.downloadAudio })
  const size = $derived(info ? (choice.kind === 'video' ? info.video.find(([q]) => q === choice.quality)?.[1] : info.audio.find(([q]) => q === choice.quality)?.[1]) : null)
  const blocked = $derived(needsFfmpeg(choice) && !ffmpeg)
  const hint = $derived(choice.kind === 'audio' ? AUDIO_Q.find((a) => a.id === choice.quality)?.hint : choice.quality === 'best' ? 'The highest resolution published, 4K included.' : `Up to ${choice.quality}p.`)
  const runtime = $derived(dl.tools?.js ? `${dl.tools.js.startsWith('node') ? 'Node.js' : 'Deno'} · ${dl.tools.js.replace(/^\w+:/, '')}` : null)

  async function pick() {
    const d = await open({ directory: true, multiple: false, title: 'Save downloads in' })
    if (typeof d === 'string') settings.downloadFolder = d
  }
  async function start() {
    await add(urls, choice, dest, settings.downloadToLibrary)
    if (!dl.error) text = ''
  }
  const pct = () => (dl.toolTotal ? Math.round((dl.toolDone / dl.toolTotal) * 100) : 0)
</script>

<h1 class="t-title">Downloads</h1>
<p class="t-sub lede">Save videos, songs and whole playlists from YouTube, SoundCloud, Bandcamp and the many other sites yt-dlp supports. Download only what you have the right to keep. Music from DRM services (Spotify, Apple Music, Tidal, Deezer) can't be downloaded.</p>

{#if !dl.tools}
  <p class="t-sub">Checking for yt-dlp…</p>
{:else if !dl.tools.ytdlp}
  <section class="card">
    <h2 class="t-h2">Get yt-dlp</h2>
    <p class="t-sub">Downloads come through yt-dlp, a free open-source tool. Qistara fetches the official {SYSTEM} build from its GitHub release (about 18 MB) and checks it against the release's published SHA-256 before using it.</p>
    <button class="btn primary press" onclick={() => install('ytdlp')} disabled={!!dl.installing}>
      <Icon name="DownloadSimple" size={15} weight="bold" />{dl.installing === 'ytdlp' ? `Downloading… ${pct()}%` : 'Get yt-dlp'}
    </button>
  </section>
{:else}
  <section class="card">
    <textarea bind:value={text} rows="3" placeholder="Paste links, one per line: videos, songs, playlists or channels" aria-label="Links to download" spellcheck="false"></textarea>

    {#if info}
      <div class="info">
        {#if info.thumbnail}
          <div class="thumb"><img src={info.thumbnail} alt="" />{#if info.duration}<span class="dur">{time(info.duration)}</span>{/if}</div>
        {/if}
        <div class="meta">
          <b class="ttl">{info.title}</b>
          <span class="t-sub">{info.uploader}</span>
          <span class="tags">{#if info.top}<span>up to {info.top}p</span>{/if}{#if info.duration && !info.thumbnail}<span>{time(info.duration)}</span>{/if}</span>
        </div>
      </div>
    {:else if looking}
      <p class="t-sub look">Looking at the link…</p>
    {:else if lookErr}
      <p class="t-sub look">{lookErr}</p>
    {/if}

    <div class="opt">
      <span class="lbl">Format</span>
      <div class="seg" role="radiogroup" aria-label="Format">
        {#each [['video', 'Video'], ['audio', 'Audio']] as const as [k, label] (k)}
          <button class="press" role="radio" aria-checked={settings.downloadKind === k} class:on={settings.downloadKind === k} onclick={() => (settings.downloadKind = k)}>{label}</button>
        {/each}
      </div>
    </div>

    <div class="opt">
      <span class="lbl">Quality</span>
      <div class="chips" role="radiogroup" aria-label="Quality">
        {#if settings.downloadKind === 'video'}
          {#each videoChoices as [q, bytes] (q)}
            <button class="chip press" role="radio" aria-checked={video === q} class:on={video === q} onclick={() => (settings.downloadVideo = q)}>
              <b>{videoLabel(q)}</b><small>{bytes ? mb(bytes) : q === 'best' && info?.top ? `${info.top}p` : info ? '' : 'up to'}</small>
            </button>
          {/each}
        {:else}
          {#each AUDIO_Q as a (a.id)}
            {@const bytes = info?.audio.find(([q]) => q === a.id)?.[1]}
            <button class="chip press" role="radio" aria-checked={settings.downloadAudio === a.id} class:on={settings.downloadAudio === a.id} title={a.hint} onclick={() => (settings.downloadAudio = a.id)}>
              <b>{a.label}</b><small>{bytes ? `≈ ${mb(bytes)}` : a.id === 'original' ? 'as published' : a.id === 'm4a' ? 'AAC' : 'needs ffmpeg'}</small>
            </button>
          {/each}
        {/if}
      </div>
    </div>
    <p class="t-sub hint">{hint}{#if !ffmpeg && choice.kind === 'video'} Without ffmpeg, video is limited to files that already include sound, and nothing gets cover art or tags.{/if}</p>

    <div class="foot">
      <div class="where">
        <span class="ellipsis" title={dest}><Icon name="FolderOpen" size={14} /> <b>{dest || '…'}</b></span>
        <span class="t-sub">{choiceLabel(choice, ffmpeg)}{size ? ` · about ${mb(size)}` : ''}</span>
      </div>
      <div class="acts">
        {#if settings.downloadFolder}<button class="btn press" onclick={() => (settings.downloadFolder = '')} title={fallback}>Use Default</button>{/if}
        <button class="btn press" onclick={pick}>Change…</button>
        <button class="btn primary press go" onclick={start} disabled={!urls.length || !dest || blocked}>
          <Icon name="DownloadSimple" size={15} weight="bold" />{urls.length > 1 ? `Download ${urls.length} Links` : 'Download'}
        </button>
      </div>
    </div>
    {#if blocked}<p class="t-sub">MP3 needs ffmpeg (below).</p>{/if}
    <Toggle bind:checked={settings.downloadToLibrary} label="Add to the library" hint="Adds this folder to your library, so finished downloads appear with the rest of your music." />
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
          {#if j.status === 'done'}<span class="ok"><Icon name="CheckCircle" size={18} weight="fill" /></span>{/if}
          {#if j.status === 'failed'}<span class="bad"><Icon name="WarningCircle" size={18} weight="fill" /></span>{/if}
          {#if j.files.length}<button class="icon-btn" aria-label="Show in {FILE_MANAGER}" title="Show in {FILE_MANAGER}" onclick={() => revealItemInDir(j.files[j.files.length - 1])}><Icon name="FolderOpen" size={16} /></button>{/if}
          {#if j.status === 'queued' || j.status === 'running'}<button class="icon-btn" aria-label="Cancel" title="Cancel" onclick={() => cancel(j.id)}><Icon name="X" size={15} weight="bold" /></button>{/if}
        </li>
      {/each}
    </ul>
  {/if}

  <section class="tools">
    <h2 class="t-h2">Tools</h2>
    <div class="line">
      <span>yt-dlp <span class="t-sub">{dl.tools.ytdlp}</span></span>
      <button class="btn press" onclick={updateYtdlp}><Icon name="ArrowsClockwise" size={14} />Check for Update</button>
    </div>
    <div class="line">
      <span>JavaScript runtime <span class="t-sub ellipsis">{runtime ?? 'not found: YouTube needs one'}</span></span>
      {#if !dl.tools.js || dl.installing === 'deno'}
        <button class="btn press" onclick={() => install('deno')} disabled={!!dl.installing}>
          {dl.installing === 'deno' ? `Downloading… ${pct()}%` : 'Get Deno (about 45 MB)'}
        </button>
      {/if}
    </div>
    <div class="line">
      <span>ffmpeg <span class="t-sub ellipsis">{dl.tools.ffmpeg ?? 'not found: needed for MP3, best video and cover art'}</span></span>
      {#if !ffmpeg && dl.tools.ffmpeg_download}
        <button class="btn press" onclick={() => install('ffmpeg')} disabled={!!dl.installing}>
          {dl.installing === 'ffmpeg' ? `Downloading… ${dl.toolTotal ? `${mb(dl.toolDone)} of ${mb(dl.toolTotal)}` : ''}` : 'Get ffmpeg (about 200 MB)'}
        </button>
      {:else if !ffmpeg}
        <code class="t-sub">brew install ffmpeg</code>
      {/if}
    </div>
  </section>
{/if}
{#if dl.error}<p class="err">{dl.error}</p>{/if}

<style>
  h1 { margin: 0 0 6px; }
  .lede { max-width: 720px; margin: 0 0 20px; line-height: 1.5; }
  .card { max-width: 760px; display: grid; gap: 14px; padding: 16px; border-radius: var(--r-lg); background: var(--fill); margin-bottom: 26px; }
  .card h2, .card p { margin: 0; }
  .card > .btn { justify-self: start; }
  textarea {
    width: 100%; resize: vertical; padding: 10px 12px; border-radius: var(--r); border: 1px solid var(--sep); background: var(--bg);
    color: var(--text); font: 12.5px/1.5 ui-monospace, 'Cascadia Mono', Consolas, monospace; user-select: text;
  }
  textarea:focus { outline: none; border-color: var(--accent); }
  .info { display: flex; gap: 14px; align-items: flex-start; min-width: 0; }
  .thumb { position: relative; flex: none; width: 168px; aspect-ratio: 16 / 9; border-radius: var(--r); overflow: hidden; background: var(--fill-2); }
  .thumb img { width: 100%; height: 100%; object-fit: cover; display: block; }
  .dur { position: absolute; right: 6px; bottom: 6px; padding: 1px 5px; border-radius: 4px; background: rgba(0, 0, 0, 0.75); color: #fff; font-size: 11px; font-weight: 600; font-variant-numeric: tabular-nums; }
  .meta { display: grid; gap: 4px; min-width: 0; }
  .ttl { font-size: 15px; line-height: 1.3; display: -webkit-box; -webkit-line-clamp: 2; line-clamp: 2; -webkit-box-orient: vertical; overflow: hidden; }
  .tags { display: flex; gap: 10px; font-size: 12px; color: var(--text-2); font-variant-numeric: tabular-nums; }
  .look { margin: 0; }
  .opt { display: grid; grid-template-columns: 76px 1fr; align-items: center; gap: 12px; }
  .lbl { font-size: 11px; font-weight: 650; letter-spacing: 0.06em; text-transform: uppercase; color: var(--text-2); }
  .seg { display: inline-flex; justify-self: start; padding: 3px; border-radius: calc(var(--r) + 3px); background: var(--fill-2); }
  .seg button { height: 28px; padding: 0 16px; border-radius: var(--r); font-weight: 600; font-size: 12.5px; color: var(--text-2); }
  .seg button.on { background: var(--bg); color: var(--text); box-shadow: 0 1px 2px rgba(0, 0, 0, 0.15); }
  .chips { display: flex; flex-wrap: wrap; gap: 6px; }
  .chip { display: grid; justify-items: start; gap: 1px; min-width: 72px; padding: 6px 12px; border-radius: var(--r); background: var(--fill-2); text-align: left; }
  .chip b { font-size: 12.5px; font-weight: 650; }
  .chip small { font-size: 10.5px; color: var(--text-2); font-variant-numeric: tabular-nums; min-height: 1.2em; }
  .chip.on { background: var(--accent); color: var(--on-accent); }
  .chip.on small { color: inherit; opacity: 0.75; }
  .hint { line-height: 1.45; }
  .foot { display: flex; align-items: center; justify-content: space-between; gap: 12px; padding-top: 12px; border-top: 1px solid var(--sep); min-width: 0; flex-wrap: wrap; }
  .where { display: grid; gap: 2px; min-width: 0; flex: 1; }
  .where .ellipsis { display: flex; align-items: center; gap: 6px; font-size: 12.5px; }
  .acts { display: flex; gap: 8px; align-items: center; flex-wrap: wrap; }
  .go { min-width: 120px; justify-content: center; }
  .btn:disabled { opacity: 0.5; }
  .row-head { display: flex; align-items: baseline; justify-content: space-between; max-width: 760px; margin-bottom: 8px; }
  .row-head h2 { margin: 0; }
  .jobs { list-style: none; margin: 0 0 26px; padding: 0; max-width: 760px; display: grid; gap: 6px; }
  .job { display: flex; align-items: center; gap: 8px; padding: 10px 10px 10px 14px; border-radius: var(--r-lg); background: var(--fill); }
  .jt { flex: 1; min-width: 0; display: grid; gap: 2px; }
  .bar { height: 3px; margin-top: 6px; border-radius: 2px; background: var(--fill-2); overflow: hidden; }
  .bar div { height: 100%; background: var(--accent); transition: width 300ms; }
  .ok { color: var(--signal-sage); display: grid; }
  .bad { color: var(--signal-veil); display: grid; }
  .tools { max-width: 760px; }
  .tools h2 { margin: 0 0 8px; }
  .line { display: flex; align-items: center; justify-content: space-between; gap: 12px; min-height: 44px; padding: 4px 8px 4px 14px; border-radius: var(--r-lg); background: var(--fill); margin-bottom: 6px; min-width: 0; }
  .line > span { min-width: 0; display: flex; gap: 8px; align-items: baseline; }
  .err { color: var(--signal-veil); max-width: 760px; }
  @media (max-width: 600px) {
    .info { flex-direction: column; }
    .thumb { width: 100%; }
    .opt { grid-template-columns: 1fr; gap: 6px; }
  }
  @media (prefers-reduced-motion: reduce) { .bar div { transition: none; } }
</style>
