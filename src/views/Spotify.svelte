<script lang="ts">
  import { untrack } from 'svelte'
  import { openUrl } from '@tauri-apps/plugin-opener'
  import SpotifyLogo from 'phosphor-svelte/lib/SpotifyLogo'
  import Play from 'phosphor-svelte/lib/Play'
  import Pause from 'phosphor-svelte/lib/Pause'
  import SkipBack from 'phosphor-svelte/lib/SkipBack'
  import SkipForward from 'phosphor-svelte/lib/SkipForward'
  import SpeakerHigh from 'phosphor-svelte/lib/SpeakerHigh'
  import ArrowSquareOut from 'phosphor-svelte/lib/ArrowSquareOut'
  import ArrowsClockwise from 'phosphor-svelte/lib/ArrowsClockwise'
  import Heart from 'phosphor-svelte/lib/Heart'
  import Slider from '../components/Slider.svelte'
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

  const rows = $derived([{ id: LIKED, name: 'Liked Songs', tracks: spotify.liked, image: null }, ...spotify.playlists])
  let picked = $state<string[]>([])
  const toggle = (id: string) => (picked = picked.includes(id) ? picked.filter((x) => x !== id) : [...picked, id])
  async function importPicked() {
    for (const r of rows.filter((r) => picked.includes(r.id))) await importPlaylist(r.id, r.name)
    picked = []
  }
</script>

{#if !spotify.user}
  <h1 class="t-title">Spotify</h1>
  <p class="t-sub">Connect your Spotify account in Settings to import playlists and control Spotify from here.</p>
  <button class="btn primary press" onclick={() => nav.root({ name: 'settings' })}>Open Settings</button>
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
          {#if p.image}<img src={p.image} alt="" />{:else}<SpotifyLogo size={40} />{/if}
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
            <button class="icon-btn press" aria-label="Previous on Spotify" onclick={() => control({ kind: 'previous' })}><SkipBack size={20} weight="fill" /></button>
            <button class="icon-btn press play" aria-label={p.playing ? 'Pause Spotify' : 'Play Spotify'} onclick={() => control({ kind: p.playing ? 'pause' : 'play' })}>
              {#if p.playing}<Pause size={24} weight="fill" />{:else}<Play size={24} weight="fill" />{/if}
            </button>
            <button class="icon-btn press" aria-label="Next on Spotify" onclick={() => control({ kind: 'next' })}><SkipForward size={20} weight="fill" /></button>
            {#if p.volume !== null}
              <div class="vol"><SpeakerHigh size={15} /><Slider label="Spotify volume" value={p.volume} max={100} wheel={5} onchange={setRemoteVolume} /></div>
            {/if}
          </div>
          <div class="foot t-sub">
            <span class="dev"><SpotifyLogo size={14} weight="fill" />Playing on {p.device}</span>
            {#if p.url}<button class="link" onclick={() => openUrl(p.url!)}>Open in Spotify <ArrowSquareOut size={12} /></button>{/if}
          </div>
        </div>
      </div>
    {:else}
      <p class="t-sub">Nothing is playing. Start something in the Spotify app on any of your devices to control it here.</p>
    {/if}
    {#if spotify.remoteError}<p class="err">{spotify.remoteError}</p>{/if}
  </section>

  <section>
    <div class="title-row">
      <h2 class="t-h2">Import Playlists</h2>
      <button class="icon-btn press" aria-label="Reload playlists" title="Reload playlists" disabled={spotify.loading} onclick={loadPlaylists}><ArrowsClockwise size={15} /></button>
    </div>
    <p class="t-sub">Makes a Sonora playlist of your own files for each Spotify playlist, so it plays in full quality. Songs you don't have are listed, never downloaded. Importing again updates it. Spotify only shares playlists you made or collaborate on.</p>
    <div class="list">
      {#each rows as r (r.id)}
        {@const res = spotify.results[r.id]}
        <div class="row">
          <input type="checkbox" checked={picked.includes(r.id)} onchange={() => toggle(r.id)} aria-label="Import {r.name}" />
          <span class="thumb">
            {#if r.image}<img src={r.image} alt="" />{:else if r.id === LIKED}<Heart size={16} weight="fill" />{:else}<SpotifyLogo size={16} />{/if}
          </span>
          <span class="txt">
            <span class="ellipsis">{r.name}</span>
            <span class="t-sub">{plural(r.tracks, 'song')}</span>
          </span>
          <span class="res t-sub">
            {#if spotify.importing === r.id}
              Importing…
            {:else if res?.error}
              <span class="err">{res.error}</span>
            {:else if res}
              <button class="link" onclick={() => res.playlist && nav.root({ name: 'playlist', id: res.playlist })}>{res.found} of {res.total} found</button>
            {/if}
          </span>
        </div>
        {#if res?.missing.length}
          <details class="missing">
            <summary class="t-sub">{plural(res.missing.length, 'song')} not in your library</summary>
            <ul>{#each res.missing as m, i (i)}<li class="t-sub">{m.title} — {m.artists.join(', ')}</li>{/each}</ul>
          </details>
        {/if}
      {:else}
        <p class="t-sub empty">{spotify.loading ? 'Loading your playlists…' : 'No playlists found.'}</p>
      {/each}
    </div>
    <div class="actions">
      <button class="btn primary press" disabled={!picked.length || !!spotify.importing} onclick={importPicked}>
        {picked.length ? `Import ${plural(picked.length, 'Playlist')}` : 'Import Playlists'}
      </button>
    </div>
    {#if spotify.error}<p class="err">{spotify.error}</p>{/if}
  </section>

  <p class="attr t-sub"><SpotifyLogo size={14} weight="fill" />Playlists, songs and artwork from Spotify.</p>
{/if}

<style>
  h1 { margin: 0 0 4px; }
  .title-row { display: flex; align-items: baseline; gap: 12px; }
  .title-row .icon-btn { align-self: center; }
  section { max-width: 900px; margin: 22px 0 30px; }
  h2 { margin: 0 0 10px; }
  p { margin: 0 0 12px; line-height: 1.5; }
  .card { display: flex; gap: 18px; padding: 14px; border-radius: 14px; background: var(--fill); }
  .art { flex: none; width: 132px; height: 132px; border-radius: 8px; overflow: hidden; display: grid; place-items: center; background: var(--fill-2); color: var(--text-3); box-shadow: 0 6px 18px rgba(0, 0, 0, 0.18); }
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
  .list { border-top: 1px solid var(--sep); }
  .row { display: flex; align-items: center; gap: 12px; padding: 6px 8px; border-bottom: 1px solid var(--sep); }
  .row:hover { background: var(--fill); }
  .row input { width: 16px; height: 16px; accent-color: var(--accent); }
  .thumb { flex: none; width: 36px; height: 36px; border-radius: 5px; overflow: hidden; display: grid; place-items: center; background: var(--fill-2); color: var(--accent); }
  .thumb img { width: 100%; height: 100%; object-fit: cover; }
  .txt { flex: 1; min-width: 0; display: grid; line-height: 18px; }
  .res { flex: none; }
  .missing { padding: 4px 8px 8px 72px; border-bottom: 1px solid var(--sep); }
  .missing summary { cursor: default; }
  .missing ul { margin: 6px 0 0; padding-left: 16px; max-height: 220px; overflow-y: auto; }
  .empty { padding: 14px 4px; }
  .actions { display: flex; gap: 10px; margin-top: 12px; }
  .btn:disabled { opacity: 0.5; }
  .err { color: #e5484d; }
  .attr { margin-top: 8px; font-size: 12px; }
</style>
