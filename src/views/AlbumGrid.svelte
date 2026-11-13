<script lang="ts">
  import Play from 'phosphor-svelte/lib/Play'
  import Artwork from '../components/Artwork.svelte'
  import type { Album } from '../lib/library.svelte'
  import { nav } from '../lib/nav.svelte'
  import { queue } from '../lib/queue.svelte'
  import { openMenu, trackMenu } from '../lib/menu.svelte'

  // row: one line that scrolls sideways (Home)
  let { albums, title = '', showArtist = true, row = false }: { albums: Album[]; title?: string; showArtist?: boolean; row?: boolean } = $props()
</script>

{#if title}<h1 class="t-title">{title}</h1>{/if}
<div class="grid" class:row>
  {#each albums as a (a.key)}
    <div class="tile" oncontextmenu={(e) => openMenu(e, trackMenu(a.tracks, a))} role="listitem">
      <div class="cover">
        <button class="open press" aria-label="Open {a.title}" onclick={() => nav.go({ name: 'album', key: a.key })}>
          <Artwork art={a.art} />
        </button>
        <button class="play press" aria-label="Play {a.title}" onclick={() => queue.play(a.tracks)}><Play size={16} weight="fill" /></button>
      </div>
      <button class="name ellipsis" onclick={() => nav.go({ name: 'album', key: a.key })}>{a.title}</button>
      {#if showArtist}
        <button class="t-sub ellipsis artist" onclick={() => nav.go({ name: 'artists', artist: a.artist })}>{a.artist}</button>
      {:else}
        <div class="t-sub">{a.year ?? ''}</div>
      {/if}
    </div>
  {/each}
</div>

<style>
  h1 { margin: 0 0 20px; }
  .grid { display: grid; grid-template-columns: repeat(auto-fill, minmax(164px, 1fr)); gap: 26px 22px; }
  .grid.row { grid-template-columns: none; grid-auto-flow: column; grid-auto-columns: 164px; overflow-x: auto; padding-bottom: 8px; scroll-snap-type: x proximity; }
  .row .tile { scroll-snap-align: start; content-visibility: visible; }
  .tile { min-width: 0; content-visibility: auto; contain-intrinsic-size: auto 230px; }
  .cover { position: relative; margin-bottom: 8px; }
  .open { display: block; width: 100%; border-radius: var(--r-art); }
  .play {
    position: absolute; left: 10px; bottom: 10px; width: 30px; height: 30px; border-radius: 50%;
    display: grid; place-items: center; color: #fff; background: rgba(0, 0, 0, 0.45);
    backdrop-filter: blur(12px); opacity: 0; transform: scale(0.9);
    transition: opacity 140ms, transform 140ms var(--ease-out), background-color 120ms;
  }
  .play:hover { background: var(--accent); }
  .tile:hover .play, .play:focus-visible { opacity: 1; transform: none; }
  .name { display: block; width: 100%; text-align: left; font-weight: 550; line-height: 17px; }
  .artist { display: block; width: 100%; text-align: left; }
  .artist:hover, .name:hover { text-decoration: underline; text-underline-offset: 2px; }
</style>
