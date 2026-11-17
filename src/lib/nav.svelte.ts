import type { Rules } from './smart'
import type { Track } from './library.svelte'
import type { Restore, Snap } from './scrollmemo'

export type Route =
  | { name: 'home' }
  | { name: 'recent' }
  | { name: 'albums' }
  | { name: 'songs' }
  | { name: 'artists'; artist?: string }
  | { name: 'genres'; genre?: string }
  | { name: 'videos' }
  | { name: 'album'; key: string }
  | { name: 'search' }
  | { name: 'settings'; tab?: string }
  | { name: 'playlist'; id: number }
  | { name: 'loved' }
  | { name: 'recentPlayed' }
  | { name: 'top' }
  | { name: 'eq' }
  | { name: 'radio' }
  | { name: 'composers'; composer?: string }
  | { name: 'duplicates' }
  | { name: 'spotify' }
  | { name: 'vocabulary' }
  | { name: 'downloads' }
  | { name: 'youtube' }
  | { name: 'ytm' }
  | { name: 'ytmAlbum'; id: string }
  | { name: 'ytmArtist'; id: string }
  | { name: 'ytmPlaylist'; id: string }
  | { name: 'providers' }
  | { name: 'server' }
  | { name: 'apple' }
  | { name: 'tidal' }

interface Entry { route: Route; snap?: Snap }

class Nav {
  route = $state<Route>({ name: 'home' })
  query = $state('')
  #back: Entry[] = []
  #forward: Entry[] = []
  canBack = $state(false)
  canForward = $state(false)
  /** Set by App: the scroll positions of the page about to be left, so Back can return to them. */
  capture: () => Snap | undefined = () => undefined
  /** What the page now showing should do with its scroll position (read by App after each change). */
  restore: Restore = 'top'

  #set(r: Route, restore: Restore) {
    this.restore = restore
    this.route = r
    this.canBack = this.#back.length > 0
    this.canForward = this.#forward.length > 0
  }
  go(r: Route) {
    this.#back.push({ route: this.route, snap: this.capture() })
    this.#forward = []
    this.#set(r, 'top')
  }
  /** Sidebar destinations reset history. */
  root(r: Route) {
    this.#back = []
    this.#forward = []
    this.#set(r, 'top')
  }
  /** The same page with a different selection (the artist picked in Artists): no history, no scrolling. */
  replace(r: Route) {
    this.#set(r, 'keep')
  }
  back() {
    const e = this.#back.pop()
    if (!e) return
    this.#forward.push({ route: this.route, snap: this.capture() })
    this.#set(e.route, e.snap ?? 'top')
  }
  forward() {
    const e = this.#forward.pop()
    if (!e) return
    this.#back.push({ route: this.route, snap: this.capture() })
    this.#set(e.route, e.snap ?? 'top')
  }
}

export const nav = new Nav()


export const ui = $state({
  nowPlaying: false,
  mini: false, // MiniPlayer: the same window, shrunk and kept on top
  // Now Playing's side pane
  queue: false,
  lyrics: false,
  // the docked pane beside the library
  panel: '' as '' | 'lyrics' | 'queue' | 'session',
  // bumped to focus the search field (Ctrl+F, the sidebar's Search); Ctrl+F in Settings searches settings
  search: 0,
  settingsSearch: 0,
  // smart playlist editor: id undefined = creating a new one
  editor: null as null | { id?: number; name: string; rules?: Rules },
  tagEditor: null as null | Track[],
  // "Qur'an: Choose Surah…" for this track
  quranPick: null as null | Track,
  toast: '',
})

let toastTimer: ReturnType<typeof setTimeout>
/** A short message at the bottom of the window (import results and the like). */
export function toast(msg: string) {
  ui.toast = msg
  clearTimeout(toastTimer)
  toastTimer = setTimeout(() => (ui.toast = ''), 4000)
}
