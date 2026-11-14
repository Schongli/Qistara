<script lang="ts">
  // Settings › Lyrics: the sources in the order they are asked. Drag a row, or focus its grip and press ↑/↓.
  import { tick } from 'svelte'
  import Icon from './Icon.svelte'
  import Switch from './Switch.svelte'
  import { settings } from '../lib/settings.svelte'
  import { SOURCES } from '../lib/lyricsources'
  import { move } from '../lib/queue'

  let list = $state<HTMLOListElement>()
  let dragFrom = $state<number | null>(null)
  let over = $state<number | null>(null)

  async function reorder(from: number, to: number, refocus = false) {
    if (to < 0 || to >= settings.lyricsSources.length || from === to) return
    settings.lyricsSources = move(settings.lyricsSources, from, to)
    // a moved row loses focus as it moves in the DOM: give it back
    if (refocus) await tick(), list?.children[to]?.querySelector<HTMLElement>('.grip')?.focus()
  }
</script>

<ol class="sources" bind:this={list} aria-label="Lyrics sources, asked in this order">
  {#each settings.lyricsSources as s, i (s.id)}
    <li
      class="src"
      class:off={!s.on}
      class:over={over === i && dragFrom !== i}
      draggable="true"
      ondragstart={(e) => { dragFrom = i; e.dataTransfer!.effectAllowed = 'move' }}
      ondragover={(e) => { e.preventDefault(); over = i }}
      ondragleave={() => over === i && (over = null)}
      ondrop={(e) => { e.preventDefault(); if (dragFrom !== null) reorder(dragFrom, dragFrom < i ? i - 1 : i); dragFrom = over = null }}
      ondragend={() => (dragFrom = over = null)}
    >
      <button
        class="grip"
        aria-label="Move {SOURCES[s.id].label}, {i + 1} of {settings.lyricsSources.length}"
        aria-keyshortcuts="ArrowUp ArrowDown"
        title="Drag, or press ↑ or ↓"
        onkeydown={(e) => {
          if (e.key !== 'ArrowUp' && e.key !== 'ArrowDown') return
          e.preventDefault()
          reorder(i, e.key === 'ArrowUp' ? i - 1 : i + 1, true)
        }}
      ><Icon name="DotsSixVertical" size={14} weight="bold" /></button>
      <span class="n" aria-hidden="true">{i + 1}</span>
      <span class="txt">
        <span>{SOURCES[s.id].label}</span>
        <span class="hint">{SOURCES[s.id].hint}</span>
      </span>
      <Switch bind:checked={s.on} label="Use {SOURCES[s.id].label}" />
    </li>
  {/each}
</ol>

<style>
  .sources { list-style: none; margin: 0; padding: 0; }
  .src {
    display: flex; align-items: center; gap: 10px; min-height: 52px; padding: 8px 14px 8px 8px;
    border-bottom: 1px solid var(--sep); border-top: 2px solid transparent; cursor: grab;
  }
  .src:last-child { border-bottom: 0; }
  .src:active { cursor: grabbing; }
  .src.over { border-top-color: var(--accent); }
  .grip { color: var(--text-3); display: grid; place-items: center; width: 22px; height: 28px; border-radius: var(--r-sm); cursor: grab; }
  .grip:hover, .grip:focus-visible { color: var(--text); background: var(--fill); }
  .n { width: 16px; text-align: center; color: var(--text-2); font-variant-numeric: tabular-nums; font-size: 12px; }
  .txt { flex: 1; display: grid; gap: 2px; min-width: 0; }
  .hint { font-size: 12px; color: var(--text-2); line-height: 1.4; }
  .off .txt > span:first-child { color: var(--text-2); }
</style>
