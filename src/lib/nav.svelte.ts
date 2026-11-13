import type { Rules } from './smart'
import type { Track } from './library.svelte'

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
  | { name: 'settings' }
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
  | { name: 'providers' }
  | { name: 'server' }
  | { name: 'apple' }
  | { name: 'tidal' }

class Nav {
  route = $state<Route>({ name: 'home' })
  query = $state('')
  #back: Route[] = []
  #forward: Route[] = []
  canBack = $state(false)
  canForward = $state(false)

  #sync() {
    this.canBack = this.#back.length > 0
    this.canForward = this.#forward.length > 0
  }
  go(r: Route) {
    this.#back.push(this.route)
    this.#forward = []
    this.route = r
    this.#sync()
  }
  /** Sidebar destinations reset history. */
  root(r: Route) {
    this.#back = []
    this.#forward = []
    this.route = r
    this.#sync()
  }
  back() {
    const r = this.#back.pop()
    if (r) this.#forward.push(this.route), (this.route = r)
    this.#sync()
  }
  forward() {
    const r = this.#forward.pop()
    if (r) this.#back.push(this.route), (this.route = r)
    this.#sync()
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
  panel: '' as '' | 'lyrics' | 'queue',
  // bumped to focus the search field (Ctrl+F, the sidebar's Search)
  search: 0,
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
