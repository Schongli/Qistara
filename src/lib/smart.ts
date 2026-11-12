// Smart playlists (iTunes / MusicBee style): rules over track fields, evaluated live in memory.
import type { Track } from './library.svelte'

export type TextField = 'title' | 'artist' | 'album' | 'albumArtist' | 'genre' | 'composer' | 'codec'
export type NumField = 'year' | 'rating' | 'plays' | 'skips' | 'duration' | 'sampleRate' | 'bitDepth' | 'bitrate'
export type DateField = 'addedAt' | 'lastPlayed'
export type BoolField = 'loved' | 'lossless'
export type Field = TextField | NumField | DateField | BoolField

export type Op =
  | 'is' | 'isNot' | 'contains' | 'notContains' | 'startsWith'
  | 'eq' | 'gt' | 'lt' | 'between'
  | 'inLast' | 'notInLast'
  | 'isTrue' | 'isFalse'

export interface Rule { field: Field; op: Op; value: string | number; value2?: number; unit?: 'days' | 'weeks' | 'months' }
export type LimitBy = 'random' | 'mostPlayed' | 'leastPlayed' | 'recentlyAdded' | 'recentlyPlayed' | 'highestRated' | 'title'
export interface Rules { match: 'all' | 'any'; rules: Rule[]; limit?: { count: number; by: LimitBy }; seed?: number }

export const FIELDS: Record<Field, { label: string; type: 'text' | 'num' | 'date' | 'bool' }> = {
  title: { label: 'Title', type: 'text' }, artist: { label: 'Artist', type: 'text' }, album: { label: 'Album', type: 'text' },
  albumArtist: { label: 'Album Artist', type: 'text' }, genre: { label: 'Genre', type: 'text' }, composer: { label: 'Composer', type: 'text' },
  codec: { label: 'Format', type: 'text' }, year: { label: 'Year', type: 'num' }, rating: { label: 'Rating', type: 'num' },
  plays: { label: 'Plays', type: 'num' }, skips: { label: 'Skips', type: 'num' }, duration: { label: 'Time (s)', type: 'num' },
  sampleRate: { label: 'Sample Rate (Hz)', type: 'num' }, bitDepth: { label: 'Bit Depth', type: 'num' }, bitrate: { label: 'Bitrate (kbps)', type: 'num' },
  addedAt: { label: 'Date Added', type: 'date' }, lastPlayed: { label: 'Last Played', type: 'date' },
  loved: { label: 'Loved', type: 'bool' }, lossless: { label: 'Lossless', type: 'bool' },
}

export const OPS: Record<'text' | 'num' | 'date' | 'bool', [Op, string][]> = {
  text: [['contains', 'contains'], ['notContains', 'does not contain'], ['is', 'is'], ['isNot', 'is not'], ['startsWith', 'starts with']],
  num: [['eq', 'is'], ['gt', 'is greater than'], ['lt', 'is less than'], ['between', 'is in the range']],
  date: [['inLast', 'is in the last'], ['notInLast', 'is not in the last']],
  bool: [['isTrue', 'is true'], ['isFalse', 'is false']],
}

const UNIT = { days: 86400, weeks: 604800, months: 2629800 }

export function test(t: Track, r: Rule, now: number): boolean {
  const v = t[r.field]
  switch (r.op) {
    case 'contains': return String(v ?? '').toLowerCase().includes(String(r.value).toLowerCase())
    case 'notContains': return !String(v ?? '').toLowerCase().includes(String(r.value).toLowerCase())
    case 'is': return String(v ?? '').toLowerCase() === String(r.value).toLowerCase()
    case 'isNot': return String(v ?? '').toLowerCase() !== String(r.value).toLowerCase()
    case 'startsWith': return String(v ?? '').toLowerCase().startsWith(String(r.value).toLowerCase())
    case 'eq': return v != null && Number(v) === Number(r.value)
    case 'gt': return v != null && Number(v) > Number(r.value)
    case 'lt': return v != null && Number(v) < Number(r.value)
    case 'between': return v != null && Number(v) >= Number(r.value) && Number(v) <= Number(r.value2 ?? r.value)
    case 'inLast': return v != null && Number(v) >= now - Number(r.value) * UNIT[r.unit ?? 'days']
    case 'notInLast': return v == null || Number(v) < now - Number(r.value) * UNIT[r.unit ?? 'days']
    case 'isTrue': return !!v
    case 'isFalse': return !v
  }
}

// stable "random" per playlist, so a live playlist doesn't reshuffle on every library change
const mix = (id: number, seed: number) => {
  let x = (id ^ seed) >>> 0
  x = Math.imul(x ^ (x >>> 16), 0x45d9f3b)
  x = Math.imul(x ^ (x >>> 16), 0x45d9f3b)
  return (x ^ (x >>> 16)) >>> 0
}

const ORDER: Record<LimitBy, (a: Track, b: Track, seed: number) => number> = {
  random: (a, b, s) => mix(a.id, s) - mix(b.id, s),
  mostPlayed: (a, b) => b.plays - a.plays,
  leastPlayed: (a, b) => a.plays - b.plays,
  recentlyAdded: (a, b) => b.addedAt - a.addedAt,
  recentlyPlayed: (a, b) => (b.lastPlayed ?? 0) - (a.lastPlayed ?? 0),
  highestRated: (a, b) => b.rating - a.rating,
  title: (a, b) => a.title.localeCompare(b.title),
}

export function evaluate(tracks: Track[], rules: Rules, now = Date.now() / 1000): Track[] {
  const pass = (t: Track) =>
    !rules.rules.length || (rules.match === 'all' ? rules.rules.every((r) => test(t, r, now)) : rules.rules.some((r) => test(t, r, now)))
  const out = tracks.filter((t) => t.kind === 'audio' && pass(t))
  if (!rules.limit) return out
  const seed = rules.seed ?? 0
  return out.sort((a, b) => ORDER[rules.limit!.by](a, b, seed)).slice(0, rules.limit.count)
}

/** One-line summary for the playlist header, e.g. "Genre contains jazz · Loved is true". */
export function describe(rules: Rules): string {
  const ops = Object.values(OPS).flat()
  const part = (r: Rule) => {
    const op = ops.find(([o]) => o === r.op)?.[1] ?? r.op
    const kind = FIELDS[r.field].type
    const val =
      kind === 'bool' ? ''
      : r.op === 'between' ? `${r.value}–${r.value2}`
      : kind === 'date' ? `${r.value} ${r.unit ?? 'days'}`
      : `“${r.value}”`
    return `${FIELDS[r.field].label} ${op} ${val}`.trim()
  }
  const body = rules.rules.map(part).join(rules.match === 'all' ? ' · ' : ' or ')
  return [body || 'All songs', rules.limit && `limited to ${rules.limit.count}`].filter(Boolean).join(', ')
}
