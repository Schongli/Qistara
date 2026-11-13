// Albums pinned to the sidebar, in kv like the rest of the app state; loaded before anything is saved.
import { kvGet, kvSet } from './persist'

export interface Pin { key: string; title: string; art: string | null }

export const pins = $state({ items: [] as Pin[], loaded: false })

kvGet<Pin[]>('pins').then((p) => {
  pins.items = p ?? []
  pins.loaded = true
})

export const isPinned = (key: string) => pins.items.some((p) => p.key === key)

export function togglePin(p: Pin) {
  if (!pins.loaded) return
  pins.items = isPinned(p.key) ? pins.items.filter((x) => x.key !== p.key) : [...pins.items, p]
  kvSet('pins', $state.snapshot(pins.items))
}
