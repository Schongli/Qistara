<script lang="ts">
  import Artwork from '../components/Artwork.svelte'
  import AlbumGrid from './AlbumGrid.svelte'
  import Videos from './Videos.svelte'
  import { search } from '../lib/library.svelte'
  import { nav } from '../lib/nav.svelte'
  import { queue } from '../lib/queue.svelte'
  import { openMenu, trackMenu } from '../lib/menu.svelte'
  import { time } from '../lib/format'

  const r = $derived(search(nav.query))
  const empty = $derived(!r.songs.length && !r.albums.length && !r.artists.length && !r.videos.length)
</script>

{#if !nav.query.trim()}
  <p class="t-sub">Search your songs, albums, artists and videos.</p>
{:else if empty}
  <h1 class="t-title">No results</h1>
  <p class="t-sub">Nothing in your library matches “{nav.query}”.</p>
{:else}
  {#if r.artists.length}
    <h2 class="t-h2">Artists</h2>
    <div class="artists" data-scroll>
      {#each r.artists as a (a.name)}
        <button class="artist press" onclick={() => nav.go({ name: 'artists', artist: a.name })}>
          <Artwork art={a.art} size="96px" radius="50%" />
          <span class="ellipsis">{a.name}</span>
        </button>
      {/each}
    </div>
  {/if}
  {#if r.songs.length}
    <h2 class="t-h2">Songs</h2>
    <div class="songs">
      {#each r.songs.slice(0, 12) as t, i (t.id)}
        <button class="song" ondblclick={() => queue.play(r.songs, i)} oncontextmenu={(e) => openMenu(e, trackMenu([t]))}>
          <Artwork art={t.art} size="40px" radius="4px" shadow={false} />
          <span class="txt"><span class="ellipsis">{t.title}</span><span class="t-sub ellipsis">{t.artist} — {t.album}</span></span>
          <span class="t-sub">{time(t.duration)}</span>
        </button>
      {/each}
    </div>
  {/if}
  {#if r.albums.length}
    <h2 class="t-h2">Albums</h2>
    <AlbumGrid albums={r.albums} />
  {/if}
  {#if r.videos.length}
    <div class="vids"><Videos videos={r.videos} title="Videos" /></div>
  {/if}
{/if}

<style>
  h2 { margin: 28px 0 14px; }
  h2:first-child { margin-top: 0; }
  .artists { display: flex; gap: 20px; overflow-x: auto; padding-bottom: 6px; }
  .artist { display: grid; justify-items: center; gap: 8px; width: 110px; flex: none; }
  .artist span { max-width: 100%; }
  .songs { display: grid; grid-template-columns: repeat(auto-fill, minmax(320px, 1fr)); gap: 2px 20px; }
  .song { display: flex; align-items: center; gap: 12px; padding: 6px 8px; border-radius: var(--r); text-align: left; min-width: 0; }
  .song:hover { background: var(--fill); }
  .txt { flex: 1; min-width: 0; display: grid; }
  .vids { margin-top: 28px; }
</style>
