<script lang="ts">
  import { getCurrentWindow } from '@tauri-apps/api/window'
  import { open } from '@tauri-apps/plugin-dialog'
  import CaretLeft from 'phosphor-svelte/lib/CaretLeft'
  import Play from 'phosphor-svelte/lib/Play'
  import Pause from 'phosphor-svelte/lib/Pause'
  import ClockCounterClockwise from 'phosphor-svelte/lib/ClockCounterClockwise'
  import ClockClockwise from 'phosphor-svelte/lib/ClockClockwise'
  import SkipBack from 'phosphor-svelte/lib/SkipBack'
  import SkipForward from 'phosphor-svelte/lib/SkipForward'
  import Subtitles from 'phosphor-svelte/lib/Subtitles'
  import Waveform from 'phosphor-svelte/lib/Waveform'
  import Gauge from 'phosphor-svelte/lib/Gauge'
  import Repeat from 'phosphor-svelte/lib/Repeat'
  import Camera from 'phosphor-svelte/lib/Camera'
  import Crop from 'phosphor-svelte/lib/Crop'
  import Info from 'phosphor-svelte/lib/Info'
  import ListNumbers from 'phosphor-svelte/lib/ListNumbers'
  import CornersOut from 'phosphor-svelte/lib/CornersOut'
  import CornersIn from 'phosphor-svelte/lib/CornersIn'
  import SpeakerHigh from 'phosphor-svelte/lib/SpeakerHigh'
  import Slider from './Slider.svelte'
  import Caps from './Caps.svelte'
  import { cmd, mpv, seek, set, togglePause, type MpvTrack } from '../lib/player.svelte'
  import { queue } from '../lib/queue.svelte'
  import { menu, openMenu, type MenuItem } from '../lib/menu.svelte'
  import { nudgeVolume, setVolume, volume } from '../lib/settings.svelte'
  import { time } from '../lib/format'

  const win = getCurrentWindow()
  let fullscreen = $state(false)
  let idle = $state(false)
  let info = $state(false)
  let toast = $state('')
  let timer: ReturnType<typeof setTimeout>

  const t = $derived(queue.current)
  const duration = $derived(mpv.duration || t?.duration || 0)
  const audio = $derived(mpv.tracks.filter((x) => x.type === 'audio'))
  const subs = $derived(mpv.tracks.filter((x) => x.type === 'sub'))
  const video = $derived(mpv.tracks.find((x) => x.type === 'video' && x.selected))
  const loopA = $derived(typeof mpv.abA === 'number' ? mpv.abA : null)
  const loopB = $derived(typeof mpv.abB === 'number' ? mpv.abB : null)
  // mpv reports the default as "-1.000000" (or "no"); any positive ratio is an override
  const aspectSet = $derived(parseFloat(mpv.aspect) > 0)

  // controls step away after 2.5 s without the pointer moving, unless paused or a menu is open
  function wake() {
    idle = false
    clearTimeout(timer)
    timer = setTimeout(() => (idle = !mpv.pause && !menu.open && !info), 2500)
  }
  $effect(() => {
    if (mpv.pause) idle = false
  })
  // subtitles move up out of the way while the controls show (int options go through `set` as text)
  $effect(() => {
    cmd('set', 'sub-margin-y', idle ? '22' : '96').catch(() => {})
  })
  wake()

  async function toggleFullscreen() {
    fullscreen = !(await win.isFullscreen())
    await win.setFullscreen(fullscreen)
  }
  function say(msg: string) {
    toast = msg
    setTimeout(() => toast === msg && (toast = ''), 1600)
  }
  const label = (x: MpvTrack, i: number) =>
    [x.title, x.lang?.toUpperCase(), x.codec?.toUpperCase(), x['demux-channel-count'] && `${x['demux-channel-count']}ch`]
      .filter(Boolean).join(' · ') || `Track ${i + 1}`
  const check = (on: boolean, s: string) => `${on ? '✓' : ' '}  ${s}`

  const audioMenu = (e: MouseEvent) =>
    openMenu(e, audio.map((x, i) => ({ label: check(x.selected, label(x, i)), action: () => cmd('set', 'aid', String(x.id)) })))

  const subMenu = (e: MouseEvent) =>
    openMenu(e, [
      { label: check(!subs.some((x) => x.selected), 'Off'), action: () => cmd('set', 'sid', 'no') },
      ...subs.map((x, i): MenuItem => ({ label: check(x.selected, label(x, i) + (x.external ? ' (file)' : '')), action: () => cmd('set', 'sid', String(x.id)) })),
      'sep',
      { label: 'Add Subtitle File…', action: addSub },
      { label: `Delay −0.1 s  (now ${mpv.subDelay.toFixed(1)} s)`, action: () => set('sub-delay', +(mpv.subDelay - 0.1).toFixed(2)) },
      { label: 'Delay +0.1 s', action: () => set('sub-delay', +(mpv.subDelay + 0.1).toFixed(2)) },
      { label: 'Reset Delay', action: () => set('sub-delay', 0), disabled: !mpv.subDelay },
      'sep',
      { label: 'Larger Text', action: () => set('sub-scale', +(mpv.subScale + 0.1).toFixed(2)) },
      { label: 'Smaller Text', action: () => set('sub-scale', Math.max(0.3, +(mpv.subScale - 0.1).toFixed(2))) },
    ])

  const speeds = [0.5, 0.75, 1, 1.25, 1.5, 1.75, 2]
  const speedMenu = (e: MouseEvent) =>
    openMenu(e, speeds.map((s) => ({ label: check(Math.abs(mpv.speed - s) < 0.01, s === 1 ? 'Normal' : `${s}×`), action: () => set('speed', s) })))

  const aspects: [string, string][] = [['-1', 'Original'], ['16:9', '16:9'], ['4:3', '4:3'], ['21:9', '21:9'], ['2.35:1', '2.35:1'], ['1:1', '1:1']]
  const aspectOf = (v: string) => { const [a, b] = v.split(':').map(Number); return (a / b).toFixed(3) }
  const aspectMenu = (e: MouseEvent) =>
    openMenu(e, [
      ...aspects.map(([v, l]): MenuItem => ({ label: check(v === '-1' ? !aspectSet : aspectSet && aspectOf(v) === parseFloat(mpv.aspect).toFixed(3), l), action: () => set('video-aspect-override', v) })),
      'sep',
      { label: check(mpv.panscan > 0, 'Zoom to Fill'), action: () => set('panscan', mpv.panscan > 0 ? 0 : 1) },
    ])

  const chapterMenu = (e: MouseEvent) =>
    openMenu(e, mpv.chapters.map((c, i) => ({ label: check(mpv.chapter === i, `${time(c.time)}  ${c.title ?? `Chapter ${i + 1}`}`), action: () => cmd('set', 'chapter', String(i)) })))

  async function addSub() {
    const f = await open({ multiple: false, filters: [{ name: 'Subtitles', extensions: ['srt', 'ass', 'ssa', 'vtt', 'sub', 'sup', 'idx'] }] })
    if (typeof f === 'string') await cmd('sub-add', f, 'select')
  }
  async function shot() {
    await cmd('screenshot', 'video')
    say('Screenshot saved to Pictures\\Confluence Player')
  }
  async function abLoop() {
    await cmd('ab-loop')
    say(loopA === null ? 'Loop start set' : loopB === null ? 'Looping A–B' : 'Loop cleared')
  }
  async function close() {
    if (fullscreen) await win.setFullscreen((fullscreen = false))
    await queue.stop()
  }

  function keys(e: KeyboardEvent) {
    if ((e.target as HTMLElement).closest('input, select, textarea')) return
    wake()
    const k = e.key
    if (k === 'ArrowRight' && !e.ctrlKey) seek(mpv.time + (e.shiftKey ? 30 : 5))
    else if (k === 'ArrowLeft' && !e.ctrlKey) seek(Math.max(0, mpv.time - (e.shiftKey ? 30 : 5)))
    else if (k === 'ArrowUp') nudgeVolume(5)
    else if (k === 'ArrowDown') nudgeVolume(-5)
    else if (k === 'f' || k === 'F11') toggleFullscreen()
    else if (k === 'Escape' && fullscreen) toggleFullscreen()
    else if (k === 'm') set('mute', !mpv.mute)
    else if (k === 'v') cmd('cycle', 'sub')
    else if (k === 'b') cmd('cycle', 'audio')
    else if (k === '[') set('speed', Math.max(0.25, +(mpv.speed - 0.25).toFixed(2)))
    else if (k === ']') set('speed', Math.min(4, +(mpv.speed + 0.25).toFixed(2)))
    else if (k === '=') set('speed', 1)
    else if (k === '.') cmd('frame-step')
    else if (k === ',') cmd('frame-back-step')
    else if (k === 'l') abLoop()
    else if (k === 'i') info = !info
    else if (k === 's' && e.ctrlKey) shot()
    else return
    e.preventDefault()
    e.stopPropagation()
  }
</script>

<svelte:window onkeydown={keys} />

<div class="video" class:idle role="application" aria-label="Video player" onpointermove={wake}>
  <!-- click pauses, double-click toggles full screen (two pauses cancel out) -->
  <!-- scrolling over the picture changes the volume, like VLC -->
  <div
    class="surface"
    role="presentation"
    onclick={togglePause}
    ondblclick={toggleFullscreen}
    onwheel={(e) => { e.preventDefault(); wake(); nudgeVolume((-e.deltaY / 100) * 5) }}
  ></div>

  <header class="top" data-tauri-drag-region>
    <button class="icon-btn press" aria-label="Close video" onclick={close}><CaretLeft size={20} weight="bold" /></button>
    <div class="title ellipsis" data-tauri-drag-region>{mpv.title ?? t?.title}</div>
    {#if !fullscreen}<Caps light />{/if}
  </header>

  {#if toast}<div class="toast" role="status">{toast}</div>{/if}

  {#if info}
    <div class="info" role="dialog" aria-label="Media info">
      <h3>Media Info</h3>
      <dl>
        <dt>File</dt><dd class="ellipsis">{t?.path}</dd>
        <dt>Container</dt><dd>{mpv.container ?? '—'}</dd>
        {#if mpv.video}
          <dt>Video</dt><dd>{mpv.videoCodec ?? video?.codec} · {mpv.video.w}×{mpv.video.h}{mpv.fps ? ` · ${mpv.fps.toFixed(3).replace(/\.?0+$/, '')} fps` : ''}</dd>
          <dt>Pixels</dt><dd>{mpv.video.pixelformat}{mpv.video.gamma ? ` · ${mpv.video.gamma}` : ''}{mpv.video.primaries ? ` · ${mpv.video.primaries}` : ''}</dd>
          <dt>Decoder</dt><dd>{mpv.hwdec && mpv.hwdec !== 'no' ? `Hardware (${mpv.hwdec})` : 'Software'}</dd>
          {#if mpv.videoBitrate}<dt>Bitrate</dt><dd>{(mpv.videoBitrate / 1e6).toFixed(1)} Mb/s</dd>{/if}
        {/if}
        <dt>Subtitles</dt><dd>{subs.find((x) => x.selected) ? label(subs.find((x) => x.selected)!, 0) : `Off (${subs.length} available)`}{mpv.subDelay ? ` · ${mpv.subDelay.toFixed(1)} s` : ''}</dd>
        <dt>Audio</dt><dd>{mpv.codec?.toUpperCase() ?? '—'}{mpv.audioIn ? ` · ${mpv.audioIn.samplerate / 1000} kHz · ${mpv.audioIn['channel-count']} ch` : ''}{mpv.audioOut?.format.startsWith('spdif') ? ' · bitstream' : ''}</dd>
      </dl>
    </div>
  {/if}

  <footer class="bottom">
    <div class="scrub">
      <span class="tm">{time(mpv.time)}</span>
      <div class="bar">
        <Slider label="Seek" value={mpv.time} max={duration} live onchange={seek} />
        {#each mpv.chapters as c, i (i)}
          {#if i > 0 && duration}<span class="tick" style:left="{(c.time / duration) * 100}%"></span>{/if}
        {/each}
        {#if loopA !== null && duration}
          <span class="loop" style:left="{(loopA / duration) * 100}%" style:width="{(((loopB ?? mpv.time) - loopA) / duration) * 100}%"></span>
        {/if}
      </div>
      <span class="tm">-{time(duration - mpv.time)}</span>
    </div>
    <div class="row">
      <div class="group">
        <button class="icon-btn press" aria-label="Previous" onclick={() => queue.prev()}><SkipBack size={18} weight="fill" /></button>
        <button class="icon-btn press" aria-label="Back 10 seconds" onclick={() => seek(Math.max(0, mpv.time - 10))}><ClockCounterClockwise size={20} /></button>
        <button class="icon-btn press play" aria-label={mpv.pause ? 'Play' : 'Pause'} onclick={togglePause}>
          {#if mpv.pause}<Play size={26} weight="fill" />{:else}<Pause size={26} weight="fill" />{/if}
        </button>
        <button class="icon-btn press" aria-label="Forward 10 seconds" onclick={() => seek(mpv.time + 10)}><ClockClockwise size={20} /></button>
        <button class="icon-btn press" aria-label="Next" onclick={() => queue.next()}><SkipForward size={18} weight="fill" /></button>
        <div class="vol"><SpeakerHigh size={16} /><Slider label="Volume" value={volume()} max={100} live wheel={5} onchange={setVolume} /></div>
      </div>
      <div class="group">
        {#if mpv.chapters.length}<button class="icon-btn press" aria-label="Chapters" onclick={chapterMenu}><ListNumbers size={19} /></button>{/if}
        <button class="icon-btn press" aria-label="Audio track" onclick={audioMenu} disabled={audio.length < 2}><Waveform size={19} /></button>
        <button class="icon-btn press" class:on={subs.some((x) => x.selected)} aria-label="Subtitles" onclick={subMenu}><Subtitles size={19} /></button>
        <button class="icon-btn press" class:on={mpv.speed !== 1} aria-label="Playback speed" onclick={speedMenu}><Gauge size={19} />{#if mpv.speed !== 1}<span class="spd">{mpv.speed}×</span>{/if}</button>
        <button class="icon-btn press" class:on={loopA !== null} aria-label="A-B loop" onclick={abLoop}><Repeat size={18} /></button>
        <button class="icon-btn press" class:on={aspectSet || mpv.panscan > 0} aria-label="Aspect ratio" onclick={aspectMenu}><Crop size={19} /></button>
        <button class="icon-btn press" aria-label="Screenshot" onclick={shot}><Camera size={19} /></button>
        <button class="icon-btn press" class:on={info} aria-label="Media info" onclick={() => (info = !info)}><Info size={19} /></button>
        <button class="icon-btn press" aria-label={fullscreen ? 'Exit full screen' : 'Full screen'} onclick={toggleFullscreen}>
          {#if fullscreen}<CornersIn size={19} />{:else}<CornersOut size={19} />{/if}
        </button>
      </div>
    </div>
  </footer>
</div>

<style>
  .video { position: fixed; inset: 0; z-index: 40; color: #fff; }
  .video.idle { cursor: none; }
  .surface { position: absolute; inset: 0; }
  .top, .bottom { position: absolute; left: 0; right: 0; transition: opacity 260ms var(--ease-out), transform 260ms var(--ease-out); }
  .idle .top, .idle .bottom { opacity: 0; pointer-events: none; }
  .idle .top { transform: translateY(-6px); }
  .idle .bottom { transform: translateY(6px); }
  /* the native video layer can't be blurred from the webview, so a scrim keeps controls legible */
  .top { top: 0; height: 64px; display: flex; align-items: center; gap: 8px; padding: 0 150px 14px 12px; background: linear-gradient(rgba(0, 0, 0, 0.62), transparent); }
  .title { font-weight: 600; font-size: 14px; text-shadow: 0 1px 2px rgba(0, 0, 0, 0.5); }
  .bottom { bottom: 0; padding: 40px 24px 16px; background: linear-gradient(transparent, rgba(0, 0, 0, 0.72)); }
  .scrub { display: flex; align-items: center; gap: 10px; }
  .tm { font-size: 11.5px; font-variant-numeric: tabular-nums; width: 52px; color: rgba(255, 255, 255, 0.8); }
  .tm:first-child { text-align: right; }
  .bar { position: relative; flex: 1; }
  .bar :global(.track) { background: rgba(255, 255, 255, 0.25); }
  .bar :global(.fill) { background: #fff; }
  .tick { position: absolute; top: 6px; width: 2px; height: 4px; margin-left: -1px; background: rgba(0, 0, 0, 0.55); pointer-events: none; }
  .loop { position: absolute; top: 3px; height: 10px; border-radius: 3px; background: color-mix(in srgb, var(--accent) 50%, transparent); pointer-events: none; }
  .row { display: flex; justify-content: space-between; align-items: center; margin-top: 8px; }
  .group { display: flex; align-items: center; gap: 4px; }
  .icon-btn { color: rgba(255, 255, 255, 0.86); width: 36px; height: 36px; }
  .icon-btn:hover { background: rgba(255, 255, 255, 0.14); color: #fff; }
  .icon-btn.on { color: #fff; background: rgba(255, 255, 255, 0.2); }
  .icon-btn:disabled { opacity: 0.35; background: none; }
  .play { width: 44px; height: 44px; border-radius: 50%; color: #fff; }
  .spd { font-size: 10px; font-weight: 650; margin-left: 2px; }
  .vol { display: flex; align-items: center; gap: 8px; width: 140px; margin-left: 12px; color: rgba(255, 255, 255, 0.8); }
  .vol :global(.slider) { flex: 1; }
  .vol :global(.track) { background: rgba(255, 255, 255, 0.25); }
  .vol :global(.fill) { background: #fff; }
  .toast {
    position: absolute; top: 72px; left: 50%; transform: translateX(-50%); padding: 7px 14px; border-radius: 9px;
    background: rgba(20, 20, 22, 0.86); font-size: 12.5px; animation: pop 160ms var(--ease-out);
  }
  @keyframes pop { from { opacity: 0; transform: translate(-50%, -4px); } }
  .info {
    position: absolute; top: 70px; right: 20px; width: 340px; padding: 14px 16px; border-radius: 12px;
    background: rgba(20, 20, 22, 0.9); box-shadow: 0 18px 48px rgba(0, 0, 0, 0.5); font-size: 12px;
  }
  h3 { margin: 0 0 8px; font-size: 13px; }
  dl { display: grid; grid-template-columns: 74px 1fr; gap: 5px 10px; margin: 0; }
  dt { color: rgba(255, 255, 255, 0.55); }
  dd { margin: 0; min-width: 0; font-variant-numeric: tabular-nums; }
  @media (prefers-reduced-motion: reduce) { .idle .top, .idle .bottom { transform: none; } }
</style>
