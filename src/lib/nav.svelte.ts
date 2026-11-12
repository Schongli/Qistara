import type { Rules } from './smart'
import type { Track } from './library.svelte'

export type Route =
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

class Nav {
  route = $state<Route>({ name: 'albums' })
  query = $state('')
  #back: Route[] = []
  canBack = $state(false)

  go(r: Route) {
    this.#back.push(this.route)
    this.route = r
    this.canBack = true
  }
  /** Sidebar destinations reset history. */
  root(r: Route) {
    this.#back = []
    this.route = r
    this.canBack = false
  }
  back() {
    const r = this.#back.pop()
    if (r) this.route = r
    this.canBack = this.#back.length > 0
  }
}

export const nav = new Nav()


export const ui = $state({
  nowPlaying: false,
  mini: false, // MiniPlayer: the same window, shrunk and kept on top
  queue: false,
  lyrics: false,
  // smart playlist editor: id undefined = creating a new one
  editor: null as null | { id?: number; name: string; rules?: Rules },
  tagEditor: null as null | Track[],
  toast: '',
})

let toastTimer: ReturnType<typeof setTimeout>
/** A short message at the bottom of the window (import results and the like). */
export function toast(msg: string) {
  ui.toast = msg
  clearTimeout(toastTimer)
  toastTimer = setTimeout(() => (ui.toast = ''), 4000)
}
