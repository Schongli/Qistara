<script lang="ts">
  // The bar over the library: back and forward, and search. The window's caption buttons live in App.
  import Icon from './Icon.svelte'
  import { nav, ui } from '../lib/nav.svelte'

  // caps: leave room for the caption buttons when nothing is docked to the right
  let { caps = true }: { caps?: boolean } = $props()
  let input: HTMLInputElement
  $effect(() => {
    if (ui.search) input.focus()
  })
</script>

<header class="bar" class:caps data-tauri-drag-region>
  <div class="hist" data-tauri-drag-region>
    <button class="icon-btn press menu" aria-label="Menu" aria-expanded={ui.drawer} onclick={() => (ui.drawer = !ui.drawer)}><Icon name="ListBullets" size={18} weight="bold" /></button>
    <button class="icon-btn press" aria-label="Back" disabled={!nav.canBack} onclick={() => nav.back()}><Icon name="CaretLeft" size={17} weight="bold" /></button>
    <button class="icon-btn press" aria-label="Forward" disabled={!nav.canForward} onclick={() => nav.forward()}><Icon name="CaretRight" size={17} weight="bold" /></button>
  </div>
  <label class="search">
    <Icon name="MagnifyingGlass" size={15} weight="bold" />
    <input
      bind:this={input}
      bind:value={nav.query}
      placeholder="Search your library"
      aria-label="Search"
      spellcheck="false"
      oninput={() => nav.route.name !== 'search' && nav.go({ name: 'search' })}
      onkeydown={(e) => e.key === 'Escape' && ((nav.query = ''), input.blur(), nav.route.name === 'search' && nav.back())}
    />
    <kbd>Ctrl F</kbd>
  </label>
</header>

<style>
  .bar {
    position: absolute; z-index: 10; inset: 0 0 auto 0; height: var(--bar-h);
    display: grid; grid-template-columns: 1fr minmax(240px, 440px) 1fr; align-items: center; gap: 12px;
    padding: 0 14px; background: var(--bar); backdrop-filter: var(--blur);
  }
  .bar.caps { padding-right: 146px; }
  :global(.os-macos) .bar.caps { padding-right: 16px; }
  .hist { display: flex; gap: 2px; }
  .icon-btn:disabled { color: var(--text-3); background: none; }
  .search {
    display: flex; align-items: center; gap: 8px; height: 32px; padding: 0 10px 0 12px;
    border-radius: 16px; background: var(--fill-2); color: var(--text-2);
  }
  .search:focus-within { box-shadow: 0 0 0 2px var(--accent); background: var(--bg); }
  input { flex: 1; min-width: 0; border: 0; background: none; outline: none; color: var(--text); }
  input::placeholder { color: var(--text-2); }
  kbd { font: 600 10px var(--font); color: var(--text-3); border: 1px solid var(--sep); border-radius: var(--r-xs); padding: 1px 4px; }
  .search:focus-within kbd { display: none; }
  .menu { display: none; }
  @media (max-width: 600px) {
    .bar { grid-template-columns: auto minmax(0, 1fr); gap: 8px; padding-left: 10px; }
    .search { min-width: 0; }
    /* Caps.svelte's buttons are 36 px wide here, and always over this bar (the pane is a sheet below it) */
    :global(.has-caps) .bar { padding-right: 114px; }
    .menu { display: grid; }
    kbd { display: none; }
  }
  @media (prefers-reduced-transparency: reduce) { .bar { backdrop-filter: none; } }
</style>
