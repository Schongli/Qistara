import { convertFileSrc, invoke } from '@tauri-apps/api/core'
import { listen } from '@tauri-apps/api/event'
import { byName, collator } from './format'

export interface Track {
  id: number
  path: string
  kind: 'audio' | 'video'
  title: string
  artist: string
  albumArtist: string
  album: string
  genre: string
  composer: string
  year: number | null
  track: number | null
  disc: number | null
  duration: number
  codec: string
  sampleRate: number | null
  bitDepth: number | null
  channels: number | null
  bitrate: number | null
  lossless: boolean
  art: string | null
  rating: number
  loved: boolean
  plays: number
  skips: number
  lastPlayed: number | null
  addedAt: number
}

export interface Album {
  key: string
  title: string
  artist: string
  year: number | null
  genre: string
  art: string | null
  tracks: Track[]
  addedAt: number
  duration: number
}

export interface Playlist {
  id: number
  name: string
  rules: string | null // JSON smart rules; null = manual
  items: number[]
}

export interface Artist {
  name: string
  albums: Album[]
  art: string | null
}

class Library {
  // raw: 50k tracks must not become 50k deep proxies; updates replace the array
  tracks = $state.raw<Track[]>([])
  folders = $state.raw<string[]>([])
  playlists = $state.raw<Playlist[]>([])
  artDir = $state('')
  loaded = $state(false)
  scan = $state<{ done: number; total: number } | null>(null)
  error = $state('')

  #grouped = $derived(group(this.tracks))
  get albums() { return this.#grouped.albums }
  get artists() { return this.#grouped.artists }
  get genres() { return this.#grouped.genres }
  songs = $derived(this.tracks.filter((t) => t.kind === 'audio').sort((a, b) => collator.compare(a.title, b.title)))
  videos = $derived(this.tracks.filter((t) => t.kind === 'video').sort((a, b) => collator.compare(a.title, b.title)))
  recent = $derived([...this.#grouped.albums].sort((a, b) => b.addedAt - a.addedAt))
  byId = $derived(new Map(this.tracks.map((t) => [t.id, t])))
  composers = $derived.by(() => {
    const m = new Map<string, Track[]>()
    for (const t of this.tracks) if (t.kind === 'audio' && t.composer) m.set(t.composer, [...(m.get(t.composer) ?? []), t])
    return [...m.entries()].sort((a, b) => byName(a[0], b[0])).map(([name, tracks]) => ({ name, tracks }))
  })

  async load() {
    try {
      const l = await invoke<{ tracks: Track[]; folders: string[]; playlists: Playlist[]; artDir: string }>('library')
      this.tracks = l.tracks
      this.folders = l.folders
      this.playlists = l.playlists
      this.artDir = l.artDir
      this.loaded = true
    } catch (e) {
      this.error = String(e)
    }
  }
}

export const lib = new Library()

/** Replaces one track in place so every derived view updates, without a reload. */
export function patchTrack(id: number, patch: Partial<Track>) {
  lib.tracks = lib.tracks.map((t) => (t.id === id ? { ...t, ...patch } : t))
}

export async function rate(t: Track, rating: number) {
  patchTrack(t.id, { rating })
  await invoke('rate', { id: t.id, rating })
}

export async function love(t: Track, loved = !t.loved) {
  patchTrack(t.id, { loved })
  await invoke('love', { id: t.id, loved })
}

export async function createPlaylist(name: string, rules: string | null = null, items: number[] = []) {
  const id = await invoke<number>('playlist_create', { name, rules, items })
  lib.playlists = [...lib.playlists, { id, name, rules, items }]
  return id
}

export async function updatePlaylist(id: number, name: string, rules: string | null) {
  lib.playlists = lib.playlists.map((p) => (p.id === id ? { ...p, name, rules } : p))
  await invoke('playlist_update', { id, name, rules })
}

export async function setPlaylistItems(id: number, items: number[]) {
  lib.playlists = lib.playlists.map((p) => (p.id === id ? { ...p, items } : p))
  await invoke('playlist_items', { id, items })
}

export async function deletePlaylist(id: number) {
  lib.playlists = lib.playlists.filter((p) => p.id !== id)
  await invoke('playlist_delete', { id })
}

listen<[number, number]>('scan-progress', (e) => (lib.scan = { done: e.payload[0], total: e.payload[1] }))
listen('library-changed', () => {
  lib.scan = null
  lib.load()
})
listen<string>('scan-error', (e) => {
  lib.scan = null
  lib.error = e.payload
})

export const artUrl = (art: string | null | undefined) =>
  art && lib.artDir ? convertFileSrc(`${lib.artDir}\\${art}`) : null

export const addFolder = (path: string) => invoke('add_folder', { path })
export const removeFolder = (path: string) => invoke('remove_folder', { path })
export const rescan = () => invoke('rescan')
export const largeCover = (path: string) =>
  invoke<string | null>('cover', { path }).then((p) => (p ? convertFileSrc(p) : null))

let extensions: Promise<{ audio: string[]; video: string[] }> | null = null
/** Supported file extensions, from the scanner so the lists never drift. */
export const mediaExtensions = () => (extensions ??= invoke('extensions'))

let adHoc = -1
/** A track for a file outside the library (opened or dropped); reuses the library entry when there is one. */
export async function fileTrack(path: string): Promise<Track | null> {
  const known = lib.tracks.find((t) => t.path.toLowerCase() === path.toLowerCase())
  if (known) return known
  const ext = path.split('.').pop()?.toLowerCase() ?? ''
  const { audio, video } = await mediaExtensions()
  const kind = video.includes(ext) ? 'video' : audio.includes(ext) ? 'audio' : null
  if (!kind) return null
  const title = path.split(/[\\/]/).pop()!.replace(/\.[^.]+$/, '')
  return {
    id: adHoc--, path, kind, title, artist: '', albumArtist: '', album: '', genre: '', composer: '', year: null,
    track: null, disc: null, duration: 0, codec: ext.toUpperCase(), sampleRate: null, bitDepth: null, channels: null,
    bitrate: null, lossless: false, art: null, rating: 0, loved: false, plays: 0, skips: 0, lastPlayed: null, addedAt: 0,
  }
}

export const albumArtist = (t: Track) => t.albumArtist || t.artist || 'Unknown Artist'
export const albumKey = (t: Track) => `${albumArtist(t).toLowerCase()}\u0000${(t.album || 'Unknown Album').toLowerCase()}`

const trackOrder = (x: Track, y: Track) =>
  (x.disc ?? 1) - (y.disc ?? 1) || (x.track ?? 999) - (y.track ?? 999) || collator.compare(x.title, y.title)

export function group(tracks: Track[]) {
  const albums = new Map<string, Album>()
  for (const t of tracks) {
    if (t.kind !== 'audio') continue
    const key = albumKey(t)
    let a = albums.get(key)
    if (!a) {
      a = { key, title: t.album || 'Unknown Album', artist: albumArtist(t), year: null, genre: '', art: null, tracks: [], addedAt: 0, duration: 0 }
      albums.set(key, a)
    }
    a.tracks.push(t)
    a.year ??= t.year
    a.genre ||= t.genre
    a.art ??= t.art
    a.addedAt = Math.max(a.addedAt, t.addedAt)
    a.duration += t.duration
  }
  const list = [...albums.values()]
  for (const a of list) a.tracks.sort(trackOrder)
  list.sort((x, y) => byName(x.artist, y.artist) || (x.year ?? 0) - (y.year ?? 0) || collator.compare(x.title, y.title))

  const artists = new Map<string, Artist>()
  const genres = new Map<string, Album[]>()
  for (const a of list) {
    const k = a.artist.toLowerCase()
    let ar = artists.get(k)
    if (!ar) artists.set(k, (ar = { name: a.artist, albums: [], art: null }))
    ar.albums.push(a)
    ar.art ??= a.art
    const g = a.genre || 'Unknown Genre'
    genres.set(g, [...(genres.get(g) ?? []), a])
  }
  return {
    albums: list,
    artists: [...artists.values()].sort((x, y) => byName(x.name, y.name)),
    genres: [...genres.entries()].sort((x, y) => collator.compare(x[0], y[0])).map(([name, albums]) => ({ name, albums })),
  }
}

export function search(q: string) {
  const s = q.trim().toLowerCase()
  if (!s) return { songs: [], albums: [], artists: [], videos: [] }
  const has = (...v: string[]) => v.some((x) => x.toLowerCase().includes(s))
  return {
    artists: lib.artists.filter((a) => has(a.name)).slice(0, 12),
    albums: lib.albums.filter((a) => has(a.title, a.artist)).slice(0, 24),
    songs: lib.songs.filter((t) => has(t.title, t.artist, t.album, t.composer)).slice(0, 100),
    videos: lib.videos.filter((t) => has(t.title)).slice(0, 24),
  }
}
