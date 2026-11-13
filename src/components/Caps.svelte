<script lang="ts">
  // Windows caption buttons, kept top-right where Windows users expect them.
  import { getCurrentWindow } from '@tauri-apps/api/window'

  let { light = false }: { light?: boolean } = $props()
  const win = getCurrentWindow()
  let maximized = $state(false)
  win.isMaximized().then((m) => (maximized = m))
  win.onResized(async () => (maximized = await win.isMaximized()))
</script>

<div class="caps" class:light>
  <button aria-label="Minimize" onclick={() => win.minimize()}>&#xE921;</button>
  <button aria-label={maximized ? 'Restore' : 'Maximize'} onclick={() => win.toggleMaximize()}>{maximized ? '' : ''}</button>
  <button class="close" aria-label="Close" onclick={() => win.close()}>&#xE8BB;</button>
</div>

<style>
  .caps { position: absolute; top: 0; right: 0; display: flex; z-index: 30; }
  button { width: 46px; height: 32px; font: 10px 'Segoe Fluent Icons', 'Segoe MDL2 Assets'; color: var(--text); }
  button:hover { background: var(--fill-2); }
  .light button { color: #fff; }
  .light button:hover { background: rgba(255, 255, 255, 0.16); }
  .close:hover, .light .close:hover { background: #c42b1c; color: #fff; }
</style>
