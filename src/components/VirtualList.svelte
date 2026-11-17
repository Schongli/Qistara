<script lang="ts" generics="T">
  // Renders only the visible rows of a long fixed-height list (50k songs stay smooth). By default the list is
  // its own scroller; with `page` it sits in the page's scroller (below a header, say) and follows that.
  import type { Snippet } from 'svelte'

  let { items, rowHeight, row, header, padTop = 0, overscan = 10, page = false, empty }: {
    items: T[]
    rowHeight: number
    row: Snippet<[T, number]>
    header?: Snippet
    padTop?: number
    overscan?: number
    page?: boolean
    empty?: Snippet
  } = $props()

  let el: HTMLDivElement
  let top = $state(0)
  let height = $state(800)
  let headH = $state(0)
  const body = $derived(Math.max(0, top - padTop - headH))
  const start = $derived(Math.max(0, Math.floor(body / rowHeight) - overscan))
  const end = $derived(Math.min(items.length, Math.ceil((body + height) / rowHeight) + overscan))
  const visible = $derived(items.slice(start, end))

  /** The nearest ancestor that scrolls. */
  function scroller(n: HTMLElement): HTMLElement {
    for (let p = n.parentElement; p; p = p.parentElement) if (/(auto|scroll)/.test(getComputedStyle(p).overflowY)) return p
    return document.documentElement
  }

  // page mode: how far the list's top has scrolled past the scroller's top, and how tall the scroller is
  $effect(() => {
    if (!page) return
    const sc = scroller(el)
    const update = () => {
      top = sc.getBoundingClientRect().top - el.getBoundingClientRect().top
      height = sc.clientHeight
    }
    update()
    sc.addEventListener('scroll', update, { passive: true })
    const ro = new ResizeObserver(update)
    ro.observe(sc)
    ro.observe(el)
    return () => (sc.removeEventListener('scroll', update), ro.disconnect())
  })

  export function scrollToIndex(i: number) {
    if (page) scroller(el).scrollTop += el.getBoundingClientRect().top - scroller(el).getBoundingClientRect().top + headH + i * rowHeight - height / 3
    else el.scrollTop = padTop + headH + i * rowHeight - height / 3
  }
</script>

<div class="scroll" class:page data-scroll={page ? undefined : ''} bind:this={el} bind:clientHeight={() => height, (v) => !page && (height = v)}
  onscroll={() => !page && (top = el.scrollTop)} style:padding-top="{padTop}px">
  {#if header}<div bind:clientHeight={headH}>{@render header()}</div>{/if}
  {#if !items.length && empty}{@render empty()}{/if}
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
  .scroll.page { height: auto; overflow: visible; }
  .body { position: relative; }
  .row { position: absolute; top: 0; left: 0; right: 0; }
</style>
