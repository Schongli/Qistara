// Bridge to libmpv: observed properties land in `mpv`, commands go straight through.
import {
  command, destroy, init, listenEvents, observeProperties, setProperty,
  type MpvObservableProperty,
} from 'tauri-plugin-libmpv-api'
import { appLocalDataDir, join } from '@tauri-apps/api/path'
import { invoke } from '@tauri-apps/api/core'
import type { AudioParams } from './signal'

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
})

const KEY: Record<(typeof PROPS)[number][0], keyof typeof mpv> = {
  pause: 'pause', 'idle-active': 'idle', 'time-pos': 'time', duration: 'duration', path: 'path',
  'media-title': 'title', 'playlist-pos': 'playlistPos', metadata: 'metadata', 'audio-params': 'audioIn',
  'audio-out-params': 'audioOut', 'audio-codec-name': 'codec', 'current-ao': 'ao', af: 'af', volume: 'volume',
  'ao-volume': 'aoVolume', mute: 'mute', speed: 'speed', 'track-list': 'tracks', 'chapter-list': 'chapters',
  chapter: 'chapter', 'video-params': 'video', 'hwdec-current': 'hwdec', 'audio-device-list': 'devices',
  'audio-device': 'device', 'sub-delay': 'subDelay', 'sub-scale': 'subScale', 'ab-loop-a': 'abA',
  'ab-loop-b': 'abB', 'container-fps': 'fps', 'video-codec': 'videoCodec', 'video-bitrate': 'videoBitrate',
  'audio-bitrate': 'audioBitrate', 'file-format': 'container', 'video-aspect-override': 'aspect', panscan: 'panscan',
  'eof-reached': 'eof',
}
const EMPTY: Partial<Record<keyof typeof mpv, unknown>> = { time: 0, duration: 0, af: [], tracks: [], chapters: [], devices: [], abA: 'no', abB: 'no', eof: false }

// Bit-perfect profile: WASAPI exclusive at the file's own rate/format, nothing in between.
export const BIT_PERFECT = {
  ao: 'wasapi', 'audio-exclusive': 'yes', af: '', volume: 100, 'volume-max': 100, replaygain: 'no', speed: 1,
} as const

type Handler = (e: { event: string; [k: string]: unknown }) => void
const handlers = new Set<Handler>()
export const onMpvEvent = (h: Handler) => (handlers.add(h), () => handlers.delete(h))

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
    const ytdlp = await join(await appLocalDataDir(), 'bin', 'yt-dlp.exe')
    await init({
      initialOptions: {
        ...BIT_PERFECT,
        vo: 'gpu-next', hwdec: 'auto-safe', 'target-colorspace-hint': 'yes', 'video-sync': 'audio',
        idle: 'yes', 'force-window': 'no', 'keep-open': 'yes',
        'gapless-audio': 'weak', 'prefetch-playlist': 'yes', 'audio-display': 'no', 'cover-art-auto': 'no',
        'input-default-bindings': 'no', 'input-vo-keyboard': 'no', osc: 'no', 'osd-level': 0,
        'sub-auto': 'fuzzy', 'msg-level': 'all=warn',
        // YouTube tracks are ytdl://… (youtube.svelte.ts): the hook runs our own yt-dlp (ytdlp.rs). Other URLs,
        // radio included, are opened by mpv itself; length-quoted so a comma in the path can't split the list.
        ytdl: 'yes', 'ytdl-format': 'bestaudio/best', 'ytdl-raw-options': 'js-runtimes=node',
        'script-opts': `ytdl_hook-ytdl_path=%${new TextEncoder().encode(ytdlp).length}%${ytdlp},ytdl_hook-all_formats=no`,
        'watch-later-options': 'start,aid,sid,sub-delay,audio-delay,sub-scale',
        'screenshot-format': 'png', 'screenshot-template': 'Confluence Player %F %wH.%wM.%wS',
      },
      observedProperties: PROPS,
    })
    await setProperty('watch-later-dir', await join(await appLocalDataDir(), 'resume'))
    // dev builds keep mpv's own log (format negotiation, device errors) next to the library
    if (import.meta.env.DEV) await setProperty('log-file', await join(await appLocalDataDir(), 'mpv.log'))
    // optional: a missing Pictures folder must not stop playback from starting
    invoke<string>('screenshot_dir')
      .then((d) => setProperty('screenshot-directory', d))
      .catch((e) => console.warn('screenshots stay in the working folder:', e))
    mpv.ready = true
  } catch (e) {
    mpv.error = String(e)
    console.error('mpv start failed:', e)
  }
}

export async function load(path: string, mode: 'replace' | 'append' | 'append-play' = 'replace') {
  await command('loadfile', [path, mode])
  // keep-open pauses at end of file and the pause carries over to the next load
  if (mode === 'replace') await setProperty('pause', false)
}
export const togglePause = () => setProperty('pause', !mpv.pause)
export const seek = (t: number) => command('seek', [t, 'absolute'])
export const stop = () => command('stop')
export const set = (name: string, value: string | number | boolean) => setProperty(name, value)
export const cmd = (name: string, ...args: (string | number | boolean)[]) => command(name, args)
