<script lang="ts">
  import { untrack } from 'svelte'
  import { openUrl } from '@tauri-apps/plugin-opener'
  import Icon from '../components/Icon.svelte'
  import Slider from '../components/Slider.svelte'
  import ImportList from '../components/ImportList.svelte'
  import { LIKED, control, importPlaylist, loadPlaylists, setRemoteVolume, spotify, watchPlayer } from '../lib/spotify.svelte'
  import { nav } from '../lib/nav.svelte'
  import { plural, time } from '../lib/format'

  $effect(() => (spotify.user ? watchPlayer() : undefined))
  $effect(() => {
    if (spotify.user) untrack(() => !spotify.playlists.length && !spotify.loading && loadPlaylists())
  })

  // move the progress on between the 3-second polls
  let now = $state(performance.now())
  $effect(() => {
    const t = setInterval(() => (now = performance.now()), 500)
    return () => clearInterval(t)
  })
  const p = $derived(spotify.player)
  const progress = $derived(p ? Math.min(p.duration, p.progress + (p.playing ? (now - spotify.readAt) / 1000 : 0)) : 0)

  const rows = $derived([{ id: LIKED, name: 'Liked Songs', tracks: spotify.liked, image: null, all: true }, ...spotify.playlists])
</script>

{#if !spotify.user}
  <h1 class="t-title">Spotify</h1>
  <p class="t-sub">Sign in to Spotify in Providers to import playlists and control Spotify from here.</p>
  <button class="btn primary press" onclick={() => nav.root({ name: 'providers' })}>Open Providers</button>
{:else}
  <div class="title-row">
    <h1 class="t-title">Spotify</h1>
    <span class="t-sub">Signed in as {spotify.user}</span>
  </div>

  <section>
    <h2 class="t-h2">Playing on Spotify</h2>
    {#if p?.title}
      <div class="card">
        <div class="art">
          {#if p.image}<img src={p.image} alt="" />{:else}<Icon name="SpotifyLogo" size={40} />{/if}
        </div>
        <div class="meta">
          <div class="name ellipsis">{p.title}</div>
          <div class="t-sub ellipsis">{p.artists}{p.album ? ` — ${p.album}` : ''}</div>
          <div class="scrub">
            <span>{time(progress)}</span>
            <Slider label="Position on Spotify" value={progress} max={p.duration || 1} onchange={(v) => control({ kind: 'seek', ms: Math.round(v * 1000) })} />
            <span>-{time(Math.max(0, p.duration - progress))}</span>
          </div>
          <div class="controls">
            <button class="icon-btn press" aria-label="Previous on Spotify" onclick={() => control({ kind: 'previous' })}><Icon name="SkipBack" size={20} weight="fill" /></button>
            <button class="icon-btn press play" aria-label={p.playing ? 'Pause Spotify' : 'Play Spotify'} onclick={() => control({ kind: p.playing ? 'pause' : 'play' })}>
              {#if p.playing}<Icon name="Pause" size={24} weight="fill" />{:else}<Icon name="Play" size={24} weight="fill" />{/if}
            </button>
            <button class="icon-btn press" aria-label="Next on Spotify" onclick={() => control({ kind: 'next' })}><Icon name="SkipForward" size={20} weight="fill" /></button>
            {#if p.volume !== null}
              <div class="vol"><Icon name="SpeakerHigh" size={15} /><Slider label="Spotify volume" value={p.volume} max={100} wheel={5} onchange={setRemoteVolume} /></div>
            {/if}
          </div>
          <div class="foot t-sub">
            <span class="dev"><Icon name="SpotifyLogo" size={14} weight="fill" />Playing on {p.device}</span>
            {#if p.url}<button class="link" onclick={() => openUrl(p.url!)}>Open in Spotify <Icon name="ArrowSquareOut" size={12} /></button>{/if}
          </div>
        </div>
      </div>
    {:else}
      <p class="t-sub">Nothing is playing. Start something in the Spotify app on any of your devices to control it here.</p>
    {/if}
    {#if spotify.remoteError}<p class="err">{spotify.remoteError}</p>{/if}
  </section>

  <ImportList
    rows={rows}
    results={spotify.results}
    importing={spotify.importing}
    loading={spotify.loading}
    service="Spotify"
    onimport={importPlaylist}
    onreload={loadPlaylists}
  />
  {#if spotify.error}<p class="err">{spotify.error}</p>{/if}

  <p class="attr t-sub"><Icon name="SpotifyLogo" size={14} weight="fill" />Playlists, songs and artwork from Spotify.</p>
{/if}

<style>
  h1 { margin: 0 0 4px; }
  .title-row { display: flex; align-items: baseline; gap: 12px; }
  section { max-width: 900px; margin: 22px 0 30px; }
  h2 { margin: 0 0 10px; }
  p { margin: 0 0 12px; line-height: 1.5; }
  .card { display: flex; gap: 18px; padding: 14px; border-radius: var(--r-xl); background: var(--fill); }
  .art { flex: none; width: 132px; height: 132px; border-radius: var(--r); overflow: hidden; display: grid; place-items: center; background: var(--fill-2); color: var(--text-3); box-shadow: 0 6px 18px rgba(0, 0, 0, 0.18); }
  .art img { width: 100%; height: 100%; object-fit: cover; }
  .meta { flex: 1; min-width: 0; display: flex; flex-direction: column; gap: 4px; }
  .name { font-size: 17px; font-weight: 650; letter-spacing: -0.01em; }
  .scrub { display: grid; grid-template-columns: 42px 1fr 46px; align-items: center; gap: 8px; margin-top: 6px; font-size: 11px; font-variant-numeric: tabular-nums; color: var(--text-2); }
  .scrub span:last-child { text-align: right; }
  .controls { display: flex; align-items: center; gap: 6px; }
  .controls .play { width: 40px; height: 40px; }
  .vol { display: flex; align-items: center; gap: 8px; width: 160px; margin-left: auto; color: var(--text-2); }
  .vol :global(.slider) { flex: 1; }
  .foot { display: flex; align-items: center; justify-content: space-between; gap: 12px; font-size: 12px; }
  .dev, .attr { display: inline-flex; align-items: center; gap: 6px; }
  .link { display: inline-flex; align-items: center; gap: 4px; color: var(--accent); }
  .err { color: #e5484d; }
  .attr { margin-top: 8px; font-size: 12px; }
</style>
