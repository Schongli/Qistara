// Where lyrics come from, in the order you set in Settings › Lyrics. Local files always stay on this PC; the
// online sources get only the title, artist, album and length (lyrics_online.rs).
import type { Lyrics } from './lyrics'

export type LyricsSource = 'local' | 'subsonic' | 'lrclib' | 'netease' | 'kugou'
export interface SourceSetting { id: LyricsSource; on: boolean }

export const SOURCES: Record<LyricsSource, { label: string; hint: string }> = {
  local: { label: 'Local files', hint: '.ttml or .lrc next to the song, or lyrics in its tags' },
  subsonic: { label: 'Navidrome / Subsonic', hint: 'Your server’s lyrics for songs played from it' },
  lrclib: { label: 'LRCLIB', hint: 'Open, community-made synced lyrics' },
  netease: { label: 'NetEase Cloud Music', hint: 'Line-synced; strong for Chinese and Japanese music' },
  kugou: { label: 'Kugou', hint: 'Word-synced karaoke lyrics; strong for Asian music' },
}

export const DEFAULT_SOURCES: readonly SourceSetting[] = [
  { id: 'local', on: true }, { id: 'subsonic', on: true }, { id: 'lrclib', on: true }, { id: 'netease', on: false }, { id: 'kugou', on: false },
]

/** The stored list made whole: your order and switches kept, unknown entries dropped, new sources appended.
 *  `onlineLyrics: false` (the old single switch) turns the online sources off the first time. */
export function normalizeSources(stored: unknown, onlineLyrics?: boolean): SourceSetting[] {
  const known = new Set(DEFAULT_SOURCES.map((s) => s.id))
  const offline = !Array.isArray(stored) && onlineLyrics === false
  const out: SourceSetting[] = []
  for (const s of Array.isArray(stored) ? stored : []) {
    if (s && known.has(s.id) && !out.some((o) => o.id === s.id)) out.push({ id: s.id, on: !!s.on })
  }
  for (const d of DEFAULT_SOURCES) {
    if (!out.some((o) => o.id === d.id)) out.push({ ...d, on: d.on && (!offline || d.id === 'local') })
  }
  return out
}

export const wordSynced = (l: Lyrics) => l.lines.some((x) => x.words)

/** Asks each switched-on source in order and takes the first with lyrics. With `preferWords`, a line-synced
 *  find is kept as the fallback while the rest are asked for word timing. */
export async function firstLyrics(
  sources: SourceSetting[], get: (id: LyricsSource) => Promise<Lyrics | null>, preferWords: boolean,
): Promise<{ lyrics: Lyrics; source: LyricsSource } | null> {
  let fallback: { lyrics: Lyrics; source: LyricsSource } | null = null
  for (const s of sources) {
    if (!s.on) continue
    const lyrics = await get(s.id).catch(() => null)
    if (!lyrics?.lines.length) continue
    if (!preferWords || wordSynced(lyrics)) return { lyrics, source: s.id }
    fallback ??= { lyrics, source: s.id }
  }
  return fallback
}

/** A song played from your server: its id, read from the stream address subsonic_stream made. */
export function serverSongId(path: string): string | null {
  if (!/^https?:\/\//i.test(path)) return null
  try {
    const u = new URL(path)
    return /\/rest\/stream(\.view)?$/.test(u.pathname) ? u.searchParams.get('id') : null
  } catch {
    return null
  }
}
