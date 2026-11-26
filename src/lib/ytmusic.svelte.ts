// YT Music (ytmusic.rs), as a guest or signed in (account.svelte.ts): what the Home shelves, your Library, search,
// album, artist and playlist pages show, and turning songs into tracks. Songs play from the streaming engine (ytstream.rs, player.svelte.ts loadTrack).
// Everything fetched stays in memory for the session (never in the database), like a browser tab would.
import { invoke } from '@tauri-apps/api/core'
import { queue } from './queue.svelte'
import { warm } from './player.svelte'
import { account } from './account.svelte'
import { toast } from './nav.svelte'
import { videoId, videoTrack } from './youtube.svelte'
import type { Track } from './library.svelte'

export interface Ref { name: string; id: string | null }
export interface Item {
  kind: 'song' | 'album' | 'artist' | 'playlist'
  id: string
  title: string
  subtitle: string
  artists: Ref[]
  album: Ref | null
  duration: number | null
  thumb: string | null
}
export interface Shelf { title: string; items: Item[] }
export interface Page { title: string; subtitle: string; artists: Ref[]; thumb: string | null; tracks: Item[]; shelves: Shelf[] }
export type Filter = '' | 'songs' | 'albums' | 'artists' | 'playlists'

/** YT Music answers in Indonesian for Indonesian systems, in English otherwise. */
const hl = () => (navigator.language.toLowerCase().startsWith('id') ? 'id' : 'en')

export const artistNames = (i: { artists: Ref[] }) => i.artists.map((a) => a.name).join(', ')

/** A song as a track the queue can play; the same video is always the same track. */
export const songTrack = (i: Item): Track =>
  videoTrack(i.id, { title: i.title, artist: artistNames(i), album: i.album?.name ?? 'YT Music', duration: i.duration ?? 0, ...(i.thumb && { art: i.thumb }) })

export const ytm = $state({ home: null as Shelf[] | null, loading: false, error: '', library: null as Shelf[] | null, libraryError: '' })

const pages = new Map<string, Promise<Page>>()
const searches = new Map<string, Promise<Shelf[]>>()
const once = <T>(cache: Map<string, Promise<T>>, key: string, load: () => Promise<T>) => {
  if (!cache.has(key)) cache.set(key, load().catch((e) => (cache.delete(key), Promise.reject(e))))
  return cache.get(key)!
}

export async function loadHome(again = false) {
  if (ytm.home && !again) return
  ytm.loading = true
  ytm.error = ''
  try {
    ytm.home = await invoke<Shelf[]>('ytm_home', { hl: hl() })
  } catch (e) {
    ytm.error = String(e)
  } finally {
    ytm.loading = false
  }
}

/** Your library (signed in): Liked Music and your playlists, then saved albums and followed artists. */
export async function loadLibrary(again = false) {
  if (ytm.library && !again) return
  ytm.libraryError = ''
  try {
    ytm.library = await invoke<Shelf[]>('ytm_library', { hl: hl() })
  } catch (e) {
    ytm.libraryError = String(e)
  }
}

// signing in or out changes what YT Music shows: Home is yours or a guest's, and pages may differ
let who: string | null | undefined
$effect.root(() => {
  $effect(() => {
    const now = account.info?.name ?? null
    if (who !== undefined && who !== now) {
      pages.clear()
      searches.clear()
      ytm.library = null
      if (ytm.home) void loadHome(true)
    }
    who = now
  })
})

export const search = (query: string, filter: Filter) =>
  once(searches, `${filter}:${query.trim().toLowerCase()}`, () => invoke<Shelf[]>('ytm_search', { query, filter: filter || null, hl: hl() }))

/** An album, artist or playlist page; its first song is fetched ahead, as it's the one most often played. */
export const page = (kind: 'album' | 'artist' | 'playlist', id: string) =>
  once(pages, `${kind}:${id}`, () => invoke<Page>(`ytm_${kind}`, { id, hl: hl() }).then((p) => (p.tracks[0] && warm([songTrack(p.tracks[0])]), p)))

/** Plays a song, or the songs of an album, playlist or artist (their top songs). */
export async function playItem(i: Item, shuffle = false) {
  try {
    if (i.kind === 'song') return queue.play([songTrack(i)])
    const p = await page(i.kind, i.id)
    if (!p.tracks.length) return toast('Nothing to play here.')
    await queue.play(p.tracks.map(songTrack), shuffle ? Math.floor(Math.random() * p.tracks.length) : 0, shuffle)
  } catch (e) {
    toast(String(e))
  }
}

/** YT Music's radio for a song: songs that follow on from it. */
export async function radioFor(t: Track): Promise<Track[]> {
  const id = videoId(t)
  if (!id) return []
  return (await invoke<Item[]>('ytm_radio', { id, hl: hl() })).map(songTrack)
}

export async function startRadio(t: Track) {
  try {
    const more = await radioFor(t)
    await queue.play([t, ...more.filter((m) => m.id !== t.id)])
  } catch (e) {
    toast(String(e))
  }
}

// Autoplay ∞ keeps going after a YouTube song with YT Music's radio for it
queue.remoteAutoplay = (t) => radioFor(t)
