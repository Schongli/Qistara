<script lang="ts">
  import { menu } from '../lib/menu.svelte'

  let el = $state<HTMLDivElement>()
  let pos = $state({ x: 0, y: 0 })

  // keep the menu inside the window, flipping at the right/bottom edges
  $effect(() => {
    if (!menu.open || !el) return
    const r = el.getBoundingClientRect()
    pos = {
      x: menu.x + r.width > innerWidth - 8 ? menu.x - r.width : menu.x,
      y: menu.y + r.height > innerHeight - 8 ? Math.max(8, menu.y - r.height) : menu.y,
    }
    el.querySelector<HTMLButtonElement>('button')?.focus()
  })

  const close = () => (menu.open = false)
</script>

<svelte:window onpointerdown={(e) => menu.open && !el?.contains(e.target as Node) && close()} onblur={close} />

{#if menu.open}
  <div
    bind:this={el}
    class="menu"
    role="menu"
    tabindex="-1"
    style:left="{pos.x}px"
    style:top="{pos.y}px"
    style:transform-origin="{pos.x < menu.x ? 'right' : 'left'} top"
    onkeydown={(e) => {
      if (e.key === 'Escape') close()
      if (e.key === 'ArrowDown' || e.key === 'ArrowUp') {
        e.preventDefault()
        const bs = [...el!.querySelectorAll<HTMLButtonElement>('button:not(:disabled)')]
        const i = bs.indexOf(document.activeElement as HTMLButtonElement)
        bs[(i + (e.key === 'ArrowDown' ? 1 : bs.length - 1)) % bs.length]?.focus()
      }
    }}
  >
    {#each menu.items as it, i (i)}
      {#if it === 'sep'}
        <div class="sep"></div>
      {:else}
        <button role="menuitem" disabled={it.disabled} onclick={() => { close(); it.action() }}>{it.label}</button>
      {/if}
    {/each}
  </div>
{/if}

<style>
  .menu {
    position: fixed; z-index: 100; min-width: 200px; padding: 5px; border-radius: var(--r);
    background: var(--bar); backdrop-filter: var(--blur);
    border: 1px solid var(--sep); box-shadow: var(--shadow-pop);
    animation: pop 140ms var(--ease-out);
  }
  @keyframes pop { from { opacity: 0; transform: scale(0.96); } }
  button { display: block; width: 100%; text-align: left; padding: 5px 10px; border-radius: var(--r-xs); }
  button:hover, button:focus-visible { background: var(--accent); color: var(--on-accent); outline: none; }
  button:disabled { color: var(--text-3); background: none; }
  .sep { height: 1px; margin: 4px 8px; background: var(--sep); }
  @media (prefers-reduced-transparency: reduce) { .menu { background: var(--bg-2); backdrop-filter: none; } }
</style>
