<script lang="ts">
  // Song list for playlists, Loved, Recently Played and Your Top. Manual playlists can reorder and remove.
  import Heart from 'phosphor-svelte/lib/Heart'
  import X from 'phosphor-svelte/lib/X'
  import Artwork from './Artwork.svelte'
  import Playing from './Playing.svelte'
  import Stars from './Stars.svelte'
  import { love, rate, type Track } from '../lib/library.svelte'
  import { queue } from '../lib/queue.svelte'
  import { mpv } from '../lib/player.svelte'
  import { openMenu, trackMenu } from '../lib/menu.svelte'
  import { plural, time } from '../lib/format'

  let { tracks, counts, onreorder, onremove }: {
    tracks: Track[]
    counts?: Map<number, number> // play counts for "Your Top"
    onreorder?: (from: number, to: number) => void
    onremove?: (i: number) => void
  } = $props()

  let dragFrom = $state<number | null>(null)
  let over = $state<number | null>(null)
</script>

<div class="table" role="list">
  {#each tracks as t, i (i + ':' + t.id)}
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
      ondblclick={() => queue.play(tracks, i)}
      oncontextmenu={(e) => openMenu(e, trackMenu([t]))}
    >
      <span class="num">{#if on}<Playing paused={mpv.pause} />{:else}{counts ? i + 1 : ''}{/if}</span>
      <Artwork art={t.art} size="34px" radius="4px" shadow={false} />
      <span class="txt"><span class="ellipsis">{t.title}</span><span class="t-sub ellipsis">{t.artist}</span></span>
      <span class="album t-sub ellipsis">{t.album}</span>
      {#if counts}<span class="plays t-sub">{plural(counts.get(t.id) ?? 0, "play")}</span>{/if}
      <span class="rate"><Stars value={t.rating} onchange={(v) => rate(t, v)} /></span>
      <button class="heart" class:loved={t.loved} aria-label={t.loved ? 'Unlove' : 'Love'} aria-pressed={t.loved} onclick={() => love(t)}>
        <Heart size={14} weight={t.loved ? 'fill' : 'regular'} />
      </button>
      <span class="tm t-sub">{time(t.duration)}</span>
      {#if onremove}
        <button class="rm" aria-label="Remove {t.title} from playlist" onclick={() => onremove(i)}><X size={12} weight="bold" /></button>
      {/if}
    </div>
  {:else}
    <p class="t-sub empty">No songs here yet.</p>
  {/each}
</div>

<style>
  .table { border-top: 1px solid var(--sep); }
  .row {
    display: grid; grid-template-columns: 28px 34px minmax(160px, 2fr) minmax(100px, 1.4fr) auto auto 24px 48px auto;
    align-items: center; gap: 12px; height: 50px; padding: 0 8px; border-radius: 8px; border-top: 2px solid transparent;
  }
  .row:hover { background: var(--fill); }
  .row.on .txt > :first-child { color: var(--accent); }
  .row.over { border-top-color: var(--accent); }
  .num { color: var(--text-2); font-variant-numeric: tabular-nums; text-align: right; display: flex; justify-content: flex-end; }
  .txt { display: grid; min-width: 0; line-height: 17px; }
  .plays { font-variant-numeric: tabular-nums; }
  .rate { opacity: 0; transition: opacity 120ms; }
  .row:hover .rate, .rate:focus-within { opacity: 1; }
  .heart { display: grid; place-items: center; width: 26px; height: 26px; margin: 0 -6px; border-radius: 6px; color: var(--text-3); opacity: 0; }
  .heart.loved { color: var(--accent); opacity: 1; }
  .row:hover .heart { opacity: 1; }
  .tm { text-align: right; font-variant-numeric: tabular-nums; }
  .rm { width: 22px; height: 22px; border-radius: 50%; display: grid; place-items: center; color: var(--text-2); opacity: 0; }
  .row:hover .rm { opacity: 1; }
  .rm:hover { background: var(--fill-2); }
  .empty { padding: 16px 8px; }
</style>
