// Your music server (subsonic.rs): browsed as albums, artists, playlists and search, played straight from
// the server. Songs become remote tracks whose path is the signed stream URL (the file as stored, so FLAC
// stays bit-perfect); covers go through the art cache.
import { invoke } from '@tauri-apps/api/core'
import { remoteTrack } from './radio.svelte'
import type { Track } from './library.svelte'

export interface Account { server: string; user: string }
export const server = $state({ account: null as Account | null, checked: false, error: '', busy: false })
invoke<Account | null>('subsonic_status').then((a) => {
  server.account = a
  server.checked = true
})

export interface SSong {
  id: string; title: string; artist?: string; album?: string; track?: number; discNumber?: number; duration?: number; year?: number
  genre?: string; suffix?: string; bitRate?: number; samplingRate?: number; bitDepth?: number; channelCount?: number; coverArt?: string
}
export interface SAlbum { id: string; name: string; artist?: string; year?: number; coverArt?: string; songCount?: number; song?: SSong[] }
export interface SArtist { id: string; name: string; albumCount?: number; coverArt?: string; album?: SAlbum[] }
export interface SPlaylist { id: string; name: string; songCount?: number; coverArt?: string; entry?: SSong[] }

type Reply = Record<string, any> // eslint-disable-line @typescript-eslint/no-explicit-any
const call = (c: object) => invoke<Reply>('subsonic_call', { call: c })

export const albumList = async (kind: 'newest' | 'alphabeticalByName' | 'frequent' | 'recent', offset = 0) =>
  ((await call({ call: 'albumList', kind, offset })).albumList2?.album ?? []) as SAlbum[]
export const getAlbum = async (id: string) => (await call({ call: 'album', id })).album as SAlbum
export const getArtists = async () => ((await call({ call: 'artists' })).artists?.index ?? []).flatMap((i: { artist?: SArtist[] }) => i.artist ?? []) as SArtist[]
export const getArtist = async (id: string) => (await call({ call: 'artist', id })).artist as SArtist
export const getPlaylists = async () => ((await call({ call: 'playlists' })).playlists?.playlist ?? []) as SPlaylist[]
export const getPlaylist = async (id: string) => (await call({ call: 'playlist', id })).playlist as SPlaylist
export async function search(query: string) {
  const r = (await call({ call: 'search', query })).searchResult3 ?? {}
  return { artists: (r.artist ?? []) as SArtist[], albums: (r.album ?? []) as SAlbum[], songs: (r.song ?? []) as SSong[] }
}

// one fetch per cover per session; the bytes land in the art cache, named by their content
const covers = new Map<string, Promise<string | null>>()
export const cover = (id: string | undefined) => {
  if (!id) return Promise.resolve(null)
  if (!covers.has(id)) covers.set(id, invoke<string | null>('subsonic_cover', { id }).catch(() => null))
  return covers.get(id)!
}

const LOSSLESS = new Set(['flac', 'alac', 'wav', 'aiff', 'aif', 'ape', 'wv', 'dsf', 'dff'])

/** Songs as tracks the queue can play. */
export const toTracks = (songs: SSong[]): Promise<Track[]> =>
  Promise.all(
    songs.map(async (s) => {
      const [path, art] = await Promise.all([invoke<string>('subsonic_stream', { id: s.id }), cover(s.coverArt)])
      const suffix = (s.suffix ?? '').toLowerCase()
      return remoteTrack({
        path, title: s.title, artist: s.artist ?? '', albumArtist: s.artist ?? '', album: s.album ?? '', track: s.track ?? null,
        disc: s.discNumber ?? null, duration: s.duration ?? 0, year: s.year ?? null, genre: s.genre ?? '', codec: suffix.toUpperCase() || 'Server',
        sampleRate: s.samplingRate ?? null, bitDepth: s.bitDepth ?? null, channels: s.channelCount ?? null, bitrate: s.bitRate ?? null,
        lossless: LOSSLESS.has(suffix), art,
      })
    }),
  )

export async function signIn(address: string, user: string, password: string) {
  server.error = ''
  server.busy = true
  try {
    server.account = await invoke<Account>('subsonic_connect', { server: address, user, password })
  } catch (e) {
    server.error = String(e)
  } finally {
    server.busy = false
  }
}

export async function signOut() {
  await invoke('subsonic_logout')
  server.account = null
  covers.clear()
}
