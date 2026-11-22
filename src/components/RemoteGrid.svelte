<script lang="ts">
  // Cards for things that live on a service, not in the library (YT Music albums, playlists, artists, songs):
  // the AlbumGrid look, with open and play handed in.
  import Icon from './Icon.svelte'
  import Artwork from './Artwork.svelte'
  import type { Item } from '../lib/ytmusic.svelte'
  import { openMenu, ytmMenu } from '../lib/menu.svelte'

  let { items, row = false, onopen, onplay }: { items: Item[]; row?: boolean; onopen: (i: Item) => void; onplay: (i: Item) => void } = $props()
</script>

<div class="grid" class:row data-scroll={row || undefined} role="list">
  {#each items as it (it.kind + it.id)}
    <div class="tile" class:round={it.kind === 'artist'} role="listitem" oncontextmenu={(e) => openMenu(e, ytmMenu(it, onopen))}>
      <div class="cover">
        <button class="open press" aria-label="Open {it.title}" onclick={() => onopen(it)}>
          <Artwork art={it.thumb} radius={it.kind === 'artist' ? '50%' : undefined} />
        </button>
        {#if it.kind !== 'artist'}
          <button class="play press" aria-label="Play {it.title}" onclick={() => onplay(it)}><Icon name="Play" size={16} weight="fill" /></button>
        {/if}
      </div>
      <button class="name ellipsis" onclick={() => onopen(it)}>{it.title}</button>
      <div class="t-sub ellipsis">{it.subtitle}</div>
    </div>
  {/each}
</div>

<style>
  .grid { display: grid; grid-template-columns: repeat(auto-fill, minmax(164px, 1fr)); gap: 26px 22px; }
  .grid.row { grid-template-columns: none; grid-auto-flow: column; grid-auto-columns: 164px; overflow-x: auto; padding-bottom: 8px; scroll-snap-type: x proximity; }
  .row .tile { scroll-snap-align: start; }
  .tile { min-width: 0; }
  .round .name, .round .t-sub { text-align: center; }
  .cover { position: relative; margin-bottom: 8px; }
  .open { display: block; width: 100%; border-radius: var(--r-art); }
  .round .open { border-radius: 50%; }
  .play {
    position: absolute; left: 10px; bottom: 10px; width: 30px; height: 30px; border-radius: 50%;
    display: grid; place-items: center; color: #fff; background: rgba(0, 0, 0, 0.45);
    backdrop-filter: blur(12px); opacity: 0; transform: scale(0.9);
    transition: opacity 140ms, transform 140ms var(--ease-out), background-color 120ms;
  }
  .play:hover { background: var(--accent); }
  .tile:hover .play, .play:focus-visible { opacity: 1; transform: none; }
  @media (hover: none) { .play { opacity: 1; transform: none; } }
  .name { display: block; width: 100%; text-align: left; font-weight: 550; line-height: 17px; }
  .name:hover { text-decoration: underline; text-underline-offset: 2px; }
  @media (prefers-reduced-motion: reduce) { .play { transition: none; } }
</style>
