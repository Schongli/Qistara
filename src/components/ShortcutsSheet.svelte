<script lang="ts">
  // Keyboard Shortcuts (Ctrl+/ or ?): everything in shortcuts.ts, grouped, with the keys as this OS writes them.
  import { SHORTCUTS, display, type Group } from '../lib/shortcuts'
  import { OS } from '../lib/os'

  let { onclose }: { onclose: () => void } = $props()
  const GROUPS: [Group, string][] = [['Playback', 'Playback'], ['Library', 'Library'], ['Window', 'Window'], ['Video', 'While a video plays']]
  let sheet = $state<HTMLDivElement>()
  $effect(() => sheet?.focus())
</script>

<div class="scrim" role="presentation" onclick={onclose}></div>
<div class="sheet" role="dialog" aria-modal="true" aria-labelledby="shortcuts-title" tabindex="-1" bind:this={sheet} onkeydown={(e) => e.key === 'Escape' && onclose()}>
  <h2 class="t-h2" id="shortcuts-title">Keyboard Shortcuts</h2>
  <div class="cols">
    {#each GROUPS as [g, title] (g)}
      <section>
        <h3>{title}</h3>
        <dl>
          {#each SHORTCUTS.filter((s) => s.group === g) as s (s.keys + s.label)}
            <dt>{s.label}</dt>
            <dd><kbd>{display(s.keys, OS === 'macos')}</kbd></dd>
          {/each}
        </dl>
      </section>
    {/each}
  </div>
</div>

<style>
  .scrim { position: fixed; inset: 0; z-index: 80; background: var(--sheet-dim); animation: fade 200ms; }
  .sheet {
    position: fixed; z-index: 81; top: 0; left: 50%; width: min(860px, 94vw); max-height: 86vh; overflow-y: auto;
    transform: translateX(-50%); padding: 20px 24px 22px; border-radius: 0 0 var(--r-xl) var(--r-xl);
    background: var(--bg-2); border: 1px solid var(--sep); border-top: 0; box-shadow: var(--shadow-pop);
    animation: drop 280ms var(--ease-out); outline: none;
  }
  @keyframes drop { from { transform: translate(-50%, -100%); } }
  @keyframes fade { from { opacity: 0; } }
  h2 { margin: 0 0 14px; }
  .cols { columns: 2 340px; column-gap: 32px; }
  section { break-inside: avoid; margin-bottom: 16px; }
  h3 { margin: 0 0 6px; font-size: 12px; font-weight: 650; letter-spacing: 0.04em; text-transform: uppercase; color: var(--text-2); }
  dl { display: grid; grid-template-columns: 1fr auto; gap: 5px 16px; margin: 0; align-items: center; }
  dt { font-size: 13px; }
  dd { margin: 0; text-align: right; }
  kbd {
    display: inline-block; min-width: 22px; padding: 1px 6px; border: 1px solid var(--sep); border-bottom-width: 2px;
    border-radius: var(--r-xs); background: var(--fill); font: 600 12px var(--font); color: var(--text); text-align: center;
  }
  @media (prefers-reduced-motion: reduce) { .sheet, .scrim { animation: none; } }
</style>
