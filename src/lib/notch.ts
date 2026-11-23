// What Qistara shares with notch (a separate top-of-screen app) about the playing track's lyrics.
// The shape is a contract with notch's reader: keep it backwards compatible, or bump `v`.
import type { Lyrics } from './lyrics'

export interface NotchWord { t: number; end: number; text: string }
export interface NotchLine { t: number; end: number; text: string; tr?: string; words?: NotchWord[] }
/** Times in seconds. notch matches title, artist and duration against Windows' media controls. */
export interface NotchLyrics { v: 1; title: string; artist: string; duration: number; rtl: boolean; lines: NotchLine[] }

/** Synced lyrics with their translation (one per line, null = none) and word timing when the source has it. */
export function lyricLines(lyrics: Lyrics | null, tr: (string | null)[] | null): NotchLine[] {
  if (!lyrics?.synced) return []
  return lyrics.lines.map((l, i) => ({
    t: l.t,
    end: l.end,
    text: l.text,
    ...(tr?.[i] ? { tr: tr[i]! } : {}),
    ...(l.words?.length ? { words: l.words.map((w) => ({ t: w.t, end: w.end, text: w.text })) } : {}),
  }))
}

/** One unit of a recitation (istiadha, basmala or an ayah), already turned into text. */
export interface RecitedUnit { text: string; tr?: string; words?: NotchWord[] }

/** A recitation as lines: each unit runs from its start until the next one starts (the last until the end). */
export function quranLines(units: RecitedUnit[], starts: number[] | null, duration: number): NotchLine[] {
  if (!starts || starts.length !== units.length) return []
  return units.map((u, i) => ({
    t: starts[i],
    end: starts[i + 1] ?? Math.max(duration, starts[i]),
    text: u.text,
    ...(u.tr ? { tr: u.tr } : {}),
    ...(u.words?.length ? { words: u.words } : {}),
  }))
}
