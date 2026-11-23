// Music videos over local songs. The song keeps playing from its own file (lossless, bit-perfect, with its play
// count and lyrics); the video, from YouTube or from your library, is added on top as a picture-only track
// (mpv.rs video-add) and kept in step by an offset you set. The same switch shows a YouTube song's own video.
import { invoke } from '@tauri-apps/api/core'
import { cmd, mpv, onMpvEvent, set } from './player.svelte'
import { queue } from './queue.svelte'
import { kvLoad, kvSet } from './persist'
import { lib, type Track } from './library.svelte'
import { ytCookies } from './downloads.svelte'
import { toast } from './nav.svelte'
import { listenTrack, videoId, watchTrack } from './youtube.svelte'
export { parseVideoId } from './videolink'

/** yt: a YouTube video id; video: a video in the library; offset: seconds the song starts after the video does. */
export interface MusicVideo { yt?: string; video?: number; offset: number }

export const mv = $state({
  /** by song path */
  links: {} as Record<string, MusicVideo>,
  /** show linked videos while their songs play */
  on: true,
  loaded: false,
  /** a video is over the playing song now */
  showing: false,
  busy: false,
})

kvLoad<{ links: Record<string, MusicVideo>; on: boolean }>('musicVideos').then((v) => {
  if (v) Object.assign(mv, { links: v.links ?? {}, on: v.on ?? true })
  mv.loaded = true
})
const save = () => mv.loaded && kvSet('musicVideos', { links: $state.snapshot(mv.links), on: mv.on })

/** A listening session's host can show a video over a song its guests stream (session.svelte.ts), by track id. */
export const sessionLinks = new Map<number, MusicVideo>()

export const linkFor = (t: Track | null): MusicVideo | null => (t ? (t.id > 0 ? mv.links[t.path] : sessionLinks.get(t.id)) ?? null : null)

export function setLink(t: Track, link: MusicVideo | null) {
  if (link) mv.links[t.path] = link
  else delete mv.links[t.path]
  save()
  if (queue.current?.path === t.path) void (link ? attach(t) : detach())
}

// YouTube's stream addresses last some hours: ask yt-dlp once per video in a while
const urls = new Map<string, { url: string; at: number }>()
async function streamOf(id: string): Promise<string> {
  const hit = urls.get(id)
  if (hit && Date.now() - hit.at < 30 * 60_000) return hit.url
  const url = await invoke<string>('youtube_video_url', { id, cookies: ytCookies() })
  urls.set(id, { url, at: Date.now() })
  return url
}

let attaching = 0
/** Adds the song's video over it, in step. */
async function attach(t: Track) {
  const link = linkFor(t)
  if (!link || !mv.on) return detach()
  const mine = ++attaching
  mv.busy = true
  try {
    const src = link.yt ? await streamOf(link.yt) : lib.byId.get(link.video ?? 0)?.path
    if (!src) throw new Error('The linked video is no longer in your library.')
    if (mine !== attaching || queue.current !== t) return
    await set('audio-delay', link.offset)
    await cmd('video-add', src, 'select')
    mv.showing = true
  } catch (e) {
    toast(`Couldn't show the music video: ${e instanceof Error ? e.message : e}`)
  } finally {
    if (mine === attaching) mv.busy = false
  }
}

/** Hides the video and puts the song's timing back. */
async function detach() {
  attaching++
  mv.busy = false
  if (mv.showing) await set('vid', 'no').catch(() => {})
  mv.showing = false
  if (mpv.audioDelay) await set('audio-delay', 0).catch(() => {})
}

// each song decides for itself: a linked one gets its video, any other plays without delay
onMpvEvent((e) => {
  if (e.event !== 'file-loaded') return
  mv.showing = false
  const t = queue.current
  if (t && linkFor(t)) void attach(t)
  else void detach()
})

/** Whether the playing song has a video to show: its own (YouTube) or a linked one. */
export const hasVideo = (t: Track | null) => !!t && (!!videoId(t) || !!linkFor(t))
/** Whether that video shows now. */
export const videoShowing = (t: Track | null) => !!t && ((!!videoId(t) && t.kind === 'video') || (!!linkFor(t) && mv.showing))

/** Ctrl+Shift+V and the player bar's Video button: the playing song with or without its video. */
export async function toggleVideo(): Promise<boolean> {
  const t = queue.current
  if (!t) return false
  if (videoId(t)) {
    const other = t.kind === 'video' ? listenTrack(t) : watchTrack(t)
    if (other) await queue.swap(other)
    return true
  }
  if (!linkFor(t)) return false
  mv.on = !videoShowing(t)
  save()
  await (mv.on ? attach(t) : detach())
  return true
}

/** The sync control: seconds the song starts after the video (positive: the video has an intro). */
export async function nudge(by: number | null) {
  const t = queue.current
  const link = t && t.id > 0 ? mv.links[t.path] : null
  if (!link) return
  link.offset = by === null ? 0 : Math.round((link.offset + by) * 10) / 10
  save()
  await set('audio-delay', link.offset)
}
