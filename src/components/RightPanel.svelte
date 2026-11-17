<script lang="ts">
  // The pane docked beside the library: lyrics (the Qur'an view for a recitation) or Up Next, over the
  // blurred cover like Now Playing, so the same views read well in light and dark mode alike.
  import Icon from './Icon.svelte'
  import PanelContent from './PanelContent.svelte'
  import SessionPanel from './SessionPanel.svelte'
  import { ui } from '../lib/nav.svelte'
  import { queue } from '../lib/queue.svelte'
  import { artUrl, largeCover } from '../lib/library.svelte'

  let cover = $state<string | null>(null)
  $effect(() => {
    const path = queue.current?.path
    cover = null
    if (path && queue.current?.id && queue.current.id > 0) largeCover(path).then((c) => path === queue.current?.path && (cover = c))
  })
  const bg = $derived(cover ?? artUrl(queue.current?.art))
  const tabs = [['lyrics', 'Lyrics'], ['queue', 'Up Next'], ['session', 'Session']] as const
</script>

<aside class="panel" aria-label={ui.panel === 'queue' ? 'Up Next' : ui.panel === 'session' ? 'Session' : 'Lyrics'}>
  <div class="backdrop" aria-hidden="true">{#if bg}<img src={bg} alt="" />{/if}</div>
  <header data-tauri-drag-region>
    <div class="tabs" role="tablist" aria-label="Pane">
      {#each tabs as [id, label] (id)}
        <button role="tab" class="tab press" class:on={ui.panel === id} aria-selected={ui.panel === id} onclick={() => (ui.panel = id)}>{label}</button>
      {/each}
    </div>
    <button class="icon-btn press close" aria-label="Close pane" onclick={() => (ui.panel = '')}><Icon name="X" size={14} weight="bold" /></button>
  </header>
  <div class="body">
    {#if ui.panel === 'session'}
      <SessionPanel />
    {:else if queue.current || ui.panel === 'queue'}
      <PanelContent mode={ui.panel === 'queue' ? 'queue' : 'lyrics'} />
    {:else}
      <p class="empty">Lyrics show here while a song plays.</p>
    {/if}
  </div>
</aside>

<style>
  .panel {
    position: relative; flex: none; width: 340px; display: flex; flex-direction: column; overflow: hidden;
    color: #fff; background: #17191a; border-left: 1px solid var(--sep);
  }
  .backdrop { position: absolute; inset: -20%; pointer-events: none; }
  .backdrop img { width: 100%; height: 100%; object-fit: cover; filter: blur(60px) saturate(1.5) brightness(0.5); transform: scale(1.2); }
  .backdrop::after { content: ''; position: absolute; inset: 0; background: rgba(0, 0, 0, 0.28); }
  header { position: relative; display: flex; align-items: center; justify-content: space-between; padding: 8px 12px 4px; }
  /* the window's caption buttons (Caps.svelte) take the top right: the tabs go on a row below them */
  :global(.has-caps) header { padding-top: 38px; }
  @media (max-width: 600px) {
    .panel { position: fixed; z-index: 45; inset: 0 0 var(--player-h) 0; width: auto; border-left: 0; }
    /* the sheet starts below the caption buttons, so they stay in reach */
    :global(.has-caps) .panel { top: 32px; }
    :global(.has-caps) header { padding-top: 8px; }
  }
  .tabs { display: flex; gap: 2px; min-width: 0; }
  .tab { height: 26px; padding: 0 9px; white-space: nowrap; border-radius: 13px; font-size: 12px; font-weight: 600; color: rgba(255, 255, 255, 0.7); }
  .tab:hover { background: rgba(255, 255, 255, 0.1); }
  .tab.on { color: #111; background: rgba(255, 255, 255, 0.92); }
  .close { color: rgba(255, 255, 255, 0.7); width: 28px; height: 28px; }
  .close:hover { background: rgba(255, 255, 255, 0.12); color: #fff; }
  .body { position: relative; flex: 1; min-height: 0; }
  .empty { margin: 40px 24px; text-align: center; color: rgba(255, 255, 255, 0.6); }
  @media (prefers-reduced-transparency: reduce) { .backdrop { display: none; } }
</style>
