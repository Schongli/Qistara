// Listening together (session.rs): hosting (share what plays, keep guests in step, chat) and joining (follow the
// host's player, chat). Guests hear the host's own files through the loopback proxy session.rs runs, YouTube from
// YouTube, and radio from the station itself.
import { invoke } from '@tauri-apps/api/core'
import { listen } from '@tauri-apps/api/event'
import { cmd, lock, mpv, set, SHOWS_VIDEO } from './player.svelte'
import { queue } from './queue.svelte'
import { remoteTrack } from './radio.svelte'
import { serverSongId } from './lyricsources'
import { toast } from './nav.svelte'
import { videoId, videoTrack, watchTrack } from './youtube.svelte'
import { linkFor, mv, sessionLinks } from './musicvideo.svelte'
import { parseVideoId } from './videolink'
import { plan, type HostItem, type HostState } from './session'
import type { Track } from './library.svelte'

export interface Msg { from: string; text: string; at: number; me?: boolean }

export const session = $state({
  role: '' as '' | 'host' | 'guest',
  invites: [] as string[],
  host: '',
  guests: [] as { id: number; name: string }[],
  chat: [] as Msg[],
  busy: false,
  error: '',
})

const say = (m: Msg) => (session.chat = [...session.chat.slice(-199), m])

listen<Msg>('session-chat', (e) => say(e.payload))
listen<{ host: string; guests: { id: number; name: string }[] }>('session-roster', (e) => {
  session.host = e.payload.host
  session.guests = e.payload.guests
})
listen<{ reason: string }>('session-ended', (e) => {
  if (session.role !== 'guest') return
  toast(e.payload.reason)
  void ended()
})

// ---- hosting ----

let seqs = new WeakMap<Track, number>()
let nextSeq = 1
const seqOf = (t: Track) => seqs.get(t) ?? (seqs.set(t, nextSeq++), nextSeq - 1)
const artCache = new Map<string, Promise<string | null>>()

/** A queue track as guests get it; null for what can't be shared (a server song carries your sign-in). */
async function itemOf(t: Track): Promise<HostItem | null> {
  const base = { seq: seqOf(t), title: t.title, artist: t.artist, album: t.album, duration: t.duration }
  const yt = videoId(t)
  if (yt) return { ...base, kind: 'yt', id: yt, video: t.kind === 'video', art: t.art?.startsWith('https://') ? t.art : null }
  if (t.id > 0) {
    if (t.art && !artCache.has(t.art)) artCache.set(t.art, invoke<string | null>('session_art', { art: t.art }).catch(() => null))
    const art = t.art ? await artCache.get(t.art)! : null
    // a YouTube music video showing over the song goes along (one from your library stays here: it isn't shared)
    const link = linkFor(t)
    const shown = link?.yt && mv.on ? { mv: { yt: link.yt, offset: link.offset } } : {}
    return { ...base, kind: 'file', art, codec: t.codec, sampleRate: t.sampleRate, bitDepth: t.bitDepth, lossless: t.lossless, ...shown }
  }
  if (/^https?:\/\//.test(t.path) && !serverSongId(t.path)) return { ...base, kind: 'url', url: t.path }
  return null
}

const shareOf = (t: Track, i: HostItem) =>
  i.kind === 'file' ? { kind: 'file', seq: i.seq, track: t.id } : i.kind === 'yt' ? { kind: 'yt', seq: i.seq, id: i.id } : { kind: 'url', seq: i.seq, url: i.url }

let last = ''
/** Tells guests what plays: on every change, and every few seconds so they stay in step. */
async function publish(force = false) {
  if (session.role !== 'host') return
  const cur = queue.current, next = queue.appended
  const [ci, ni] = await Promise.all([cur ? itemOf(cur) : null, next ? itemOf(next) : null])
  const key = JSON.stringify([ci?.seq, ni?.seq, mpv.pause])
  if (!force && key === last) return
  last = key
  const shares = [cur && ci && shareOf(cur, ci), next && ni && shareOf(next, ni)].filter(Boolean)
  try {
    await invoke('session_share', { items: shares })
    const state: HostState = { cur: ci, next: ni, pos: mpv.time, paused: mpv.pause }
    await invoke('session_state', { state })
  } catch (e) {
    session.error = String(e)
  }
}

let beat: ReturnType<typeof setInterval> | undefined
$effect.root(() => {
  $effect(() => {
    // what's playing, what's next, paused or not: tell the guests now
    void [queue.current, queue.appended, mpv.pause, session.role]
    void publish()
  })
  // a seek on the host: a jump in time that isn't playback
  let prev = 0
  $effect(() => {
    const t = mpv.time
    if (session.role === 'host' && Math.abs(t - prev) > 2) void publish(true)
    prev = t
  })
})

export async function host(name: string, port: number | null) {
  session.busy = true
  session.error = ''
  try {
    const r = await invoke<{ port: number; invites: string[] }>('session_host', { name, port })
    Object.assign(session, { role: 'host', invites: r.invites, host: name, guests: [], chat: [] })
    seqs = new WeakMap()
    last = ''
    beat = setInterval(() => void publish(true), 5000)
    await publish(true)
  } catch (e) {
    session.error = String(e)
  } finally {
    session.busy = false
  }
}

export async function kick(id: number) {
  try {
    session.invites = await invoke<string[]>('session_kick', { id })
    toast('Removed. The invite changed: share the new one with anyone joining later.')
  } catch (e) {
    toast(String(e))
  }
}

// ---- joining ----

let proxy = ''
let received = 0 // when the last state arrived (performance.now())
let hostState: HostState | null = null
const bySeq = new Map<number, Track>()

/** One track per host item for the whole session (stable ids keep the now-playing mark and the queue steady). */
function trackOf(i: HostItem): Track {
  const known = bySeq.get(i.seq)
  if (known) return withVideo(known, i)
  const meta = { title: i.title, artist: i.artist, album: i.album, duration: i.duration, ...(i.art && { art: i.art }) }
  const t =
    i.kind === 'yt' && i.id ? (i.video && SHOWS_VIDEO ? watchTrack(videoTrack(i.id, meta))! : videoTrack(i.id, meta))
    : i.kind === 'url' && i.url ? remoteTrack({ path: i.url, ...meta })
    : remoteTrack({
        path: `${proxy}${i.seq}`, ...meta, codec: i.codec ?? 'Stream', sampleRate: i.sampleRate ?? null,
        bitDepth: i.bitDepth ?? null, lossless: !!i.lossless,
      })
  bySeq.set(i.seq, t)
  return withVideo(t, i)
}

/** The host's music video over a song: shown here too once the guest's own yt-dlp finds it (an id, checked). */
function withVideo(t: Track, i: HostItem): Track {
  const yt = i.mv && parseVideoId(i.mv.yt)
  if (yt && Number.isFinite(i.mv!.offset)) sessionLinks.set(t.id, { yt, offset: Math.max(-30, Math.min(30, i.mv!.offset)) })
  else sessionLinks.delete(t.id)
  return t
}

let applying = false
async function follow() {
  if (session.role !== 'guest' || !hostState || applying) return
  applying = true
  try {
    const h = hostState
    const since = (performance.now() - received) / 1000
    const cur = queue.current, nx = queue.appended
    const seqFor = (t: Track | null) => (t ? ([...bySeq].find(([, v]) => v === t)?.[0] ?? null) : null)
    const mine = { curSeq: seqFor(cur), nextSeq: seqFor(nx), time: mpv.time, duration: mpv.duration, paused: mpv.pause }
    for (const a of plan(mine, h, since)) {
      if (a.do === 'stop') await cmd('stop')
      else if (a.do === 'load') await queue.follow(trackOf(h.cur!), h.next ? trackOf(h.next) : null, a.at)
      else if (a.do === 'seek') await cmd('seek', a.at, 'absolute')
      else if (a.do === 'pause') await set('pause', a.paused)
      else if (a.do === 'next') await queue.followNext(h.next ? trackOf(h.next) : null)
    }
  } catch (e) {
    console.warn('session:', e)
  } finally {
    applying = false
  }
}

listen<{ state: HostState; lag: number }>('session-state', (e) => {
  hostState = e.payload.state
  received = performance.now() - e.payload.lag
  void follow()
})

export async function join(code: string, name: string) {
  session.busy = true
  session.error = ''
  try {
    const r = await invoke<{ host: string; guests: { id: number; name: string }[]; state: HostState | null; proxy: string }>('session_join', { code: code.trim(), name })
    proxy = r.proxy
    bySeq.clear()
    Object.assign(session, { role: 'guest', host: r.host, guests: r.guests, chat: [] })
    lock.following = true
    if (r.state?.cur) {
      hostState = r.state
      received = performance.now()
      await follow()
    }
  } catch (e) {
    session.error = String(e)
  } finally {
    session.busy = false
  }
}

// ---- both ----

export const chat = (text: string) => invoke('session_chat', { text }).catch((e) => toast(String(e)))

async function ended() {
  const was = session.role
  Object.assign(session, { role: '', invites: [], guests: [], host: '' })
  clearInterval(beat)
  hostState = null
  if (was === 'guest') await queue.unfollow()
}

export async function leave() {
  await invoke('session_leave').catch(() => {})
  await ended()
}
