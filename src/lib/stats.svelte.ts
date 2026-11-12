// Counts plays and skips from what was actually heard (seeking doesn't count as listening).
import { invoke } from '@tauri-apps/api/core'
import { untrack } from 'svelte'
import { mpv } from './player.svelte'
import { queue } from './queue.svelte'
import { patchTrack, type Track } from './library.svelte'

type Listener = (t: Track, startedAt: number) => void
const onPlayed = new Set<Listener>()
/** Called once per playthrough when a track counts as played (Last.fm uses this). */
export const whenPlayed = (f: Listener) => (onPlayed.add(f), () => onPlayed.delete(f))

/** A play counts at half the track or 4 minutes, whichever comes first (Last.fm's rule). */
export const threshold = (duration: number) => Math.min(Math.max(duration, 30) / 2, 240)

let cur: Track | null = null
let heard = 0
let counted = false
let last = 0
let startedAt = 0

function finish() {
  // left between 2 s and the play threshold: a skip
  if (cur && cur.id > 0 && !counted && heard > 2) {
    invoke('skipped', { id: cur.id }).catch(() => {})
    patchTrack(cur.id, { skips: cur.skips + 1 })
  }
}

$effect.root(() => {
  $effect(() => {
    const t = queue.current
    untrack(() => {
      if (t?.id === cur?.id) return
      finish()
      cur = t
      heard = 0
      counted = false
      last = mpv.time
      startedAt = Date.now() / 1000
    })
  })
  $effect(() => {
    const time = mpv.time
    untrack(() => {
      const d = time - last
      last = time
      if (!cur || mpv.pause) return
      if (time < 2 && d < -5) {
        // looped back to the start (repeat one): a new playthrough
        heard = 0
        counted = false
        startedAt = Date.now() / 1000
        return
      }
      if (d > 0 && d < 1.5) heard += d
      const dur = mpv.duration || cur.duration
      // short tracks (an intro, an interlude) count once heard to the end
      if (!counted && dur > 0 && heard >= Math.min(threshold(dur), Math.max(dur - 1, dur * 0.9))) {
        counted = true
        const t = cur
        if (t.id > 0) {
          invoke<number>('played', { id: t.id })
            .then((at) => patchTrack(t.id, { plays: t.plays + 1, lastPlayed: at }))
            .catch(() => {})
        }
        onPlayed.forEach((f) => f(t, startedAt))
      }
    })
  })
})
