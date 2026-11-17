<script lang="ts">
  import Icon from './Icon.svelte'
  import Artwork from './Artwork.svelte'
  import Playing from './Playing.svelte'
  import { queue } from '../lib/queue.svelte'
  import { mpv } from '../lib/player.svelte'
  import { openMenu, trackMenu } from '../lib/menu.svelte'
  import VirtualList from './VirtualList.svelte'
  import type { Track } from '../lib/library.svelte'

  let { dark = false }: { dark?: boolean } = $props()
  let dragFrom = $state<number | null>(null)
  let over = $state<number | null>(null)
  const base = $derived(queue.index + 1)
  // Up Next can be the whole library (play from Songs): only the rows on screen are made
  const ROW = 52
  const reset = () => (dragFrom = over = null)
</script>

<svelte:window ondragend={reset} ondrop={reset} />

<div class="q" class:dark>
  {#if queue.current}
    <div class="label">Now Playing</div>
    <div class="item cur">
      <Artwork art={queue.current.art} size="40px" radius="4px" shadow={false} />
      <span class="txt"><span class="ellipsis">{queue.current.title}</span><span class="sub ellipsis">{queue.current.artist}</span></span>
      <Playing paused={mpv.pause} />
    </div>
  {/if}

  <div class="label row">
    <span>Up Next {#if queue.shuffle}<span class="sub">· Shuffled</span>{/if}</span>
    <span class="acts">
      <button class="auto" class:on={queue.autoplay} aria-pressed={queue.autoplay} title="Autoplay: keep playing similar songs when Up Next ends" onclick={() => queue.toggleAutoplay()}>
        <Icon name="Infinity" size={14} weight="bold" /> Autoplay
      </button>
      {#if queue.upNext.length}<button class="clear" onclick={() => queue.clearUpNext()}>Clear</button>{/if}
    </span>
  </div>
  <VirtualList items={queue.upNext} rowHeight={ROW} page overscan={6}>
    {#snippet empty()}<p class="sub empty">{queue.current ? 'Nothing up next. Use Play Next or Play Later on any song or album.' : 'Play something to fill Up Next.'}</p>{/snippet}
    {#snippet row(t: Track, i: number)}
    {@const at = base + i}
    <div
      class="item"
      class:over={over === at && dragFrom !== at}
      role="listitem"
      draggable="true"
      ondragstart={(e) => { dragFrom = at; e.dataTransfer!.effectAllowed = 'move' }}
      ondragover={(e) => { e.preventDefault(); over = at }}
      ondragleave={() => over === at && (over = null)}
      ondrop={(e) => { e.preventDefault(); if (dragFrom !== null && dragFrom !== at) queue.reorder(dragFrom, at); dragFrom = over = null }}
      ondragend={() => (dragFrom = over = null)}
      ondblclick={() => queue.jump(at)}
      oncontextmenu={(e) => openMenu(e, trackMenu([t]))}
    >
      <span class="grip"><Icon name="DotsSixVertical" size={14} weight="bold" /></span>
      <Artwork art={t.art} size="40px" radius="4px" shadow={false} />
      <span class="txt"><span class="ellipsis">{t.title}</span><span class="sub ellipsis">{t.artist}</span></span>
      <button class="rm" aria-label="Remove {t.title}" onclick={() => queue.remove(at)}><Icon name="X" size={12} weight="bold" /></button>
    </div>
    {/snippet}
  </VirtualList>
</div>

<style>
  .q { display: grid; gap: 1px; }
  .label { font-weight: 650; font-size: 13px; padding: 14px 8px 6px; }
  .label.row { display: flex; justify-content: space-between; align-items: center; }
  .clear { color: var(--accent); font-weight: 550; }
  .acts { display: flex; align-items: center; gap: 12px; }
  .auto { display: inline-flex; align-items: center; gap: 4px; height: 22px; padding: 0 8px; border-radius: 11px; font-size: 11.5px; font-weight: 600; color: var(--text-2); background: var(--fill); }
  .auto.on { color: var(--on-accent); background: var(--accent); }
  .item { display: flex; align-items: center; gap: 10px; height: 52px; box-sizing: border-box; padding: 6px 8px; border-radius: var(--r); min-width: 0; border-top: 2px solid transparent; }
  .item:hover { background: var(--fill); }
  .item.over { border-top-color: var(--accent); }
  .txt { flex: 1; min-width: 0; display: grid; line-height: 17px; }
  .sub { font-size: 12px; color: var(--text-2); }
  .grip { color: var(--text-3); cursor: grab; opacity: 0; margin-right: -6px; }
  .item:hover .grip { opacity: 1; }
  .rm { width: 22px; height: 22px; border-radius: 50%; display: grid; place-items: center; color: var(--text-2); opacity: 0; }
  .item:hover .rm, .rm:focus-visible { opacity: 1; }
  .rm:hover { background: var(--fill-2); }
  .empty { padding: 4px 8px; margin: 0; line-height: 1.5; }
  .dark { --fill: rgba(255, 255, 255, 0.08); --fill-2: rgba(255, 255, 255, 0.14); --text: #fff; --text-2: rgba(255, 255, 255, 0.62); --text-3: rgba(255, 255, 255, 0.4); color: #fff; }
</style>
