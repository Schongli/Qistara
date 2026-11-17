<script lang="ts">
  // Caption buttons for Windows and Linux, top-right where people expect them (macOS keeps its own traffic
  // lights: tauri.macos.conf.json). Drawn as SVG: the Segoe glyph font only exists on Windows.
  import { getCurrentWindow } from '@tauri-apps/api/window'
  import { OS } from '../lib/player.svelte'
  import { PHONE } from '../lib/os'

  // light: white, over the dark pane or a video. overPane: light only because of the pane, which is a sheet
  // below the buttons at phone width, where they're back on the top bar
  let { light = false, overPane = false }: { light?: boolean; overPane?: boolean } = $props()
  const win = getCurrentWindow()
  let maximized = $state(false)
  win.isMaximized().then((m) => (maximized = m))
  win.onResized(async () => (maximized = await win.isMaximized()))
</script>

{#if OS !== 'macos' && !PHONE}
  <div class="caps" class:light class:pane={overPane}>
    <button aria-label="Minimize" onclick={() => win.minimize()}><svg viewBox="0 0 10 10"><path d="M0 5.5h10" /></svg></button>
    <button aria-label={maximized ? 'Restore' : 'Maximize'} onclick={() => win.toggleMaximize()}>
      <svg viewBox="0 0 10 10">{#if maximized}<path d="M2.5 2.5V.5h7v7h-2M.5 2.5h7v7h-7z" />{:else}<path d="M.5.5h9v9h-9z" />{/if}</svg>
    </button>
    <button class="close" aria-label="Close" onclick={() => win.close()}><svg viewBox="0 0 10 10"><path d="M.5.5l9 9M9.5.5l-9 9" /></svg></button>
  </div>
{/if}

<style>
  .caps { position: absolute; top: 0; right: 0; display: flex; z-index: 30; }
  button { display: grid; place-items: center; width: 46px; height: 32px; color: var(--text); }
  svg { width: 10px; height: 10px; fill: none; stroke: currentColor; stroke-width: 1; shape-rendering: crispEdges; }
  .close svg { shape-rendering: auto; }
  button:hover { background: var(--fill-2); }
  @media (max-width: 600px) { button { width: 36px; } }
  .light button { color: #fff; }
  .light button:hover { background: rgba(255, 255, 255, 0.16); }
  @media (max-width: 600px) {
    .pane.light button { color: var(--text); }
    .pane.light button:hover { background: var(--fill-2); }
  }
  .close:hover, .light .close:hover { background: #c42b1c; color: #fff; }
</style>
