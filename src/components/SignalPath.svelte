<script lang="ts">
  import CheckCircle from 'phosphor-svelte/lib/CheckCircle'
  import WarningCircle from 'phosphor-svelte/lib/WarningCircle'
  import ArrowDown from 'phosphor-svelte/lib/ArrowDown'
  import { mpv } from '../lib/player.svelte'
  import { queue } from '../lib/queue.svelte'
  import { settings } from '../lib/settings.svelte'
  import { bitsOf, khz, verdict } from '../lib/signal'
  import { quality } from '../lib/format'
  import { nav } from '../lib/nav.svelte'

  let { open = $bindable(false), anchor }: { open?: boolean; anchor?: HTMLElement | null } = $props()
  let el = $state<HTMLDivElement>()

  const t = $derived(queue.current)
  const v = $derived(verdict({
    audioIn: mpv.audioIn, audioOut: mpv.audioOut, ao: mpv.ao, exclusive: settings.bitPerfect,
    filters: mpv.af.length, volume: mpv.volume, speed: mpv.speed,
    replayGain: !settings.bitPerfect && settings.soundCheck, sourceBits: t?.bitDepth ?? undefined,
  }))
  const ch = (n?: number) => (n === 1 ? 'Mono' : n === 2 ? 'Stereo' : n ? `${n} ch` : '')
  const fmt = (f?: string) => (f ? f.replace(/p$/, '').replace(/^s(\d+)$/, '$1-bit int').replace(/^float$/, '32-bit float').replace(/^double$/, '64-bit float').replace(/^spdif-(.+)$/, '$1 bitstream') : '')
  const device = $derived(mpv.device === 'auto' ? 'System default device' : (mpv.devices.find((d) => d.name === mpv.device)?.description ?? mpv.device))
  // 24-bit FLAC decodes into a 32-bit container; say so rather than imply 32-bit audio
  const carried = (f?: string) => {
    const n = f ? bitsOf(f) : 0
    return t?.bitDepth && n > t.bitDepth && !f!.startsWith('float') ? `${t.bitDepth}-bit in ${n}-bit int` : fmt(f)
  }

  let pos = $state({ left: 0, top: 0 })
  $effect(() => {
    if (!open || !anchor) return
    const r = anchor.getBoundingClientRect()
    pos = { left: Math.min(innerWidth - 336, Math.max(8, r.left + r.width / 2 - 160)), top: r.bottom + 8 }
  })
</script>

<svelte:window
  onpointerdown={(e) => open && !el?.contains(e.target as Node) && !anchor?.contains(e.target as Node) && (open = false)}
  onkeydown={(e) => open && e.key === 'Escape' && (open = false)}
/>

{#if open && t}
  <div bind:this={el} class="pop" role="dialog" aria-label="Signal path" style:left="{pos.left}px" style:top="{pos.top}px">
    <div class="verdict" class:ok={v.bitPerfect}>
      {#if v.bitPerfect}<CheckCircle size={20} weight="fill" />{:else}<WarningCircle size={20} weight="fill" />{/if}
      <div>
        <div class="vt">{v.bitPerfect ? (mpv.audioOut?.format.startsWith('spdif') ? 'Bitstream passthrough' : 'Bit-perfect') : 'Not bit-perfect'}</div>
        <div class="vs">{v.bitPerfect ? 'Your DAC receives the exact samples in the file.' : v.issues.join(' · ')}</div>
      </div>
    </div>

    <ol class="path">
      <li>
        <span class="k">Source</span>
        <span class="val">{quality(t).detail || t.codec}{t.channels ? ` · ${ch(t.channels)}` : ''}</span>
      </li>
      <li class="arrow"><ArrowDown size={12} /></li>
      <li>
        <span class="k">Decoder</span>
        <span class="val">{mpv.codec?.toUpperCase() ?? '—'} → {carried(mpv.audioIn?.format)}{mpv.audioIn ? ` · ${khz(mpv.audioIn.samplerate)} · ${ch(mpv.audioIn['channel-count'])}` : ''}</span>
      </li>
      <li class="arrow"><ArrowDown size={12} /></li>
      <li>
        <span class="k">Processing</span>
        <span class="val">{[mpv.af.length && 'Equalizer', !settings.bitPerfect && settings.soundCheck && 'Sound Check', Math.round(mpv.volume) !== 100 && `Volume ${Math.round(mpv.volume)}%`, mpv.speed !== 1 && `Speed ${mpv.speed}×`].filter(Boolean).join(', ') || 'None'}</span>
      </li>
      <li class="arrow"><ArrowDown size={12} /></li>
      <li>
        <span class="k">Output</span>
        <span class="val">WASAPI {settings.bitPerfect ? 'exclusive' : 'shared'} · {carried(mpv.audioOut?.format)}{mpv.audioOut ? ` · ${khz(mpv.audioOut.samplerate)}` : ''}</span>
      </li>
      <li class="dev t-sub ellipsis">{device}</li>
    </ol>
    {#if !settings.bitPerfect}
      <button class="fix" onclick={() => { settings.bitPerfect = true; open = false }}>Turn on Bit-Perfect mode</button>
    {:else}
      <button class="fix ghost" onclick={() => { open = false; nav.root({ name: 'settings' }) }}>Audio Settings…</button>
    {/if}
  </div>
{/if}

<style>
  .pop {
    position: fixed; z-index: 70; width: 320px; padding: 14px; border-radius: 12px; color: var(--text);
    background: var(--bar); backdrop-filter: blur(34px) saturate(180%); border: 1px solid var(--sep); box-shadow: var(--shadow-pop);
    transform-origin: top center; animation: pop 160ms var(--ease-out);
  }
  @keyframes pop { from { opacity: 0; transform: scale(0.96) translateY(-4px); } }
  .verdict { display: flex; gap: 10px; align-items: flex-start; padding-bottom: 12px; border-bottom: 1px solid var(--sep); color: #e8a33d; }
  .verdict.ok { color: #30d158; }
  .vt { font-weight: 650; color: var(--text); }
  .vs { font-size: 12px; color: var(--text-2); line-height: 1.4; }
  .path { list-style: none; margin: 10px 0 0; padding: 0; display: grid; gap: 2px; }
  .path li:not(.arrow):not(.dev) { display: grid; grid-template-columns: 78px 1fr; gap: 8px; font-size: 12px; }
  .k { color: var(--text-2); }
  .val { font-variant-numeric: tabular-nums; }
  .arrow { color: var(--text-3); padding-left: 30px; height: 14px; }
  .dev { padding-left: 86px; }
  .fix { margin-top: 12px; width: 100%; height: 28px; border-radius: 7px; background: var(--accent); color: var(--on-accent); font-weight: 600; }
  .fix.ghost { background: var(--fill-2); color: var(--text); }
  @media (prefers-reduced-transparency: reduce) { .pop { background: var(--bg-2); backdrop-filter: none; } }
</style>
