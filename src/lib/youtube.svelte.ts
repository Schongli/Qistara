// YouTube through yt-dlp (ytdlp.rs): search results, pasted links and Liked Music become tracks whose
// path is ytdl://<watch url>, which mpv's ytdl hook plays with our yt-dlp (player.svelte.ts).
import { invoke } from '@tauri-apps/api/core'
import { remoteTrack } from './radio.svelte'
import { ytCookies } from './downloads.svelte'
import { engine, mpv, onMpvEvent } from './player.svelte'
import { queue } from './queue.svelte'
import { toast } from './nav.svelte'
import { settings } from './settings.svelte'
import type { Track } from './library.svelte'

type Video = { id: string; title: string; channel: string; duration: number | null }

export const yt = $state({ tracks: [] as Track[], title: '', loading: false, error: '' })

// one track per video for the whole session: the same song found twice keeps its id, so the now-playing mark,
// play-next checks and the queue all recognise it
const byVideo = new Map<string, Track>()

/** The track for a YouTube video id; what's known about it (a YT Music album and square art) fills in what isn't. */
export function videoTrack(id: string, info: Partial<Track> & { title: string }): Track {
  const known = byVideo.get(id)
  if (known) {
    if (info.album && known.album === 'YouTube') Object.assign(known, { album: info.album, art: info.art ?? known.art })
    return known
  }
  const t = remoteTrack({
    path: `ytdl://https://www.youtube.com/watch?v=${id}`, album: 'YouTube', codec: 'YouTube', art: `https://i.ytimg.com/vi/${id}/mqdefault.jpg`, ...info,
  })
  byVideo.set(id, t)
  return t
}

const watching = new Map<string, Track>()

/** The video version of a YouTube song: the same page played with its picture (player.svelte.ts loadTrack). */
export function watchTrack(t: Track): Track | null {
  const id = videoId(t)
  if (!id) return null
  if (t.kind === 'video') return t
  let w = watching.get(id)
  if (!w) {
    const { id: _, ...song } = t
    watching.set(id, (w = remoteTrack({ ...song, kind: 'video' })))
  }
  return w
}

/** Back to the song, sound only. */
export function listenTrack(t: Track): Track | null {
  const id = videoId(t)
  return id ? videoTrack(id, { title: t.title, artist: t.artist, duration: t.duration }) : null
}

export const ytTrack = (v: Video) => videoTrack(v.id, { title: v.title, artist: v.channel, duration: v.duration ?? 0 })
/** The video id of a YouTube track, if it is one. */
export const videoId = (t: Track) => t.path.match(/^ytdl:\/\/https:\/\/www\.youtube\.com\/watch\?v=([\w-]{11})$/)?.[1] ?? null

async function run(title: string, cmd: string, args: Record<string, string>) {
  yt.loading = true
  yt.error = ''
  try {
    yt.tracks = (await invoke<Video[]>(cmd, { ...args, cookies: ytCookies() })).map(ytTrack)
    yt.title = title
  } catch (e) {
    yt.error = String(e)
  } finally {
    yt.loading = false
  }
}

/** Searches, or opens the text as a link when it is one. */
export const lookUp = (text: string) =>
  /^https?:\/\//i.test(text.trim()) ? run('From your link', 'youtube_list', { url: text.trim() }) : run(`Results for “${text.trim()}”`, 'youtube_search', { query: text })
export const likedMusic = () => run('Liked Music', 'youtube_list', { url: 'liked' })

// the same sign-in for playback as for search and downloads: the streaming engine's yt-dlp fallback, and mpv.rs
// sets the hook's yt-dlp options
$effect.root(() => {
  $effect(() => {
    const c = settings.ytCookies
    engine.cookies = ytCookies()
    if (!mpv.ready) return
    invoke('mpv_ytdl_sign_in', { cookies: c }).catch((e) => console.warn('YouTube playback sign-in:', e))
  })
})

// a YouTube song the streaming engine couldn't open plays through mpv's ytdl hook instead; one that fails there
// too says so rather than go silent
onMpvEvent(async (e) => {
  const t = queue.current
  if (e.event !== 'end-file' || String(e.reason) !== 'error' || !t?.path.startsWith('ytdl://')) return
  if (t.kind === 'audio' && !engine.viaHook.has(t.path)) {
    engine.viaHook.add(t.path)
    const missing = await invoke('youtube_ready').then(() => '', String)
    return missing ? toast(`Couldn't open this song. ${missing}`) : void queue.retry()
  }
  toast(`Couldn't play this YouTube video${e.file_error ? ` (${e.file_error})` : ''}. Check for a yt-dlp update in Downloads.`)
})
