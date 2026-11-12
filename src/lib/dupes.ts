// Duplicate finder: the same song in several files, best quality first.

type Dupeable = { id: number; kind: string; title: string; artist: string; albumArtist: string; duration: number; lossless: boolean; sampleRate: number | null; bitDepth: number | null; bitrate: number | null }

/** "Song (2011 Remaster) [Live]" and "song - remastered" compare equal; versions in brackets are dropped. */
export const songKey = (s: string) =>
  s.toLowerCase()
    .replace(/[([{][^)\]}]*(remaster|version|edit|mono|stereo|deluxe|bonus)[^)\]}]*[)\]}]/g, '')
    .replace(/\s+-\s+.*remaster.*$/, '')
    .replace(/[^\p{L}\p{N}]+/gu, ' ')
    .trim()

/** Higher is better: lossless, then sample rate, bit depth and bitrate. */
export const rank = (t: Pick<Dupeable, 'lossless' | 'sampleRate' | 'bitDepth' | 'bitrate'>) => (t.lossless ? 1e9 : 0) + (t.sampleRate ?? 0) * 100 + (t.bitDepth ?? 0) * 10 + (t.bitrate ?? 0) / 1000

export function findDuplicates<T extends Dupeable>(tracks: T[]): T[][] {
  const groups = new Map<string, T[]>()
  for (const t of tracks) {
    if (t.kind !== 'audio' || !t.title) continue
    const key = `${songKey(t.albumArtist || t.artist)}|${songKey(t.title)}`
    groups.set(key, [...(groups.get(key) ?? []), t])
  }
  const out: T[][] = []
  for (const g of groups.values()) {
    if (g.length < 2) continue
    const best = [...g].sort((a, b) => rank(b) - rank(a))
    // same song only if the lengths agree (within 3 s), so an intro and a full mix stay apart
    const same = best.filter((t) => !best[0].duration || !t.duration || Math.abs(t.duration - best[0].duration) <= 3)
    if (same.length > 1) out.push(same)
  }
  return out.sort((a, b) => a[0].title.localeCompare(b[0].title))
}
