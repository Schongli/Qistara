<script lang="ts">
  import Icon from '../components/Icon.svelte'
  import Artwork from '../components/Artwork.svelte'
  import AlbumGrid from './AlbumGrid.svelte'
  import { lib } from '../lib/library.svelte'
  import { nav } from '../lib/nav.svelte'
  import { groupMenu, openMenu } from '../lib/menu.svelte'
  import { queue } from '../lib/queue.svelte'
  import { plural } from '../lib/format'

  // the picked artist is part of the route, so Back from one of its albums returns to it
  let { artist }: { artist?: string } = $props()
  const name = $derived(artist ?? lib.artists[0]?.name)
  const a = $derived(lib.artists.find((x) => x.name === name))
  const tracks = $derived(a?.albums.flatMap((al) => al.tracks) ?? [])
</script>

<div class="split">
  <div class="list" data-scroll role="listbox" aria-label="Artists">
    {#each lib.artists as ar (ar.name)}
      <button class="item press" class:sel={ar.name === name} role="option" aria-selected={ar.name === name} onclick={() => nav.replace({ name: 'artists', artist: ar.name })} oncontextmenu={(e) => openMenu(e, groupMenu(ar.albums.flatMap((al) => al.tracks), () => nav.replace({ name: 'artists', artist: ar.name }), ar.name))}>
        <Artwork art={ar.art} size="32px" radius="50%" shadow={false} />
        <span class="ellipsis">{ar.name}</span>
      </button>
    {/each}
  </div>
  <div class="detail" data-scroll>
    {#if a}
      <h1 class="t-title">{a.name}</h1>
      <p class="t-sub">{plural(a.albums.length, 'album')}, {plural(tracks.length, 'song')}</p>
      <div class="actions">
        <button class="btn primary press" onclick={() => queue.play(tracks)}><Icon name="Play" size={14} weight="fill" />Play</button>
        <button class="btn press" onclick={() => queue.play(tracks, Math.floor(Math.random() * tracks.length), true)}><Icon name="Shuffle" size={14} weight="bold" />Shuffle</button>
      </div>
      <AlbumGrid albums={a.albums} showArtist={false} />
    {/if}
  </div>
</div>

<style>
  .split { position: absolute; inset: 0; display: grid; grid-template-columns: 250px 1fr; }
  /* a phone-width window: the list on top, what's chosen below */
  @media (max-width: 600px) { .split { grid-template-columns: 1fr; grid-template-rows: minmax(0, 38%) 1fr; } .list { border-right: 0; border-bottom: 1px solid var(--sep); } .detail { padding: 16px; } }
  .list { overflow-y: auto; padding: calc(var(--bar-h) + 12px) 8px 16px 14px; border-right: 1px solid var(--sep); }
  .item { display: flex; align-items: center; gap: 10px; width: 100%; height: 44px; padding: 0 8px; border-radius: var(--r); text-align: left; content-visibility: auto; contain-intrinsic-size: auto 44px; }
  .item:hover { background: var(--fill); }
  .item.sel { background: var(--fill-2); font-weight: 550; }
  .detail { overflow-y: auto; padding: calc(var(--bar-h) + 24px) 28px 40px; }
  h1 { margin: 0; }
  p { margin: 4px 0 14px; }
  .actions { display: flex; gap: 10px; margin-bottom: 28px; }
</style>
