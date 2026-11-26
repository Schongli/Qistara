// Bridge to libmpv: observed properties land in `mpv`. Commands and properties go through mpv.rs, which
// lets through only what the app uses: the page isn't granted the plugin's own command/set_property/init.
import { destroy, listenEvents, observeProperties, type MpvObservableProperty } from 'tauri-plugin-libmpv-api'
import { invoke } from '@tauri-apps/api/core'
import { AO, type AudioParams } from './signal'
import { OS, SHOWS_VIDEO } from './os'
import { youtubeSongId } from './lyricsources'

export interface MpvTrack {
  id: number
  type: 'video' | 'audio' | 'sub'
  title?: string
  lang?: string
  codec?: string
  selected: boolean
  external?: boolean
  'demux-channel-count'?: number
  'demux-samplerate'?: number
  image?: boolean
}
export interface MpvChapter { title?: string; time: number }
export interface MpvDevice { name: string; description: string }
export interface VideoParams { w: number; h: number; pixelformat: string; gamma?: string; primaries?: string; 'sig-peak'?: number }

const PROPS = [
  ['pause', 'flag'],
  ['idle-active', 'flag'],
  ['time-pos', 'double', 'none'],
  ['duration', 'double', 'none'],
  ['path', 'string', 'none'],
  ['media-title', 'string', 'none'],
  ['playlist-pos', 'int64'],
  ['metadata', 'node', 'none'],
  ['audio-params', 'node', 'none'],
  ['audio-out-params', 'node', 'none'],
  ['audio-codec-name', 'string', 'none'],
  ['current-ao', 'string', 'none'],
  ['af', 'node'],
  ['volume', 'double'],
  ['ao-volume', 'double', 'none'],
  ['mute', 'flag'],
  ['speed', 'double'],
  ['track-list', 'node'],
  ['chapter-list', 'node'],
  ['chapter', 'int64', 'none'],
  ['video-params', 'node', 'none'],
  ['hwdec-current', 'string', 'none'],
  ['audio-device-list', 'node'],
  ['audio-device', 'string'],
  ['sub-delay', 'double'],
  ['audio-delay', 'double'],
  ['sub-scale', 'double'],
  ['ab-loop-a', 'node'],
  ['ab-loop-b', 'node'],
  ['container-fps', 'double', 'none'],
  ['video-codec', 'string', 'none'],
  ['video-bitrate', 'double', 'none'],
  ['audio-bitrate', 'double', 'none'],
  ['file-format', 'string', 'none'],
  ['video-aspect-override', 'string'],
  ['panscan', 'double'],
  ['eof-reached', 'flag', 'none'],
] as const satisfies MpvObservableProperty[]

export const mpv = $state({
  ready: false,
  error: '',
  pause: true,
  idle: true,
  time: 0,
  duration: 0,
  path: null as string | null,
  title: null as string | null,
  playlistPos: -1,
  metadata: null as Record<string, string> | null,
  audioIn: null as AudioParams | null,
  audioOut: null as AudioParams | null,
  codec: null as string | null,
  ao: null as string | null,
  af: [] as unknown[],
  volume: 100,
  aoVolume: null as number | null,
  mute: false,
  speed: 1,
  tracks: [] as MpvTrack[],
  chapters: [] as MpvChapter[],
  chapter: null as number | null,
  video: null as VideoParams | null,
  hwdec: null as string | null,
  devices: [] as MpvDevice[],
  device: 'auto',
  subDelay: 0,
  audioDelay: 0,
  subScale: 1,
  abA: 'no' as number | string,
  abB: 'no' as number | string,
  fps: null as number | null,
  videoCodec: null as string | null,
  videoBitrate: null as number | null,
  audioBitrate: null as number | null,
  container: null as string | null,
  aspect: '-1',
  panscan: 0,
  eof: false,
  /** a file (or a seek) is opening: the play button shows a ring once it takes a moment */
  loading: false,
})

const KEY: Record<(typeof PROPS)[number][0], keyof typeof mpv> = {
  pause: 'pause', 'idle-active': 'idle', 'time-pos': 'time', duration: 'duration', path: 'path',
  'media-title': 'title', 'playlist-pos': 'playlistPos', metadata: 'metadata', 'audio-params': 'audioIn',
  'audio-out-params': 'audioOut', 'audio-codec-name': 'codec', 'current-ao': 'ao', af: 'af', volume: 'volume',
  'ao-volume': 'aoVolume', mute: 'mute', speed: 'speed', 'track-list': 'tracks', 'chapter-list': 'chapters',
  chapter: 'chapter', 'video-params': 'video', 'hwdec-current': 'hwdec', 'audio-device-list': 'devices',
  'audio-device': 'device', 'sub-delay': 'subDelay', 'audio-delay': 'audioDelay', 'sub-scale': 'subScale', 'ab-loop-a': 'abA',
  'ab-loop-b': 'abB', 'container-fps': 'fps', 'video-codec': 'videoCodec', 'video-bitrate': 'videoBitrate',
  'audio-bitrate': 'audioBitrate', 'file-format': 'container', 'video-aspect-override': 'aspect', panscan: 'panscan',
  'eof-reached': 'eof',
}
const EMPTY: Partial<Record<keyof typeof mpv, unknown>> = { time: 0, duration: 0, af: [], tracks: [], chapters: [], devices: [], abA: 'no', abB: 'no', eof: false }

export { OS, SHOWS_VIDEO }

// Bit-perfect profile: the OS's exclusive output (WASAPI exclusive, CoreAudio hog mode, ALSA) at the file's own
// rate/format, nothing in between.
export const BIT_PERFECT = {
  ao: AO[OS], 'audio-exclusive': 'yes', af: '', volume: 100, 'volume-max': 100, replaygain: 'no', speed: 1,
} as const

type Handler = (e: { event: string; [k: string]: unknown }) => void
const handlers = new Set<Handler>()
export const onMpvEvent = (h: Handler) => (handlers.add(h), () => handlers.delete(h))

onMpvEvent(({ event }) => {
  if (event === 'start-file' || event === 'seek') mpv.loading = true
  else if (event === 'playback-restart' || event === 'end-file' || event === 'idle') mpv.loading = false
})

function apply(name: (typeof PROPS)[number][0], data: unknown) {
  const k = KEY[name]
  ;(mpv as Record<string, unknown>)[k] = data ?? (k in EMPTY ? EMPTY[k] : null)
}

export async function startMpv() {
  try {
    // Listen before init: mpv sends the initial value of every observed property right away.
    await observeProperties(PROPS, ({ name, data }) => apply(name, data))
    await listenEvents((e) => handlers.forEach((h) => h(e as Parameters<Handler>[0])))
    // A page reload leaves the old instance running and init would skip, so start fresh.
    // (Re-reading props with get_property instead crashed libmpv-wrapper 0.3.2.)
    await destroy().catch(() => {})
    // mpv.rs adds the options that hold a path: the ytdl hook's yt-dlp, resume folder, log, screenshot folder
    await invoke('mpv_init', {
      mpvConfig: {
        initialOptions: {
          ...BIT_PERFECT,
          // no video in the window here (os.ts): video files play their sound, and mpv never opens a window
          ...(SHOWS_VIDEO ? {} : { vid: 'no' }),
          vo: 'gpu-next', hwdec: 'auto-safe', 'target-colorspace-hint': 'yes', 'video-sync': 'audio',
          idle: 'yes', 'force-window': 'no', 'keep-open': 'yes',
          'gapless-audio': 'weak', 'prefetch-playlist': 'yes', 'audio-display': 'no', 'cover-art-auto': 'no',
          'input-default-bindings': 'no', 'input-vo-keyboard': 'no', osc: 'no', 'osd-level': 0,
          'sub-auto': 'fuzzy', 'msg-level': 'all=warn',
          // YouTube tracks are ytdl://… (youtube.svelte.ts), played by the hook with our own yt-dlp (ytdlp.rs).
          // Other URLs, radio included, are opened by mpv itself.
          ytdl: 'yes', 'ytdl-format': 'bestaudio/best',
          'watch-later-options': 'start,aid,sid,sub-delay,audio-delay,sub-scale',
          'screenshot-format': 'png', 'screenshot-template': 'Qistara %F %wH.%wM.%wS',
        },
        observedProperties: Object.fromEntries(PROPS),
      },
    })
    mpv.ready = true
  } catch (e) {
    mpv.error = String(e)
    console.error('mpv start failed:', e)
  }
}

/** The one video format a YouTube video plays in, with its sound (mpv.rs VIDEO_FORMAT allows only this). */
export const VIDEO_FORMAT = 'bestvideo[height<=?1080]+bestaudio/best[height<=?1080]'

/** YouTube songs play from the streaming engine's loopback server (ytstream.rs). A song it couldn't open plays
 *  through mpv's ytdl hook instead (youtube.svelte.ts); `cookies` is the sign-in its yt-dlp uses, and `premium`
 *  asks for Premium quality through yt-dlp when signed in (youtube.svelte.ts keeps both current). */
export const engine = { cookies: null as unknown, premium: false, viaHook: new Set<string>() }

/** What mpv opens for a track: the engine's address for a YouTube song, else the track's own path. */
async function source(t: { path: string; kind: 'audio' | 'video' }) {
  const id = t.kind === 'audio' && !engine.viaHook.has(t.path) ? youtubeSongId(t.path) : null
  if (!id) return t.path
  return invoke<string>('ytstream_url', { id, cookies: engine.cookies, premium: engine.premium }).catch((e) => (console.warn('streaming engine:', e), t.path))
}

/** Looks the next YouTube songs up and fetches their start, so they play at once. */
export function warm(tracks: ({ path: string; kind: 'audio' | 'video' } | null | undefined)[]) {
  const ids = tracks.flatMap((t) => (t && t.kind === 'audio' && !engine.viaHook.has(t.path) && youtubeSongId(t.path)) || [])
  if (ids.length) invoke('ytstream_warm', { ids, cookies: engine.cookies, premium: engine.premium }).catch(() => {})
}

let resting = 0
/** Spread onto a row or card that plays a YouTube song: resting the pointer on it, or pressing it, fetches the
 *  song's start so a click plays at once. `{...warmOnHover(() => track)}` */
export const warmOnHover = (track: () => { path: string; kind: 'audio' | 'video' } | null) => ({
  onpointerenter: (e: PointerEvent) => {
    clearTimeout(resting)
    if (e.pointerType === 'mouse') resting = window.setTimeout(() => warm([track()]), 150)
  },
  onpointerleave: () => clearTimeout(resting),
  onpointerdown: () => warm([track()]),
})

/**
 * Loads a track with what it needs for this file only: a start time, and for a YouTube video (youtube.svelte.ts
 * watchTrack) the video format; YouTube songs come from the streaming engine.
 */
export async function loadTrack(t: { path: string; kind: 'audio' | 'video' }, mode: 'replace' | 'append', start?: number) {
  const opts = [start !== undefined && `start=${Math.max(0, start)}`, t.kind === 'video' && t.path.startsWith('ytdl://') && `ytdl-format=${VIDEO_FORMAT}`]
    .filter(Boolean)
    .join(',')
  const path = await source(t)
  await (opts ? cmd('loadfile', path, mode, -1, opts) : cmd('loadfile', path, mode))
}

export async function load(path: string, mode: 'replace' | 'append' | 'append-play' = 'replace') {
  await cmd('loadfile', path, mode)
  // keep-open pauses at end of file and the pause carries over to the next load
  if (mode === 'replace') await set('pause', false)
}
/** A listening session's guest follows the host: buttons, keys and lyric taps leave playback alone while it does
 *  (session.svelte.ts moves mpv itself). */
export const lock = $state({ following: false })
export const togglePause = () => (lock.following ? Promise.resolve() : set('pause', !mpv.pause))
export const seek = (t: number) => (lock.following ? Promise.resolve() : cmd('seek', t, 'absolute'))
export const stop = () => cmd('stop')
// a new command or property must also be allowed in mpv.rs
export const set = (name: string, value: string | number | boolean) => invoke<void>('mpv_set', { name, value })
export const cmd = (name: string, ...args: (string | number | boolean)[]) => invoke<void>('mpv_command', { name, args })
