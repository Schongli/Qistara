<script lang="ts">
  // Shown when the player didn't start (mpv.error): on macOS and Linux that's usually mpv not installed yet. What to
  // do comes from playbackhelp.ts; Restart is needed because a libmpv that failed to load isn't looked for again.
  import { invoke } from '@tauri-apps/api/core'
  import { openUrl, revealItemInDir } from '@tauri-apps/plugin-opener'
  import { OS } from '../lib/os'
  import { playbackHelp } from '../lib/playbackhelp'

  let { error, onclose }: { error: string; onclose: () => void } = $props()
  const help = $derived(playbackHelp(OS, error))
  let copied = $state('')
  let sheet = $state<HTMLDivElement>()
  $effect(() => sheet?.focus())

  async function copy(command: string) {
    await navigator.clipboard.writeText(command).catch(() => {})
    copied = command
    setTimeout(() => copied === command && (copied = ''), 1600)
  }
  const showLog = () => invoke<string>('log_file').then(revealItemInDir).catch(() => {})
</script>

<div class="scrim" role="presentation" onclick={onclose}></div>
<div class="sheet" role="alertdialog" aria-modal="true" aria-labelledby="ps-title" aria-describedby="ps-steps" tabindex="-1" bind:this={sheet} onkeydown={(e) => e.key === 'Escape' && onclose()}>
  <h2 class="t-h2" id="ps-title">{help.title}</h2>
  <ol id="ps-steps">
    {#each help.steps as s, i (i)}
      <li>
        <p>{s.text}{#if s.link}{' '}<button class="link" onclick={() => openUrl(s.link!.url)}>{s.link.label}</button>{/if}</p>
        {#each s.commands ?? [] as c (c.command)}
          <div class="cmd">
            {#if c.label}<span class="t-sub who">{c.label}</span>{/if}
            <code>{c.command}</code>
            <button class="btn press" aria-label="Copy {c.command}" onclick={() => copy(c.command)}>{copied === c.command ? 'Copied' : 'Copy'}</button>
          </div>
        {/each}
      </li>
    {/each}
  </ol>
  <details>
    <summary class="t-sub">Details</summary>
    <p class="detail">{help.detail}</p>
  </details>
  <div class="actions">
    <button class="btn press" onclick={showLog}>Show Log</button>
    <span class="grow"></span>
    <button class="btn press" onclick={onclose}>Not Now</button>
    <button class="btn primary press" onclick={() => invoke('app_restart')}>Restart Qistara</button>
  </div>
</div>

<style>
  .scrim { position: fixed; inset: 0; z-index: 80; background: var(--sheet-dim); animation: fade 200ms; }
  .sheet {
    position: fixed; z-index: 81; top: 0; left: 50%; width: min(560px, 94vw); max-height: 88vh; overflow-y: auto;
    transform: translateX(-50%); padding: 20px 22px 18px; border-radius: 0 0 var(--r-xl) var(--r-xl);
    background: var(--bg-2); border: 1px solid var(--sep); border-top: 0; box-shadow: var(--shadow-pop);
    animation: drop 280ms var(--ease-out); outline: none;
  }
  @keyframes drop { from { transform: translate(-50%, -100%); } }
  @keyframes fade { from { opacity: 0; } }
  h2 { margin: 0 0 8px; }
  ol { margin: 0; padding-left: 20px; display: grid; gap: 10px; }
  li p { margin: 0; line-height: 1.45; }
  .cmd { display: grid; grid-template-columns: 1fr auto; align-items: center; gap: 10px; margin-top: 6px; padding: 6px 6px 6px 10px; border-radius: var(--r); background: var(--fill); }
  /* the same label column in every row, so the commands line up */
  .cmd:has(.who) { grid-template-columns: 11em 1fr auto; }
  code { font-family: ui-monospace, 'SF Mono', Menlo, Consolas, monospace; font-size: 12.5px; user-select: text; overflow-wrap: anywhere; }
  details { margin-top: 14px; }
  summary { cursor: pointer; }
  .detail { margin: 6px 0 0; font-family: ui-monospace, 'SF Mono', Menlo, Consolas, monospace; font-size: 11.5px; color: var(--text-2); user-select: text; overflow-wrap: anywhere; }
  .actions { display: flex; gap: 8px; margin-top: 16px; }
  .grow { flex: 1; }
  @media (max-width: 600px) { .cmd:has(.who) { grid-template-columns: 1fr auto; } .cmd .who { grid-column: 1 / -1; } }
  @media (prefers-reduced-motion: reduce) { .sheet, .scrim { animation: none; } }
</style>
