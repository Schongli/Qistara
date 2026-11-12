<script lang="ts">
  import AlbumGrid from './AlbumGrid.svelte'
  import Artwork from '../components/Artwork.svelte'
  import { lib } from '../lib/library.svelte'
  import { nav } from '../lib/nav.svelte'
  import { plural } from '../lib/format'

  let { genre }: { genre?: string } = $props()
  const g = $derived(genre ? lib.genres.find((x) => x.name === genre) : null)
</script>

{#if g}
  <AlbumGrid albums={g.albums} title={g.name} />
{:else}
  <h1 class="t-title">Genres</h1>
  <div class="grid">
    {#each lib.genres as gn (gn.name)}
      <button class="card press" onclick={() => nav.go({ name: 'genres', genre: gn.name })}>
        <div class="stack">
          {#each gn.albums.slice(0, 3) as a, i (a.key)}
            <span style:--i={i}><Artwork art={a.art} size="72px" radius="6px" /></span>
          {/each}
        </div>
        <div class="name ellipsis">{gn.name}</div>
        <div class="t-sub">{plural(gn.albums.length, 'album')}</div>
      </button>
    {/each}
  </div>
{/if}

<style>
  h1 { margin: 0 0 20px; }
  .grid { display: grid; grid-template-columns: repeat(auto-fill, minmax(200px, 1fr)); gap: 16px; }
  .card { padding: 16px; border-radius: 12px; background: var(--fill); text-align: left; min-width: 0; }
  .card:hover { background: var(--fill-2); }
  .stack { position: relative; height: 80px; margin-bottom: 12px; }
  .stack span { position: absolute; left: calc(var(--i) * 40px); top: calc(var(--i) * 3px); z-index: calc(3 - var(--i)); }
  .name { font-weight: 600; }
</style>
