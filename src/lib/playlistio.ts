// Playlist import/export as M3U8.
import { invoke } from '@tauri-apps/api/core'
import { open, save } from '@tauri-apps/plugin-dialog'
import { createPlaylist, lib, type Track } from './library.svelte'
import { buildM3u, parseM3u } from './m3u'
import { nav, toast } from './nav.svelte'
import { plural } from './format'

export async function exportPlaylist(name: string, tracks: Track[]) {
  const path = await save({ title: 'Export playlist', defaultPath: `${name.replace(/[\\/:*?"<>|]/g, '_')}.m3u8`, filters: [{ name: 'M3U8 playlist', extensions: ['m3u8', 'm3u'] }] })
  if (!path) return
  try {
    await invoke('write_playlist', { path, text: buildM3u(tracks) })
    toast(`Exported ${plural(tracks.length, 'song')}`)
  } catch (e) {
    toast(String(e))
  }
}

export async function importPlaylist() {
  const file = await open({ multiple: false, title: 'Import playlist', filters: [{ name: 'Playlists', extensions: ['m3u8', 'm3u'] }] })
  if (typeof file !== 'string') return
  try {
    const text = await invoke<string>('read_text', { path: file })
    const dir = file.replace(/[\\/][^\\/]*$/, '')
    const byPath = new Map(lib.tracks.map((t) => [t.path.toLowerCase(), t.id]))
    const entries = parseM3u(text, dir)
    const ids = entries.map((e) => byPath.get(e.path.toLowerCase())).filter((id): id is number => id !== undefined)
    const name = file.split(/[\\/]/).pop()!.replace(/\.m3u8?$/i, '')
    const id = await createPlaylist(name, null, ids)
    nav.root({ name: 'playlist', id })
    const missing = entries.length - ids.length
    toast(`Imported ${plural(ids.length, 'song')}${missing ? `; ${missing} not in your library` : ''}`)
  } catch (e) {
    toast(String(e))
  }
}
