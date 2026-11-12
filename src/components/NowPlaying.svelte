<script lang="ts">
  import { Spring } from 'svelte/motion'
  import { untrack } from 'svelte'
  import CaretDown from 'phosphor-svelte/lib/CaretDown'
  import Play from 'phosphor-svelte/lib/Play'
  import Pause from 'phosphor-svelte/lib/Pause'
  import SkipBack from 'phosphor-svelte/lib/SkipBack'
  import SkipForward from 'phosphor-svelte/lib/SkipForward'
  import Shuffle from 'phosphor-svelte/lib/Shuffle'
  import Repeat from 'phosphor-svelte/lib/Repeat'
  import RepeatOnce from 'phosphor-svelte/lib/RepeatOnce'
  import SpeakerHigh from 'phosphor-svelte/lib/SpeakerHigh'
  import Quotes from 'phosphor-svelte/lib/Quotes'
  import ListBullets from 'phosphor-svelte/lib/ListBullets'
  import Moon from 'phosphor-svelte/lib/Moon'
  import Translate from 'phosphor-svelte/lib/Translate'
  import GraduationCap from 'phosphor-svelte/lib/GraduationCap'
  import WordCard from './WordCard.svelte'
  import { card, learn } from '../lib/learn.svelte'
  import { lyr } from '../lib/lyrics.svelte'
  import { targetLanguage, tr } from '../lib/translate.svelte'
  import { languageName } from '../lib/translate'
  import { openMenu } from '../lib/menu.svelte'
  import { setSleep, sleep, sleepLabel } from '../lib/sleep.svelte'
  import Artwork from './Artwork.svelte'
  import Slider from './Slider.svelte'
  import QueueList from './QueueList.svelte'
  import LyricsView from './LyricsView.svelte'
  import { mpv, seek, togglePause } from '../lib/player.svelte'
  import { settings, setVolume, volume } from '../lib/settings.svelte'
  import { queue } from '../lib/queue.svelte'
  import { ui } from '../lib/nav.svelte'
  import { largeCover } from '../lib/library.svelte'
  import { quality, time } from '../lib/format'
  import { project, rubberband, velocity } from '../lib/gesture'

  let h = $state(900)
  const reduced = matchMedia('(prefers-reduced-motion: reduce)')
  // critically damped: the sheet settles without bounce unless a flick carried momentum
  const y = new Spring(2000, { stiffness: 0.12, damping: 0.95 })
  const art = new Spring(1, { stiffness: 0.14, damping: 0.62 }) // a touch of bounce, like Apple's pause shrink
  const shown = $derived(ui.nowPlaying || y.current < h - 1)

  // only opening and closing animate; a closed sheet follows window resizes instantly
  $effect(() => {
    const open = ui.nowPlaying
    untrack(() => {
      const target = open ? 0 : h
      if (reduced.matches) y.set(target, { instant: true })
      else y.target = target
    })
  })
  $effect(() => {
    const height = h
    untrack(() => {
      if (!ui.nowPlaying) y.set(height, { instant: true })
    })
  })
  $effect(() => {
    art.target = mpv.pause ? 0.84 : 1
  })

  const t = $derived(queue.current)
  const q = $derived(t ? quality(t) : null)
  const duration = $derived(mpv.duration || t?.duration || 0)
  let cover = $state<string | null>(null)
  $effect(() => {
    const path = t?.path
    cover = null
    if (path) largeCover(path).then((c) => path === queue.current?.path && (cover = c))
  })

  // drag the sheet down to dismiss: 1:1 tracking, rubber-band upwards, momentum decides
  let drag: { start: number; samples: { t: number; y: number }[] } | null = null
  function down(e: PointerEvent) {
    if ((e.target as HTMLElement).closest('button, [role="slider"], .side-pane')) return
    ;(e.currentTarget as HTMLElement).setPointerCapture(e.pointerId)
    drag = { start: e.clientY - y.current, samples: [{ t: e.timeStamp, y: e.clientY }] }
  }
  function moveSheet(e: PointerEvent) {
    if (!drag) return
    const dy = e.clientY - drag.start
    y.set(dy < 0 ? rubberband(dy, h) : dy, { instant: true })
    drag.samples.push({ t: e.timeStamp, y: e.clientY })
    if (drag.samples.length > 12) drag.samples.shift()
  }
  function up() {
    if (!drag) return
    const v = velocity(drag.samples)
    drag = null
    const dismiss = y.current + project(v) > h * 0.35
    if (dismiss) ui.nowPlaying = false
    else y.target = 0
  }
</script>

<svelte:window bind:innerHeight={h} />

{#if shown && t}
  <div
    class="sheet"
    role="dialog"
    aria-modal="true"
    aria-label="Now Playing"
    tabindex="-1"
    style:transform="translateY({Math.max(-40, y.current)}px)"
    onpointerdown={down}
    onpointermove={moveSheet}
    onpointerup={up}
    onpointercancel={up}
  >
    <div class="backdrop" aria-hidden="true">
      {#if cover}<img src={cover} alt="" />{/if}
    </div>
    <button class="close icon-btn press" aria-label="Close Now Playing" onclick={() => (ui.nowPlaying = false)}><CaretDown size={20} weight="bold" /></button>

    <div class="stage" class:with-side={ui.lyrics || ui.queue}>
      <div class="player">
        <div class="art" style:transform="scale({art.current})">
          <Artwork art={t.art} src={cover} size="100%" radius="12px" />
        </div>
        <div class="titles">
          <div class="title ellipsis">{t.title}</div>
          <div class="artist ellipsis">{t.artist}{t.album ? ` — ${t.album}` : ''}</div>
        </div>
        <div class="scrub">
          <Slider label="Seek" value={mpv.time} max={duration} onchange={seek} />
          <div class="times">
            <span>{time(mpv.time)}</span>
            {#if q?.badge}<span class="badge light" title={q.detail}>{q.badge}</span>{/if}
            <span>-{time(duration - mpv.time)}</span>
          </div>
        </div>
        <div class="transport">
          <button class="icon-btn press" class:on={queue.shuffle} aria-label="Shuffle" onclick={() => queue.toggleShuffle()}><Shuffle size={18} weight="bold" /></button>
          <button class="icon-btn press big" aria-label="Previous" onclick={() => queue.prev()}><SkipBack size={30} weight="fill" /></button>
          <button class="icon-btn press big play" aria-label={mpv.pause ? 'Play' : 'Pause'} onclick={togglePause}>
            {#if mpv.pause}<Play size={42} weight="fill" />{:else}<Pause size={42} weight="fill" />{/if}
          </button>
          <button class="icon-btn press big" aria-label="Next" onclick={() => queue.next()}><SkipForward size={30} weight="fill" /></button>
          <button class="icon-btn press" class:on={queue.repeat !== 'off'} aria-label="Repeat {queue.repeat}" onclick={() => queue.cycleRepeat()}>
            {#if queue.repeat === 'one'}<RepeatOnce size={18} weight="bold" />{:else}<Repeat size={18} weight="bold" />{/if}
          </button>
        </div>
        <div class="bottom">
          <div class="vol"><SpeakerHigh size={16} /><Slider label="Volume" value={volume()} max={100} live wheel={5} onchange={setVolume} /></div>
          <button
            class="icon-btn press sleep"
            class:on={sleep.until || sleep.endOfSong}
            aria-label="Sleep timer{sleepLabel() ? `: ${sleepLabel()}` : ''}"
            title="Sleep timer"
            onclick={(e) => openMenu(e, [
              { label: 'Off', action: () => setSleep(null), disabled: !sleep.until && !sleep.endOfSong },
              { label: 'End of Current Song', action: () => setSleep('song') },
              ...[15, 30, 45, 60, 90].map((m) => ({ label: `${m} Minutes`, action: () => setSleep(m) })),
            ])}
          ><Moon size={17} weight={sleep.until || sleep.endOfSong ? 'fill' : 'regular'} />{#if sleepLabel()}<span class="sl">{sleepLabel()}</span>{/if}</button>
          <button class="icon-btn press" class:on={ui.lyrics} aria-label="Lyrics" onclick={() => { ui.lyrics = !ui.lyrics; ui.queue = false }}><Quotes size={18} weight="fill" /></button>
          <button class="icon-btn press" class:on={ui.queue} aria-label="Up Next" onclick={() => { ui.queue = !ui.queue; ui.lyrics = false }}><ListBullets size={19} weight="bold" /></button>
        </div>
      </div>
      {#if ui.queue}
        <div class="side-pane"><QueueList dark /></div>
      {:else if ui.lyrics}
        <div class="side-pane lyrics">
          <LyricsView />
          {#if lyr.data}
            {@const lang = languageName(targetLanguage())}
            <!-- outside LyricsView so its top fade never dims the buttons -->
            <div class="tools">
              <button
                class="icon-btn press"
                class:on={settings.learnLyrics}
                aria-pressed={settings.learnLyrics}
                aria-label="Learn mode"
                title={settings.learnLyrics ? learn.error || 'Learn mode: tap any word for its meaning' : 'Learn: readings, romaji and word meanings'}
                onclick={() => (settings.learnLyrics = !settings.learnLyrics)}
              ><GraduationCap size={18} weight={settings.learnLyrics ? 'bold' : 'regular'} /></button>
              {#if !settings.learnLyrics}
                <button
                  class="icon-btn press"
                  class:on={settings.translateLyrics}
                  class:busy={tr.busy}
                  aria-pressed={settings.translateLyrics}
                  aria-label="Translate lyrics to {lang}"
                  title={!settings.translateLyrics ? `Translate to ${lang}` : tr.error || (tr.same ? `These lyrics are already in ${lang}` : `Translated to ${lang} by Google`)}
                  onclick={() => (settings.translateLyrics = !settings.translateLyrics)}
                ><Translate size={18} weight={settings.translateLyrics ? 'bold' : 'regular'} /></button>
              {/if}
            </div>
          {/if}
          {#if card.open}<WordCard />{/if}
        </div>
      {/if}
    </div>
  </div>
{/if}

<style>
  .sheet {
    position: fixed; inset: 0; z-index: 50; overflow: hidden; color: #fff; background: #111;
    touch-action: none; will-change: transform;
  }
  .backdrop { position: absolute; inset: -10%; }
  .backdrop img {
    width: 100%; height: 100%; object-fit: cover; filter: blur(70px) saturate(1.6) brightness(0.62);
    transform: scale(1.3); animation: drift 48s ease-in-out infinite alternate;
  }
  .backdrop::after { content: ''; position: absolute; inset: 0; background: linear-gradient(rgba(0, 0, 0, 0.1), rgba(0, 0, 0, 0.45)); }
  @keyframes drift { from { transform: scale(1.3) rotate(0deg) translate(0, 0); } to { transform: scale(1.45) rotate(8deg) translate(-3%, 2%); } }

  .close { position: absolute; top: 14px; left: 16px; z-index: 2; color: rgba(255, 255, 255, 0.85); }
  .close:hover { background: rgba(255, 255, 255, 0.14); color: #fff; }

  .stage { position: relative; height: 100%; display: grid; grid-template-columns: 1fr; place-items: center; padding: 48px 6vw; gap: 5vw; }
  .stage.with-side { grid-template-columns: minmax(320px, 1fr) minmax(300px, 1fr); }
  .player { width: min(440px, 56vh, 100%); display: grid; gap: 18px; }
  .art { will-change: transform; filter: drop-shadow(0 24px 40px rgba(0, 0, 0, 0.4)); }
  .titles { display: grid; gap: 2px; }
  .title { font-size: 22px; font-weight: 700; letter-spacing: -0.018em; }
  .artist { font-size: 17px; color: rgba(255, 255, 255, 0.66); letter-spacing: -0.01em; }

  .scrub :global(.track) { background: rgba(255, 255, 255, 0.22); }
  .scrub :global(.fill) { background: rgba(255, 255, 255, 0.86); }
  .times { display: flex; justify-content: space-between; align-items: center; font-size: 11px; color: rgba(255, 255, 255, 0.6); font-variant-numeric: tabular-nums; margin-top: 2px; }
  .badge.light { color: rgba(255, 255, 255, 0.85); border-color: rgba(255, 255, 255, 0.3); }

  .transport { display: flex; justify-content: space-between; align-items: center; }
  .transport .icon-btn, .bottom .icon-btn { color: rgba(255, 255, 255, 0.72); }
  .transport .icon-btn.on, .bottom .icon-btn.on { color: #fff; background: rgba(255, 255, 255, 0.16); }
  .transport .big { color: #fff; width: 56px; height: 56px; border-radius: 50%; }
  .transport .play { width: 68px; height: 68px; }
  .icon-btn:hover { background: rgba(255, 255, 255, 0.12); }

  .bottom { display: flex; align-items: center; gap: 8px; }
  .sleep { width: auto; min-width: 32px; padding: 0 8px; gap: 5px; display: inline-flex; }
  .sl { font-size: 11px; font-weight: 600; white-space: nowrap; }
  .vol { flex: 1; display: flex; align-items: center; gap: 10px; color: rgba(255, 255, 255, 0.66); }
  .vol :global(.slider) { flex: 1; }
  .vol :global(.track) { background: rgba(255, 255, 255, 0.22); }
  .vol :global(.fill) { background: rgba(255, 255, 255, 0.86); }

  .side-pane { align-self: stretch; width: 100%; max-width: 620px; overflow-y: auto; padding: 8px; border-radius: 14px; }
  .side-pane.lyrics { position: relative; overflow: hidden; height: min(78vh, 720px); align-self: center; padding: 0; }
  .tools { position: absolute; top: 6px; right: 6px; z-index: 2; display: flex; gap: 2px; }
  .tools .icon-btn { color: rgba(255, 255, 255, 0.72); }
  .tools .icon-btn.on { color: #fff; background: rgba(255, 255, 255, 0.16); }
  .tools .busy { animation: pulse 1.2s ease-in-out infinite; }
  @keyframes pulse { 50% { opacity: 0.45; } }
  @media (prefers-reduced-motion: reduce) { .tools .busy { animation: none; } }

  @media (prefers-reduced-motion: reduce) { .backdrop img { animation: none; } }
</style>
