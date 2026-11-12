<script lang="ts">
  import Play from 'phosphor-svelte/lib/Play'
  import Shuffle from 'phosphor-svelte/lib/Shuffle'
  import Artwork from '../components/Artwork.svelte'
  import AlbumGrid from './AlbumGrid.svelte'
  import { lib } from '../lib/library.svelte'
  import { queue } from '../lib/queue.svelte'
  import { plural } from '../lib/format'

  let { artist }: { artist?: string } = $props()
  let picked = $state<string | undefined>()
  const name = $derived(picked ?? artist ?? lib.artists[0]?.name)
  const a = $derived(lib.artists.find((x) => x.name === name))
  const tracks = $derived(a?.albums.flatMap((al) => al.tracks) ?? [])
  $effect(() => {
    artist
    picked = undefined // a new Go to Artist wins over the last click
  })
</script>

<div class="split">
  <div class="list" role="listbox" aria-label="Artists">
    {#each lib.artists as ar (ar.name)}
      <button class="item press" class:sel={ar.name === name} role="option" aria-selected={ar.name === name} onclick={() => (picked = ar.name)}>
        <Artwork art={ar.art} size="32px" radius="50%" shadow={false} />
        <span class="ellipsis">{ar.name}</span>
      </button>
    {/each}
  </div>
  <div class="detail">
    {#if a}
      <h1 class="t-title">{a.name}</h1>
      <p class="t-sub">{plural(a.albums.length, 'album')}, {plural(tracks.length, 'song')}</p>
      <div class="actions">
        <button class="btn primary press" onclick={() => queue.play(tracks)}><Play size={14} weight="fill" />Play</button>
        <button class="btn press" onclick={() => queue.play(tracks, Math.floor(Math.random() * tracks.length), true)}><Shuffle size={14} weight="bold" />Shuffle</button>
      </div>
      <AlbumGrid albums={a.albums} showArtist={false} />
    {/if}
  </div>
</div>

<style>
  .split { position: absolute; inset: 0; display: grid; grid-template-columns: 250px 1fr; }
  .list { overflow-y: auto; padding: calc(var(--bar-h) + 12px) 8px 16px 14px; border-right: 1px solid var(--sep); }
  .item { display: flex; align-items: center; gap: 10px; width: 100%; height: 44px; padding: 0 8px; border-radius: 8px; text-align: left; content-visibility: auto; contain-intrinsic-size: auto 44px; }
  .item:hover { background: var(--fill); }
  .item.sel { background: var(--fill-2); font-weight: 550; }
  .detail { overflow-y: auto; padding: calc(var(--bar-h) + 24px) 28px 40px; }
  h1 { margin: 0; }
  p { margin: 4px 0 14px; }
  .actions { display: flex; gap: 10px; margin-bottom: 28px; }
</style>
