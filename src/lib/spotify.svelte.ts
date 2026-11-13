// Spotify: the connection, playlist import (matched to your own files) and the Spotify Connect remote.
// Sign-in, tokens and every request live in Rust (spotify.rs); this side only shows and decides.
import { invoke } from '@tauri-apps/api/core'
import { listen } from '@tauri-apps/api/event'
import { mpv, togglePause } from './player.svelte'
import { queue } from './queue.svelte'
import { kvSet } from './persist'
import type { RemoteTrack } from './spotify'
import { importInto, type ImportResult } from './imports.svelte'

export interface SpPlaylist { id: string; name: string; tracks: number; image: string | null }
export interface SpPlayer {
  device: string; playing: boolean; progress: number; volume: number | null
  title: string; artists: string; album: string; image: string | null; duration: number; url: string | null
}
export type { ImportResult }
export type Control =
  | { kind: 'play' | 'pause' | 'next' | 'previous' }
  | { kind: 'seek'; ms: number }
  | { kind: 'volume'; percent: number }

/** Liked Songs has no playlist id; this stands in for it. */
export const LIKED = 'liked'
const IMPORTS = 'spotifyImports' // Spotify playlist id → Confluence Player playlist id, so importing again updates

export const spotify = $state({
  configured: false,
  user: null as string | null,
  waiting: false,
  error: '',
  liked: 0,
  playlists: [] as SpPlaylist[],
  loading: false,
  importing: '',
  results: {} as Record<string, ImportResult>,
  player: null as SpPlayer | null,
  readAt: 0, // performance.now() when `player` was read, to move progress on between polls
  remoteError: '',
})

export async function refresh() {
  const s = await invoke<{ configured: boolean; user: string | null }>('spotify_status').catch(() => null)
  if (s) Object.assign(spotify, s)
}
refresh()

// the sign-in finishes in the browser; Rust reports back with null (connected) or an error
listen<string | null>('spotify-changed', async (e) => {
  spotify.waiting = false
  spotify.error = e.payload ?? ''
  await refresh()
})

const message = (e: unknown) => (e instanceof Error ? e.message : String(e))

export async function setClient(clientId: string) {
  spotify.error = ''
  try {
    await invoke('spotify_set_client', { clientId })
    await refresh()
  } catch (e) {
    spotify.error = message(e)
  }
}

export async function connect() {
  spotify.error = ''
  try {
    await invoke('spotify_begin')
    spotify.waiting = true
  } catch (e) {
    spotify.error = message(e)
  }
}

/** Forgets the account and what was imported from it; imported playlists stay, they're your own files.
 *  `all` also forgets the Client ID, to set up a different developer app. */
export async function disconnect(all = false) {
  await invoke('spotify_logout', { all })
  await kvSet(IMPORTS, {})
  Object.assign(spotify, { user: null, playlists: [], liked: 0, results: {}, player: null, waiting: false, error: '' })
  if (all) spotify.configured = false
}

export async function loadPlaylists() {
  spotify.loading = true
  spotify.error = ''
  try {
    const r = await invoke<{ liked: number; playlists: SpPlaylist[] }>('spotify_playlists')
    spotify.liked = r.liked
    spotify.playlists = r.playlists
  } catch (e) {
    spotify.error = message(e)
  } finally {
    spotify.loading = false
  }
}

/** Copies one playlist (or Liked Songs) into Confluence Player as a playlist of your matching files. */
export async function importPlaylist(id: string, name: string) {
  spotify.importing = id
  try {
    const songs = await invoke<RemoteTrack[]>('spotify_tracks', { playlist: id === LIKED ? null : id })
    spotify.results = { ...spotify.results, [id]: await importInto(IMPORTS, id, name, songs) }
  } catch (e) {
    spotify.results = { ...spotify.results, [id]: { found: 0, total: 0, missing: [], error: message(e) } }
  } finally {
    spotify.importing = ''
  }
}

export async function pollPlayer() {
  try {
    spotify.player = await invoke<SpPlayer | null>('spotify_player')
    spotify.readAt = performance.now()
    spotify.remoteError = ''
  } catch (e) {
    spotify.remoteError = message(e)
  }
}

/** Polls every 3 s while a view needs it and the window is visible: developer apps have a request quota. */
export function watchPlayer() {
  const tick = () => document.visibilityState === 'visible' && pollPlayer()
  tick()
  const timer = setInterval(tick, 3000)
  return () => clearInterval(timer)
}

export async function control(action: Control) {
  // in Bit-Perfect mode Confluence Player holds the audio device alone, so Spotify on this PC couldn't play over it
  if (action.kind === 'play' && queue.current && !mpv.pause) await togglePause()
  const p = spotify.player
  if (p && (action.kind === 'play' || action.kind === 'pause')) p.playing = action.kind === 'play' // answer the click at once
  try {
    await invoke('spotify_control', { action })
    setTimeout(pollPlayer, 500)
  } catch (e) {
    spotify.remoteError = message(e)
  }
}

let volumeTimer: ReturnType<typeof setTimeout>
/** The wheel and slider send many values; Spotify gets the last one. */
export function setRemoteVolume(v: number) {
  const percent = Math.round(v)
  if (spotify.player) spotify.player.volume = percent
  clearTimeout(volumeTimer)
  volumeTimer = setTimeout(() => control({ kind: 'volume', percent }), 250)
}
