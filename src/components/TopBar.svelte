<script lang="ts">
  import Play from 'phosphor-svelte/lib/Play'
  import Pause from 'phosphor-svelte/lib/Pause'
  import SkipBack from 'phosphor-svelte/lib/SkipBack'
  import SkipForward from 'phosphor-svelte/lib/SkipForward'
  import Shuffle from 'phosphor-svelte/lib/Shuffle'
  import Repeat from 'phosphor-svelte/lib/Repeat'
  import RepeatOnce from 'phosphor-svelte/lib/RepeatOnce'
  import SpeakerHigh from 'phosphor-svelte/lib/SpeakerHigh'
  import SpeakerNone from 'phosphor-svelte/lib/SpeakerNone'
  import Quotes from 'phosphor-svelte/lib/Quotes'
  import ListBullets from 'phosphor-svelte/lib/ListBullets'
  import SlidersHorizontal from 'phosphor-svelte/lib/SlidersHorizontal'
  import PictureInPicture from 'phosphor-svelte/lib/PictureInPicture'
  import { enterMini } from '../lib/mini'
  import { eq } from '../lib/eq.svelte'
  import CaretLeft from 'phosphor-svelte/lib/CaretLeft'
  import Artwork from './Artwork.svelte'
  import Caps from './Caps.svelte'
  import Slider from './Slider.svelte'
  import SignalPath from './SignalPath.svelte'
  import { mpv, seek, togglePause } from '../lib/player.svelte'
  import { queue } from '../lib/queue.svelte'
  import { nav, ui } from '../lib/nav.svelte'
  import { quality, time } from '../lib/format'
  import { setVolume, volume } from '../lib/settings.svelte'

  const t = $derived(queue.current)
  const q = $derived(t ? quality(t) : null)
  const duration = $derived(mpv.duration || t?.duration || 0)
  const icy = $derived(mpv.metadata?.['icy-title'] ?? '')
  // streams opened from Radio/Open URL; mpv reports a buffer length as "duration" for them
  const live = $derived(t?.codec === 'Stream')
  const vol = $derived(volume())
  let signalOpen = $state(false)
  let badge = $state<HTMLButtonElement | null>(null)
</script>

<header class="bar" data-tauri-drag-region>
  <div class="left" data-tauri-drag-region>
    {#if nav.canBack}
      <button class="icon-btn press" aria-label="Back" onclick={() => nav.back()}><CaretLeft size={18} weight="bold" /></button>
    {/if}
    <button class="icon-btn press" class:on={queue.shuffle} aria-label="Shuffle" aria-pressed={queue.shuffle} onclick={() => queue.toggleShuffle()}>
      <Shuffle size={16} weight="bold" />
    </button>
    <button class="icon-btn press big" aria-label="Previous" onclick={() => queue.prev()} disabled={!t}><SkipBack size={20} weight="fill" /></button>
    <button class="icon-btn press big play" aria-label={mpv.pause || !t ? 'Play' : 'Pause'} onclick={togglePause} disabled={!t}>
      {#if mpv.pause || !t}<Play size={26} weight="fill" />{:else}<Pause size={26} weight="fill" />{/if}
    </button>
    <button class="icon-btn press big" aria-label="Next" onclick={() => queue.next()} disabled={!t}><SkipForward size={20} weight="fill" /></button>
    <button class="icon-btn press" class:on={queue.repeat !== 'off'} aria-label="Repeat {queue.repeat}" onclick={() => queue.cycleRepeat()}>
      {#if queue.repeat === 'one'}<RepeatOnce size={16} weight="bold" />{:else}<Repeat size={16} weight="bold" />{/if}
    </button>
  </div>

  <div class="lcd" class:empty={!t}>
    {#if t}
      <button class="cover press" aria-label="Open Now Playing" onclick={() => (ui.nowPlaying = true)}>
        <Artwork art={t.art} size="44px" radius="4px" shadow={false} />
      </button>
      <div class="meta">
        <div class="title ellipsis">{live && icy ? icy : t.title}</div>
        <div class="sub ellipsis">
          {live ? (icy ? t.title : 'Live stream') : [t.artist, t.album].filter(Boolean).join(' — ')}
          <button bind:this={badge} class="badge sig" aria-label="Signal path" aria-expanded={signalOpen} title={q?.detail} onclick={() => (signalOpen = !signalOpen)}>
            {q?.badge ?? (t.codec || 'Info')}
          </button>
        </div>
        <div class="scrub" class:hidden={live}>
          <span class="tm">{time(mpv.time)}</span>
          <Slider class="lcd-slider" label="Seek" value={mpv.time} max={duration} onchange={seek} />
          <span class="tm">-{time(duration - mpv.time)}</span>
        </div>
      </div>
    {:else}
      <div class="logo" data-tauri-drag-region>Sonora</div>
    {/if}
  </div>

  <div class="right" data-tauri-drag-region>
    <div class="vol">
      {#if vol === 0}<SpeakerNone size={16} />{:else}<SpeakerHigh size={16} />{/if}
      <Slider label="Volume" value={vol} max={100} live wheel={5} onchange={setVolume} />
    </div>
    <button class="icon-btn press" aria-label="MiniPlayer" title="MiniPlayer (Ctrl+Shift+M)" onclick={enterMini}><PictureInPicture size={17} /></button>
    <button class="icon-btn press" class:on={eq.enabled} aria-label="Equalizer" title="Equalizer" onclick={() => nav.root({ name: 'eq' })}><SlidersHorizontal size={17} weight="bold" /></button>
    <button class="icon-btn press" class:on={ui.lyrics} aria-label="Lyrics" aria-pressed={ui.lyrics} onclick={() => { ui.lyrics = true; ui.queue = false; ui.nowPlaying = true }} disabled={!t}><Quotes size={17} weight="fill" /></button>
    <button class="icon-btn press" class:on={ui.queue} aria-label="Up Next" aria-pressed={ui.queue} onclick={() => (ui.queue = !ui.queue)}><ListBullets size={18} weight="bold" /></button>
  </div>

  <Caps />
</header>
<SignalPath bind:open={signalOpen} anchor={badge} />

<style>
  .bar {
    position: absolute; z-index: 10; inset: 0 0 auto 0; height: var(--bar-h);
    display: grid; grid-template-columns: 1fr minmax(320px, 560px) 1fr; align-items: center; gap: 12px;
    padding: 0 146px 0 14px; background: var(--bar); backdrop-filter: blur(30px) saturate(180%);
    border-bottom: 1px solid var(--sep);
  }
  .left, .right { display: flex; align-items: center; gap: 2px; min-width: 0; }
  .right { justify-content: flex-end; }
  .big { color: var(--text); }
  .play { width: 38px; height: 38px; }
  .icon-btn:disabled { color: var(--text-3); background: none; }

  .lcd {
    display: flex; align-items: center; gap: 10px; height: 46px; padding: 0 12px 0 1px; min-width: 0;
    border-radius: 7px; background: var(--fill); border: 1px solid var(--sep);
  }
  .lcd.empty { justify-content: center; }
  .logo { font-weight: 650; letter-spacing: -0.01em; color: var(--text-3); }
  .cover { flex: none; margin-left: 0; border-radius: 5px; overflow: hidden; }
  .meta { flex: 1; min-width: 0; display: grid; gap: 0; text-align: center; }
  .title { font-weight: 600; font-size: 12.5px; line-height: 16px; }
  .sub { font-size: 11.5px; color: var(--text-2); line-height: 15px; display: flex; justify-content: center; gap: 6px; align-items: center; }
  .sub .badge { height: 14px; font-size: 9px; }
  .sig:hover, .sig[aria-expanded='true'] { color: var(--text); border-color: var(--text-3); }
  .scrub { display: flex; align-items: center; gap: 6px; height: 12px; }
  .scrub.hidden { visibility: hidden; }
  .tm { font-size: 9.5px; color: var(--text-3); font-variant-numeric: tabular-nums; width: 34px; opacity: 0; transition: opacity 150ms; }
  .tm:first-child { text-align: right; }
  .lcd:hover .tm { opacity: 1; }
  .scrub :global(.lcd-slider) { flex: 1; height: 12px; }

  .vol { display: flex; align-items: center; gap: 7px; flex: 0 1 120px; min-width: 70px; margin-right: 6px; color: var(--text-2); }
  .vol :global(.slider) { flex: 1; }

  @media (prefers-reduced-transparency: reduce) { .bar { backdrop-filter: none; } }
</style>
