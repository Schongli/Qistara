<script lang="ts">
  // The player along the bottom: what's playing, transport and progress, then the panes and volume.
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
  import ArrowsOutSimple from 'phosphor-svelte/lib/ArrowsOutSimple'
  import Heart from 'phosphor-svelte/lib/Heart'
  import Artwork from './Artwork.svelte'
  import Slider from './Slider.svelte'
  import SignalPath from './SignalPath.svelte'
  import { enterMini } from '../lib/mini'
  import { eq } from '../lib/eq.svelte'
  import { mpv, seek, togglePause } from '../lib/player.svelte'
  import { queue } from '../lib/queue.svelte'
  import { nav, ui } from '../lib/nav.svelte'
  import { albumKey, lib, love } from '../lib/library.svelte'
  import { quality, time } from '../lib/format'
  import { setVolume, volume } from '../lib/settings.svelte'

  const t = $derived(queue.current)
  // the library's copy: Love patches it, not the queue's
  const loved = $derived(t && t.id > 0 ? (lib.byId.get(t.id)?.loved ?? t.loved) : false)
  const q = $derived(t ? quality(t) : null)
  const duration = $derived(mpv.duration || t?.duration || 0)
  const icy = $derived(mpv.metadata?.['icy-title'] ?? '')
  // radio and URL streams; mpv reports a buffer length as their "duration"
  const live = $derived(t?.codec === 'Stream')
  const vol = $derived(volume())
  const toggle = (p: 'lyrics' | 'queue') => (ui.panel = ui.panel === p ? '' : p)
  let signalOpen = $state(false)
  let badge = $state<HTMLButtonElement | null>(null)
</script>

<footer class="player" aria-label="Player">
  <div class="now">
    {#if t}
      <button class="cover press" aria-label="Open Now Playing" onclick={() => (ui.nowPlaying = true)}>
        <Artwork art={t.art} size="52px" radius="6px" shadow={false} />
      </button>
      <div class="meta">
        <button class="title ellipsis" disabled={t.id <= 0} onclick={() => nav.go({ name: 'album', key: albumKey(t) })}>{live && icy ? icy : t.title}</button>
        <div class="sub ellipsis">{live ? (icy ? t.title : 'Live stream') : t.artist}</div>
      </div>
      {#if t.id > 0}
        <button class="icon-btn press love" class:on={loved} aria-label={loved ? 'Unlove' : 'Love'} aria-pressed={loved} onclick={() => love(lib.byId.get(t.id) ?? t)}>
          <Heart size={16} weight={loved ? 'fill' : 'regular'} />
        </button>
      {/if}
    {/if}
  </div>

  <div class="center">
    <div class="transport">
      <button class="icon-btn press" class:on={queue.shuffle} aria-label="Shuffle" aria-pressed={queue.shuffle} onclick={() => queue.toggleShuffle()}><Shuffle size={16} weight="bold" /></button>
      <button class="icon-btn press" aria-label="Previous" onclick={() => queue.prev()} disabled={!t}><SkipBack size={18} weight="fill" /></button>
      <button class="play press" aria-label={mpv.pause || !t ? 'Play' : 'Pause'} onclick={togglePause} disabled={!t}>
        {#if mpv.pause || !t}<Play size={18} weight="fill" />{:else}<Pause size={18} weight="fill" />{/if}
      </button>
      <button class="icon-btn press" aria-label="Next" onclick={() => queue.next()} disabled={!t}><SkipForward size={18} weight="fill" /></button>
      <button class="icon-btn press" class:on={queue.repeat !== 'off'} aria-label="Repeat {queue.repeat}" onclick={() => queue.cycleRepeat()}>
        {#if queue.repeat === 'one'}<RepeatOnce size={16} weight="bold" />{:else}<Repeat size={16} weight="bold" />{/if}
      </button>
    </div>
    <div class="scrub" class:hidden={!t || live}>
      <span class="tm">{time(mpv.time)}</span>
      <Slider class="bar-slider" label="Seek" value={mpv.time} max={duration} onchange={seek} />
      <span class="tm">{time(duration)}</span>
    </div>
  </div>

  <div class="right">
    {#if t}
      <button bind:this={badge} class="badge sig" aria-label="Signal path" aria-expanded={signalOpen} title={q?.detail} onclick={() => (signalOpen = !signalOpen)}>{q?.badge ?? (t.codec || 'Info')}</button>
    {/if}
    <button class="icon-btn press" class:on={ui.panel === 'lyrics'} aria-label="Lyrics" aria-pressed={ui.panel === 'lyrics'} onclick={() => toggle('lyrics')}><Quotes size={17} weight="fill" /></button>
    <button class="icon-btn press" class:on={ui.panel === 'queue'} aria-label="Up Next" aria-pressed={ui.panel === 'queue'} onclick={() => toggle('queue')}><ListBullets size={18} weight="bold" /></button>
    <button class="icon-btn press" class:on={eq.enabled} aria-label="Equalizer" title="Equalizer" onclick={() => nav.root({ name: 'eq' })}><SlidersHorizontal size={17} weight="bold" /></button>
    <div class="vol">
      {#if vol === 0}<SpeakerNone size={16} />{:else}<SpeakerHigh size={16} />{/if}
      <Slider label="Volume" value={vol} max={100} live wheel={5} onchange={setVolume} />
    </div>
    <button class="icon-btn press" aria-label="MiniPlayer" title="MiniPlayer (Ctrl+Shift+M)" onclick={enterMini}><PictureInPicture size={17} /></button>
    <button class="icon-btn press" aria-label="Open Now Playing" title="Now Playing" onclick={() => (ui.nowPlaying = true)} disabled={!t}><ArrowsOutSimple size={16} weight="bold" /></button>
  </div>
</footer>
<SignalPath bind:open={signalOpen} anchor={badge} />

<style>
  .player {
    flex: none; height: var(--player-h); display: grid; grid-template-columns: minmax(180px, 1fr) minmax(300px, 600px) minmax(180px, 1fr);
    align-items: center; gap: 16px; padding: 0 14px; background: var(--bar); backdrop-filter: blur(30px) saturate(180%);
    border-top: 1px solid var(--sep);
  }
  .now { display: flex; align-items: center; gap: 12px; min-width: 0; }
  .cover { flex: none; border-radius: 6px; }
  .meta { min-width: 0; display: grid; }
  .title { font-weight: 600; text-align: left; }
  .title:not(:disabled):hover { text-decoration: underline; text-underline-offset: 2px; }
  .sub { font-size: 12px; color: var(--text-2); }
  .love.on { color: var(--accent); }

  .center { display: grid; gap: 2px; justify-items: center; }
  .transport { display: flex; align-items: center; gap: 6px; }
  .play { width: 36px; height: 36px; border-radius: 50%; display: grid; place-items: center; color: var(--bg); background: var(--text); }
  .play:hover { transform: scale(1.05); }
  .play:disabled, .icon-btn:disabled { opacity: 0.4; }
  .scrub { width: 100%; display: flex; align-items: center; gap: 8px; }
  .scrub.hidden { visibility: hidden; }
  .tm { width: 40px; font-size: 11px; color: var(--text-2); font-variant-numeric: tabular-nums; }
  .tm:first-child { text-align: right; }
  .scrub :global(.bar-slider) { flex: 1; }

  .right { display: flex; align-items: center; justify-content: flex-end; gap: 2px; min-width: 0; }
  .sig { margin-right: 6px; }
  .sig:hover, .sig[aria-expanded='true'] { color: var(--text); border-color: var(--text-3); }
  .vol { display: flex; align-items: center; gap: 7px; flex: 0 1 110px; min-width: 64px; margin: 0 6px; color: var(--text-2); }
  .vol :global(.slider) { flex: 1; }
  @media (max-width: 1100px) { .sig { display: none; } }
  @media (prefers-reduced-transparency: reduce) { .player { backdrop-filter: none; } }
</style>
