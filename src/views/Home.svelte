<script lang="ts">
  // Home: shelves made from your own library and listening; nothing is fetched or recommended.
  import AlbumGrid from './AlbumGrid.svelte'
  import { lib, type Album } from '../lib/library.svelte'
  import { pins } from '../lib/pins.svelte'
  import { nav, type Route } from '../lib/nav.svelte'

  const hour = new Date().getHours()
  const greeting = hour < 5 ? 'Good night' : hour < 12 ? 'Good morning' : hour < 18 ? 'Good afternoon' : 'Good evening'

  const last = (a: Album) => a.tracks.reduce((m, t) => Math.max(m, t.lastPlayed ?? 0), 0)
  const plays = (a: Album) => a.tracks.reduce((s, t) => s + t.plays, 0)
  const top = (score: (a: Album) => number) =>
    lib.albums.map((a) => [a, score(a)] as const).filter(([, s]) => s > 0).sort((x, y) => y[1] - x[1]).slice(0, 18).map(([a]) => a)

  const shelves = $derived(
    [
      { title: 'Pinned', albums: pins.items.flatMap((p) => lib.albums.find((a) => a.key === p.key) ?? []) },
      { title: 'Jump back in', albums: top(last), more: { name: 'recentPlayed' } as Route },
      { title: 'Recently added', albums: lib.recent.slice(0, 18), more: { name: 'recent' } as Route },
      { title: 'Most played', albums: top(plays), more: { name: 'top' } as Route },
    ].filter((s) => s.albums.length),
  )
</script>

<h1 class="t-title">{greeting}</h1>
{#each shelves as s (s.title)}
  <section>
    <div class="head">
      <h2 class="t-h2">{s.title}</h2>
      {#if s.more}<button class="see press" onclick={() => s.more && nav.root(s.more)}>See all</button>{/if}
    </div>
    <AlbumGrid albums={s.albums} row />
  </section>
{:else}
  <p class="t-sub">Play something and it shows up here.</p>
{/each}

<style>
  h1 { margin: 0 0 22px; }
  section { margin-bottom: 30px; }
  .head { display: flex; align-items: baseline; justify-content: space-between; margin-bottom: 12px; }
  h2 { margin: 0; }
  .see { font-size: 12px; font-weight: 600; color: var(--text-2); }
  .see:hover { color: var(--text); }
</style>
