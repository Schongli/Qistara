// Apple Music and Tidal: sign-in and playlist import, matched to your own files. Both work the same way
// (apple.rs, tidal.rs: <id>_status, _begin, _logout, _playlists, _tracks and an <id>-changed event).
import { invoke } from '@tauri-apps/api/core'
import { listen } from '@tauri-apps/api/event'
import { kvSet } from './persist'
import { importInto, type ImportResult } from './imports.svelte'
import type { RemoteTrack } from './spotify'

export type ServiceId = 'apple' | 'tidal'
export interface RemotePlaylist { id: string; name: string; tracks: number; image: string | null }
/** The whole collection (Apple: library songs, Tidal: favourite tracks) stands in a playlist's place. */
export const ALL = 'all'

function service(id: ServiceId, label: string, allLabel: string) {
  const s = $state({
    id, label, allLabel, configured: false, user: null as string | null, redirect: '', waiting: false, error: '',
    playlists: [] as RemotePlaylist[], loading: false, importing: '', results: {} as Record<string, ImportResult>,
  })
  const IMPORTS = `${id}Imports`
  const refresh = async () => {
    const st = await invoke<{ configured: boolean; user: string | null; redirect?: string }>(`${id}_status`).catch(() => null)
    if (st) Object.assign(s, st)
  }
  refresh()
  listen<string | null>(`${id}-changed`, async (e) => {
    s.waiting = false
    s.error = e.payload ?? ''
    await refresh()
  })
  return {
    s,
    async setup(args: Record<string, string>) {
      s.error = ''
      try {
        await invoke(id === 'apple' ? 'apple_setup' : 'tidal_set_client', args)
        await refresh()
      } catch (e) {
        s.error = String(e)
      }
    },
    async connect() {
      s.error = ''
      try {
        await invoke(`${id}_begin`)
        s.waiting = true
      } catch (e) {
        s.error = String(e)
      }
    },
    /** Forgets the sign-in and the import map; imported playlists stay (they are your files). `all`: the key too. */
    async disconnect(all = false) {
      await invoke(`${id}_logout`, { all })
      await kvSet(IMPORTS, {})
      Object.assign(s, { user: null, playlists: [], results: {}, waiting: false, error: '' })
      if (all) s.configured = false
    },
    async load() {
      s.loading = true
      s.error = ''
      try {
        s.playlists = (await invoke<{ playlists: RemotePlaylist[] }>(`${id}_playlists`)).playlists
      } catch (e) {
        s.error = String(e)
      } finally {
        s.loading = false
      }
    },
    async importOne(pid: string, name: string) {
      s.importing = pid
      try {
        const songs = await invoke<RemoteTrack[]>(`${id}_tracks`, { playlist: pid === ALL ? null : pid })
        s.results = { ...s.results, [pid]: await importInto(IMPORTS, pid, name, songs) }
      } catch (e) {
        s.results = { ...s.results, [pid]: { found: 0, total: 0, missing: [], error: String(e) } }
      } finally {
        s.importing = ''
      }
    },
  }
}

export const apple = service('apple', 'Apple Music', 'Library Songs')
export const tidal = service('tidal', 'Tidal', 'My Tracks')
export const services = { apple, tidal }
