<script lang="ts">
  import Play from 'phosphor-svelte/lib/Play'
  import Pause from 'phosphor-svelte/lib/Pause'
  import SkipBack from 'phosphor-svelte/lib/SkipBack'
  import SkipForward from 'phosphor-svelte/lib/SkipForward'
  import ArrowsOut from 'phosphor-svelte/lib/ArrowsOut'
  import X from 'phosphor-svelte/lib/X'
  import { getCurrentWindow } from '@tauri-apps/api/window'
  import Artwork from './Artwork.svelte'
  import Slider from './Slider.svelte'
  import { mpv, seek, togglePause } from '../lib/player.svelte'
  import { queue } from '../lib/queue.svelte'
  import { exitMini } from '../lib/mini'

  const t = $derived(queue.current)
  const duration = $derived(mpv.duration || t?.duration || 0)
  const icy = $derived(mpv.metadata?.['icy-title'] ?? '')
</script>

<div class="mini" data-tauri-drag-region role="region" aria-label="MiniPlayer">
  <button class="art press" aria-label="Back to the full player" onclick={exitMini}>
    <Artwork art={t?.art ?? null} size="76px" radius="6px" />
  </button>
  <div class="body" data-tauri-drag-region>
    <div class="txt" data-tauri-drag-region>
      <div class="title ellipsis">{icy || t?.title || 'Not Playing'}</div>
      <div class="sub ellipsis">{t ? [t.artist, t.album].filter(Boolean).join(' — ') : 'Confluence Player'}</div>
    </div>
    <div class="row">
      <button class="icon-btn press" aria-label="Previous" onclick={() => queue.prev()} disabled={!t}><SkipBack size={16} weight="fill" /></button>
      <button class="icon-btn press play" aria-label={mpv.pause || !t ? 'Play' : 'Pause'} onclick={togglePause} disabled={!t}>
        {#if mpv.pause || !t}<Play size={20} weight="fill" />{:else}<Pause size={20} weight="fill" />{/if}
      </button>
      <button class="icon-btn press" aria-label="Next" onclick={() => queue.next()} disabled={!t}><SkipForward size={16} weight="fill" /></button>
      <span class="sp" data-tauri-drag-region></span>
      <button class="icon-btn press" aria-label="Back to the full player" title="Full player" onclick={exitMini}><ArrowsOut size={15} /></button>
      <button class="icon-btn press" aria-label="Close Confluence Player" onclick={() => getCurrentWindow().close()}><X size={14} weight="bold" /></button>
    </div>
  </div>
  {#if t && duration}<div class="scrub"><Slider label="Seek" value={mpv.time} max={duration} onchange={seek} /></div>{/if}
</div>

<style>
  .mini {
    position: fixed; inset: 0; display: flex; align-items: center; gap: 12px; padding: 0 8px 0 10px;
    background: var(--sidebar); color: var(--text);
  }
  .art { flex: none; border-radius: 6px; }
  .body { flex: 1; min-width: 0; display: grid; gap: 2px; }
  .txt { min-width: 0; padding-top: 2px; }
  .title { font-weight: 650; font-size: 13px; }
  .sub { font-size: 11.5px; color: var(--text-2); }
  .row { display: flex; align-items: center; gap: 2px; margin-left: -6px; }
  .row .icon-btn { width: 28px; height: 28px; color: var(--text); }
  .row .play { width: 32px; height: 32px; }
  .icon-btn:disabled { color: var(--text-3); }
  .sp { flex: 1; align-self: stretch; }
  .scrub { position: absolute; left: 96px; right: 12px; bottom: 2px; }
  .scrub :global(.slider) { height: 8px; }
  .scrub :global(.track) { height: 3px; }
</style>
