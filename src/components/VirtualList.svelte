<script lang="ts" generics="T">
  // Renders only the visible rows of a long fixed-height list (50k songs stay smooth).
  import type { Snippet } from 'svelte'

  let { items, rowHeight, row, header, padTop = 0, overscan = 10 }: {
    items: T[]
    rowHeight: number
    row: Snippet<[T, number]>
    header?: Snippet
    padTop?: number
    overscan?: number
  } = $props()

  let el: HTMLDivElement
  let top = $state(0)
  let height = $state(800)
  let headH = $state(0)
  const body = $derived(Math.max(0, top - padTop - headH))
  const start = $derived(Math.max(0, Math.floor(body / rowHeight) - overscan))
  const end = $derived(Math.min(items.length, Math.ceil((body + height) / rowHeight) + overscan))
  const visible = $derived(items.slice(start, end))

  export function scrollToIndex(i: number) {
    el.scrollTop = padTop + headH + i * rowHeight - height / 3
  }
</script>

<div class="scroll" bind:this={el} bind:clientHeight={height} onscroll={() => (top = el.scrollTop)} style:padding-top="{padTop}px">
  {#if header}<div bind:clientHeight={headH}>{@render header()}</div>{/if}
  <div class="body" style:height="{items.length * rowHeight}px">
    {#each visible as item, i (start + i)}
      <div class="row" style:transform="translateY({(start + i) * rowHeight}px)" style:height="{rowHeight}px">
        {@render row(item, start + i)}
      </div>
    {/each}
  </div>
</div>

<style>
  .scroll { height: 100%; overflow-y: auto; overflow-x: hidden; }
  .body { position: relative; }
  .row { position: absolute; top: 0; left: 0; right: 0; }
</style>
