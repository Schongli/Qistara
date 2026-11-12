// Last.fm: "now playing" when a song starts, a scrobble once it counts as played (stats.svelte.ts).
import { invoke } from '@tauri-apps/api/core'
import { untrack } from 'svelte'
import { queue } from './queue.svelte'
import { whenPlayed } from './stats.svelte'
import type { Track } from './library.svelte'

export const lastfm = $state({ configured: false, user: null as string | null, waiting: false, error: '', pending: 0 })

export async function refresh() {
  const s = await invoke<{ configured: boolean; user: string | null }>('lastfm_status').catch(() => null)
  if (s) Object.assign(lastfm, s)
}
refresh()

// Last.fm only accepts tracks over 30 s with an artist and title
const eligible = (t: Track) => t.kind === 'audio' && !!t.artist && !!t.title && (t.duration === 0 || t.duration > 30)
const payload = (t: Track, timestamp = 0) => ({
  s: { artist: t.artist, track: t.title, album: t.album, duration: Math.round(t.duration), timestamp },
})

$effect.root(() => {
  $effect(() => {
    const t = queue.current
    untrack(() => {
      if (t && lastfm.user && eligible(t)) invoke('lastfm_now_playing', payload(t)).catch(() => {})
    })
  })
})

whenPlayed((t, startedAt) => {
  if (!lastfm.user || !eligible(t)) return
  invoke<number>('lastfm_scrobble', payload(t, Math.floor(startedAt)))
    .then((n) => (lastfm.pending = n))
    .catch((e) => (lastfm.error = String(e)))
})

export async function connect() {
  lastfm.error = ''
  try {
    await invoke('lastfm_begin')
    lastfm.waiting = true
  } catch (e) {
    lastfm.error = String(e)
  }
}

export async function finish() {
  try {
    lastfm.user = await invoke<string>('lastfm_finish')
    lastfm.waiting = false
    lastfm.error = ''
  } catch (e) {
    lastfm.error = `${e}. Approve Sonora on the Last.fm page first, then try again.`
  }
}
