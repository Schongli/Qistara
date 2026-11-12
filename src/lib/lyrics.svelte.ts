// Finds lyrics for the playing track: local first, then LRCLIB (when online lookup is on).
import { invoke } from '@tauri-apps/api/core'
import { fetch } from '@tauri-apps/plugin-http'
import { untrack } from 'svelte'
import { parseLyrics, type Lyrics } from './lyrics'
import { queue } from './queue.svelte'
import { settings } from './settings.svelte'
import type { Track } from './library.svelte'

export const lyr = $state({ data: null as Lyrics | null, source: '', loading: false })

const UA = 'Sonora/0.1.0' // LRCLIB asks clients to identify themselves; no personal data

async function online(t: Track): Promise<string | null> {
  const q = new URLSearchParams({ track_name: t.title, artist_name: t.artist, album_name: t.album })
  if (t.duration) q.set('duration', String(Math.round(t.duration)))
  const r = await fetch(`https://lrclib.net/api/get?${q}`, { headers: { 'User-Agent': UA } })
  if (r.status === 404) return ''
  if (!r.ok) return null // server trouble: try again next time, don't cache
  const j = (await r.json()) as { syncedLyrics?: string; plainLyrics?: string; instrumental?: boolean }
  return j.syncedLyrics || j.plainLyrics || ''
}

async function load(t: Track) {
  lyr.loading = true
  lyr.data = null
  lyr.source = ''
  try {
    const found = await invoke<{ kind: string; text: string; source: string } | null>('lyrics', { path: t.path })
    let text = found?.text ?? null
    let kind = found?.kind ?? 'lrc'
    let source = found?.source ?? ''
    if (text === null && settings.onlineLyrics && t.kind === 'audio' && t.artist && t.title) {
      text = await online(t).catch(() => null)
      if (text !== null) await invoke('lyrics_store', { path: t.path, text })
      kind = 'lrc'
      source = 'lrclib'
    }
    if (queue.current?.path !== t.path) return // the track changed while we were looking
    lyr.data = text?.trim() ? parseLyrics(kind, text) : null
    lyr.source = source
  } finally {
    if (queue.current?.path === t.path) lyr.loading = false
  }
}

$effect.root(() => {
  $effect(() => {
    const t = queue.current
    if (t) untrack(() => load(t))
    else lyr.data = null
  })
})
