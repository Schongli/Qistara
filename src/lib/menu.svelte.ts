import { nav, ui } from './nav.svelte'
import { albumKey, albumArtist, createPlaylist, lib, love, setPlaylistItems, type Track } from './library.svelte'
import { queue } from './queue.svelte'

export type MenuItem = { label: string; action: () => void; disabled?: boolean } | 'sep'

export const menu = $state({ open: false, x: 0, y: 0, items: [] as MenuItem[] })

export function openMenu(e: MouseEvent, items: MenuItem[]) {
  e.preventDefault()
  e.stopPropagation()
  Object.assign(menu, { open: true, x: e.clientX, y: e.clientY, items })
}

export function trackMenu(tracks: Track[]): MenuItem[] {
  const t = tracks[0]
  const items: MenuItem[] = [
    { label: 'Play Next', action: () => queue.playNext(tracks) },
    { label: 'Play Later', action: () => queue.playLater(tracks) },
  ]
  const ids = tracks.filter((x) => x.id > 0).map((x) => x.id)
  if (ids.length) {
    items.push('sep')
    for (const p of lib.playlists.filter((p) => !p.rules)) {
      items.push({ label: `Add to “${p.name}”`, action: () => setPlaylistItems(p.id, [...p.items, ...ids]) })
    }
    items.push({ label: 'New Playlist from Selection', action: async () => nav.root({ name: 'playlist', id: await createPlaylist(t.album || t.title, null, ids) }) })
  }
  if (tracks.length === 1 && t.id > 0) items.push({ label: t.loved ? 'Unlove' : 'Love', action: () => love(t) })
  const editable = tracks.filter((x) => x.id > 0 && x.kind === 'audio')
  if (editable.length) items.push({ label: editable.length > 1 ? 'Get Info for Album…' : 'Get Info…', action: () => (ui.tagEditor = editable) })
  if (tracks.length === 1 && t.kind === 'audio') {
    items.push('sep',
      { label: 'Go to Album', action: () => nav.go({ name: 'album', key: albumKey(t) }) },
      { label: 'Go to Artist', action: () => nav.go({ name: 'artists', artist: albumArtist(t) }) })
  }
  return items
}
