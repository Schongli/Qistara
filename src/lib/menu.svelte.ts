import { ask } from '@tauri-apps/plugin-dialog'
import { nav, ui } from './nav.svelte'
import { albumKey, albumArtist, createPlaylist, deletePlaylist, lib, love, setPlaylistItems, type Album, type Playlist, type Track } from './library.svelte'
import { queue } from './queue.svelte'
import { isPinned, togglePin } from './pins.svelte'
import { quickDownload } from './downloads.svelte'
import { playItem, startRadio, type Item } from './ytmusic.svelte'
import { evaluate, type Rules } from './smart'
import { exportPlaylist } from './playlistio'
import { linkFor } from './musicvideo.svelte'
import { listenTrack, watchTrack } from './youtube.svelte'

const YT = 'ytdl://' // YouTube tracks (youtube.svelte.ts)

/** keys: a shortcut that does the same, shown dimmed on the right. */
export type MenuItem = { label: string; action: () => void; disabled?: boolean; keys?: string } | 'sep'

export const menu = $state({ open: false, x: 0, y: 0, items: [] as MenuItem[] })

export function openMenu(e: MouseEvent, items: MenuItem[]) {
  e.preventDefault()
  e.stopPropagation()
  Object.assign(menu, { open: true, x: e.clientX, y: e.clientY, items })
}

/** Ctrl+N and the sidebar's + menu. */
export const newPlaylist = async () => nav.root({ name: 'playlist', id: await createPlaylist('Untitled Playlist') })

export const shuffle = (tracks: Track[]) => queue.play(tracks, Math.floor(Math.random() * tracks.length), true)

/**
 * The top of every menu that stands for songs: Play, Shuffle, Play Next, Play Later. `from`: the list the songs were
 * picked from, so Play on one row plays on from there (as a double-click does).
 */
export function playItems(tracks: Track[], from?: { list: Track[]; index: number }): MenuItem[] {
  const none = !tracks.length
  return [
    { label: 'Play', action: () => (from ? queue.play(from.list, from.index) : queue.play(tracks)), disabled: none },
    ...(tracks.length > 1 ? [{ label: 'Shuffle', action: () => shuffle(tracks) }] : []),
    { label: 'Play Next', action: () => queue.playNext(tracks), disabled: none },
    { label: 'Play Later', action: () => queue.playLater(tracks), disabled: none },
  ]
}

/** Add to each playlist, or start a new one, for songs in the library. */
function playlistItems(tracks: Track[], name: string): MenuItem[] {
  const ids = tracks.filter((x) => x.id > 0).map((x) => x.id)
  if (!ids.length) return []
  return [
    ...lib.playlists.filter((p) => !p.rules).map((p) => ({ label: `Add to “${p.name}”`, action: () => setPlaylistItems(p.id, [...p.items, ...ids]) })),
    { label: 'New Playlist from These', action: async () => nav.root({ name: 'playlist', id: await createPlaylist(name, null, ids) }) },
  ]
}

/** Drops separators at either end or next to each other, so optional groups can be left out freely. */
function tidy(items: MenuItem[]): MenuItem[] {
  return items.filter((it, i, a) => it !== 'sep' || (i > 0 && i < a.length - 1 && a[i - 1] !== 'sep'))
}

/**
 * album: the menu is for a whole album, which can be pinned to the sidebar. from: the list the row is in (Play goes on
 * from it). play: other first items (Up Next rows jump instead).
 */
export function trackMenu(tracks: Track[], { album, from, play }: { album?: Album; from?: { list: Track[]; index: number }; play?: MenuItem[] } = {}): MenuItem[] {
  const t = tracks[0]
  const one = tracks.length === 1
  const local = one && t.id > 0 && t.kind === 'audio'
  const yt = tracks.filter((x) => x.path.startsWith(YT))
  const editable = tracks.filter((x) => x.id > 0 && x.kind === 'audio')
  return tidy([
    ...(play ?? playItems(tracks, from)),
    'sep',
    ...(one && yt.length ? [{ label: 'Start Radio', action: () => startRadio(t) }] : []),
    // a YouTube song with or without its picture; a local song with a music video of its own
    ...(one && yt.length ? [t.kind === 'video'
      ? { label: 'Listen Without Video', action: () => playVersion(t, listenTrack(t)) }
      : { label: 'Watch Video', action: () => playVersion(t, watchTrack(t)), keys: queue.current === t ? 'Ctrl+Shift+V' : undefined }] : []),
    ...(local ? [{ label: linkFor(t) ? 'Music Video…' : 'Add Music Video…', action: () => (ui.musicVideo = t) }] : []),
    ...(album ? [{ label: isPinned(album.key) ? 'Unpin from Sidebar' : 'Pin to Sidebar', action: () => togglePin({ key: album.key, title: album.title, art: album.art }) }] : []),
    ...(album ? [{ label: 'Go to Artist', action: () => nav.go({ name: 'artists', artist: album.artist }) }] : []),
    ...(local ? [{ label: 'Go to Album', action: () => nav.go({ name: 'album', key: albumKey(t) }) }, { label: 'Go to Artist', action: () => nav.go({ name: 'artists', artist: albumArtist(t) }) }] : []),
    'sep',
    ...playlistItems(tracks, album?.title ?? (t.album || t.title)),
    'sep',
    ...(one && t.id > 0 ? [{ label: t.loved ? 'Unlove' : 'Love', action: () => love(t) }] : []),
    ...(editable.length ? [{ label: editable.length > 1 ? 'Get Info for Album…' : 'Get Info…', action: () => (ui.tagEditor = editable) }] : []),
    ...(local ? [{ label: "Qur'an: Choose Surah…", action: () => (ui.quranPick = t) }] : []),
    'sep',
    ...(yt.length ? [{ label: yt.length > 1 ? `Download ${yt.length} Songs` : 'Download', action: () => quickDownload(yt.map((x) => x.path.slice(YT.length))) }] : []),
  ])
}

/** Plays the other version of a song: in place when it's the one playing, else on its own. */
function playVersion(t: Track, other: Track | null) {
  if (!other) return
  if (queue.current === t) void queue.swap(other)
  else void queue.play([other])
}

/** A playlist's songs: its items, or what its rules pick from the library. */
export const playlistTracks = (p: Playlist): Track[] =>
  p.rules ? evaluate(lib.tracks, JSON.parse(p.rules) as Rules) : p.items.map((i) => lib.byId.get(i)).filter((t): t is Track => !!t)

export function playlistMenu(p: Playlist): MenuItem[] {
  const tracks = playlistTracks(p)
  return tidy([
    ...playItems(tracks),
    'sep',
    { label: 'Open', action: () => nav.root({ name: 'playlist', id: p.id }) },
    ...(p.rules ? [{ label: 'Edit Rules…', action: () => (ui.editor = { id: p.id, name: p.name, rules: JSON.parse(p.rules!) as Rules }) }] : []),
    { label: 'Export as M3U8…', action: () => exportPlaylist(p.name, tracks), disabled: !tracks.length },
    'sep',
    {
      label: 'Delete Playlist…',
      action: async () => {
        if (!(await ask(`Delete “${p.name}”? Its songs stay in your library.`, { title: 'Delete Playlist', kind: 'warning', okLabel: 'Delete' }))) return
        await deletePlaylist(p.id)
        if (nav.route.name === 'playlist' && nav.route.id === p.id) nav.root({ name: 'albums' })
      },
    },
  ])
}

/** For a group of songs with a page of their own: an artist, a composer, a genre. */
export const groupMenu = (tracks: Track[], open: () => void, name: string): MenuItem[] =>
  tidy([...playItems(tracks), 'sep', { label: 'Open', action: open }, 'sep', ...playlistItems(tracks, name)])

/** A YT Music card: a song, album, playlist or artist. */
export function ytmMenu(i: Item, open: (i: Item) => void): MenuItem[] {
  if (i.kind === 'song') return [{ label: 'Play', action: () => playItem(i) }]
  return [
    { label: i.kind === 'artist' ? 'Play Top Songs' : 'Play', action: () => playItem(i) },
    { label: 'Shuffle', action: () => playItem(i, true) },
    'sep',
    { label: 'Open', action: () => open(i) },
  ]
}
