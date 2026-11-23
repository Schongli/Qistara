<script lang="ts">
  import { activate } from '../lib/touch'
  // Song list for playlists, Loved, Recently Played and Your Top. Manual playlists can reorder and remove.
  import Icon from './Icon.svelte'
  import Artwork from './Artwork.svelte'
  import Playing from './Playing.svelte'
  import Stars from './Stars.svelte'
  import { love, rate, type Track } from '../lib/library.svelte'
  import { queue } from '../lib/queue.svelte'
  import { mpv } from '../lib/player.svelte'
  import { openMenu, trackMenu } from '../lib/menu.svelte'
  import { plural, time } from '../lib/format'
  import { settings } from '../lib/settings.svelte'
  import VirtualList from './VirtualList.svelte'

  let { tracks, counts, onreorder, onremove, remote = false }: {
    tracks: Track[]
    counts?: Map<number, number> // play counts for "Your Top"
    onreorder?: (from: number, to: number) => void
    onremove?: (i: number) => void
    remote?: boolean // YouTube or server songs: no ratings or Love, they aren't in the library
  } = $props()

  let dragFrom = $state<number | null>(null)
  let over = $state<number | null>(null)
  // rows are 50 px × --density (app.css): only the ones on screen are made, so a 20,000-song playlist opens at once
  const rowHeight = $derived(50 * (settings.density === 'compact' ? 0.7778 : 1))
  // a dragged row can scroll out of view and unmount before its dragend fires
  const reset = () => (dragFrom = over = null)
</script>

<svelte:window ondragend={reset} ondrop={reset} />

<div class="table" role="list">
  <VirtualList items={tracks} {rowHeight} page overscan={8}>
    {#snippet empty()}<p class="t-sub empty">No songs here yet.</p>{/snippet}
    {#snippet row(t: Track, i: number)}
    {@const on = queue.current?.id === t.id}
    <div
      class="row"
      class:on
      class:over={over === i && dragFrom !== i}
      role="listitem"
      draggable={!!onreorder}
      ondragstart={() => (dragFrom = i)}
      ondragover={(e) => { if (onreorder) { e.preventDefault(); over = i } }}
      ondrop={(e) => { e.preventDefault(); if (dragFrom !== null && dragFrom !== i) onreorder?.(dragFrom, i); dragFrom = over = null }}
      ondragend={() => (dragFrom = over = null)}
      {...activate(() => queue.play(tracks, i))}
      oncontextmenu={(e) => openMenu(e, trackMenu([t], { from: { list: tracks, index: i } }))}
    >
      <span class="num">{#if on}<Playing paused={mpv.pause} />{:else}{counts ? i + 1 : ''}{/if}</span>
      <Artwork art={t.art} size="34px" radius="4px" shadow={false} />
      <span class="txt"><span class="ellipsis">{t.title}</span><span class="t-sub ellipsis">{t.artist}</span></span>
      <span class="album t-sub ellipsis">{t.album}</span>
      {#if counts}<span class="plays t-sub">{plural(counts.get(t.id) ?? 0, "play")}</span>{/if}
      {#if remote}
        <span></span><span></span>
      {:else}
        <span class="rate"><Stars value={t.rating} onchange={(v) => rate(t, v)} /></span>
        <button class="heart" class:loved={t.loved} aria-label={t.loved ? 'Unlove' : 'Love'} aria-pressed={t.loved} onclick={() => love(t)}>
          <Icon name="Heart" size={14} weight={t.loved ? 'fill' : 'regular'} />
        </button>
      {/if}
      <span class="tm t-sub">{time(t.duration)}</span>
      {#if onremove}
        <button class="rm" aria-label="Remove {t.title} from playlist" onclick={() => onremove(i)}><Icon name="X" size={12} weight="bold" /></button>
      {/if}
    </div>
    {/snippet}
  </VirtualList>
</div>

<style>
  .table { border-top: 1px solid var(--sep); }
  .row {
    display: grid; grid-template-columns: 28px 34px minmax(160px, 2fr) minmax(100px, 1.4fr) auto auto 24px 48px auto;
    align-items: center; gap: 12px; height: 100%; padding: 0 8px; border-radius: var(--r); border-top: 2px solid transparent;
  }
  .row:hover { background: var(--fill); }
  .row.on .txt > :first-child { color: var(--accent); }
  .row.over { border-top-color: var(--accent); }
  .num { color: var(--text-2); font-variant-numeric: tabular-nums; text-align: right; display: flex; justify-content: flex-end; }
  .txt { display: grid; min-width: 0; line-height: 17px; }
  .plays { font-variant-numeric: tabular-nums; }
  .rate { opacity: 0; transition: opacity 120ms; }
  .row:hover .rate, .rate:focus-within { opacity: 1; }
  .heart { display: grid; place-items: center; width: 26px; height: 26px; margin: 0 -6px; border-radius: var(--r-sm); color: var(--text-3); opacity: 0; }
  .heart.loved { color: var(--accent); opacity: 1; }
  .row:hover .heart { opacity: 1; }
  .tm { text-align: right; font-variant-numeric: tabular-nums; }
  .rm { width: 22px; height: 22px; border-radius: 50%; display: grid; place-items: center; color: var(--text-2); opacity: 0; }
  .row:hover .rm { opacity: 1; }
  .rm:hover { background: var(--fill-2); }
  .empty { padding: 16px 8px; }
  @media (hover: none) { .rate, .heart, .rm { opacity: 1; } }
  @media (max-width: 600px) {
    .row { grid-template-columns: 22px 34px minmax(0, 1fr) auto auto; gap: 10px; padding: 0 4px; }
    .album, .plays, .rate, .heart, .row > span:empty:not(.num) { display: none; }
  }
</style>
