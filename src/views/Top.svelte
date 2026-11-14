<script lang="ts" module>
  type Period = 'month' | 'year' | 'all'
  let period = $state<Period>('month') // kept across visits
</script>

<script lang="ts">
  // "Your Top": most played songs, artists and albums for a period, from the play history.
  import { invoke } from '@tauri-apps/api/core'
  import TrackTable from '../components/TrackTable.svelte'
  import Artwork from '../components/Artwork.svelte'
  import { albumArtist, albumKey, lib, type Track } from '../lib/library.svelte'
  import { nav } from '../lib/nav.svelte'
  import { plural } from '../lib/format'

  let rows = $state<[number, number][]>([])

  const since = (p: Period) => {
    const d = new Date()
    if (p === 'all') return 0
    return Math.floor(new Date(d.getFullYear(), p === 'month' ? d.getMonth() : 0, 1).getTime() / 1000)
  }
  $effect(() => {
    const s = since(period)
    lib.tracks // refresh when plays are counted
    invoke<[number, number][]>('top', { since: s }).then((r) => (rows = r))
  })

  const counts = $derived(new Map(rows))
  const songs = $derived(rows.map(([id]) => lib.byId.get(id)).filter((t): t is Track => !!t))
  function tally(key: (t: Track) => string) {
    const m = new Map<string, { n: number; t: Track }>()
    for (const t of songs) {
      const k = key(t)
      const e = m.get(k)
      m.set(k, { n: (e?.n ?? 0) + (counts.get(t.id) ?? 0), t: e?.t ?? t })
    }
    return [...m.values()].sort((a, b) => b.n - a.n).slice(0, 8)
  }
  const artists = $derived(tally(albumArtist))
  const albums = $derived(tally(albumKey))
  const total = $derived(rows.reduce((s, [, n]) => s + n, 0))
</script>

<h1 class="t-title">Your Top</h1>
<div class="seg" role="tablist" aria-label="Period">
  {#each [['month', 'This Month'], ['year', 'This Year'], ['all', 'All Time']] as [p, label] (p)}
    <button role="tab" aria-selected={period === p} class:sel={period === p} onclick={() => (period = p as Period)}>{label}</button>
  {/each}
</div>

{#if !rows.length}
  <p class="t-sub">Nothing yet. A song counts once you have heard half of it (or 4 minutes).</p>
{:else}
  <p class="t-sub">{plural(total, 'play')} of {plural(songs.length, 'song')}.</p>
  <div class="cards">
    <section>
      <h2 class="t-h2">Top Artists</h2>
      {#each artists as a, i (i)}
        <button class="line" onclick={() => nav.go({ name: 'artists', artist: albumArtist(a.t) })}>
          <span class="rank">{i + 1}</span><Artwork art={a.t.art} size="40px" radius="50%" shadow={false} />
          <span class="ellipsis grow">{albumArtist(a.t)}</span><span class="t-sub">{plural(a.n, 'play')}</span>
        </button>
      {/each}
    </section>
    <section>
      <h2 class="t-h2">Top Albums</h2>
      {#each albums as a, i (i)}
        <button class="line" onclick={() => nav.go({ name: 'album', key: albumKey(a.t) })}>
          <span class="rank">{i + 1}</span><Artwork art={a.t.art} size="40px" radius="4px" shadow={false} />
          <span class="ellipsis grow">{a.t.album || 'Unknown Album'}<span class="t-sub"> — {albumArtist(a.t)}</span></span><span class="t-sub">{plural(a.n, 'play')}</span>
        </button>
      {/each}
    </section>
  </div>
  <h2 class="t-h2 songs">Top Songs</h2>
  <TrackTable tracks={songs.slice(0, 100)} {counts} />
{/if}

<style>
  h1 { margin: 0 0 14px; }
  .seg { display: inline-flex; padding: 2px; border-radius: var(--r); background: var(--fill-2); margin-bottom: 16px; }
  .seg button { height: 26px; padding: 0 14px; border-radius: var(--r-sm); font-weight: 550; color: var(--text-2); }
  .seg button.sel { background: var(--bg); color: var(--text); box-shadow: 0 1px 3px rgba(0, 0, 0, 0.12); }
  .cards { display: grid; grid-template-columns: repeat(auto-fit, minmax(320px, 1fr)); gap: 20px; margin: 16px 0 28px; }
  section { padding: 14px; border-radius: var(--r-lg); background: var(--fill); }
  h2 { margin: 0 0 8px; }
  .songs { margin-bottom: 12px; }
  .line { display: flex; align-items: center; gap: 12px; width: 100%; padding: 6px; border-radius: var(--r); text-align: left; }
  .line:hover { background: var(--fill-2); }
  .rank { width: 18px; text-align: right; color: var(--text-2); font-variant-numeric: tabular-nums; font-weight: 600; }
  .grow { flex: 1; min-width: 0; }
</style>
