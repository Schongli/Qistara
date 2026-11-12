import { describe as group, expect, it } from 'vitest'
import { describe, evaluate, test, type Rules } from './smart'
import type { Track } from './library.svelte'

const base: Track = {
  id: 0, path: '', kind: 'audio', title: '', artist: '', albumArtist: '', album: '', genre: '', composer: '', year: null,
  track: null, disc: null, duration: 200, codec: 'FLAC', sampleRate: 44100, bitDepth: 16, channels: 2, bitrate: null,
  lossless: true, art: null, rating: 0, loved: false, plays: 0, skips: 0, lastPlayed: null, addedAt: 0,
}
const now = 1_000_000
const t = (o: Partial<Track>): Track => ({ ...base, ...o })
const lib = [
  t({ id: 1, title: 'Blue in Green', genre: 'Jazz', plays: 10, rating: 5, loved: true, addedAt: now - 3600, sampleRate: 96000, bitDepth: 24 }),
  t({ id: 2, title: 'So What', genre: 'jazz', plays: 3, addedAt: now - 86400 * 40, lastPlayed: now - 100 }),
  t({ id: 3, title: 'Teardrop', genre: 'Trip Hop', plays: 7, codec: 'MP3', lossless: false, addedAt: now - 86400 * 2 }),
  t({ id: 4, title: 'Clip', kind: 'video', genre: 'Jazz' }),
]

group('smart rules', () => {
  it('matches text case-insensitively', () => {
    expect(test(lib[1], { field: 'genre', op: 'is', value: 'JAZZ' }, now)).toBe(true)
    expect(test(lib[2], { field: 'title', op: 'startsWith', value: 'tear' }, now)).toBe(true)
    expect(test(lib[2], { field: 'genre', op: 'notContains', value: 'hop' }, now)).toBe(false)
  })
  it('compares numbers and handles missing values', () => {
    expect(test(lib[0], { field: 'sampleRate', op: 'gt', value: 48000 }, now)).toBe(true)
    expect(test(lib[0], { field: 'plays', op: 'between', value: 5, value2: 10 }, now)).toBe(true)
    expect(test(lib[1], { field: 'year', op: 'lt', value: 3000 }, now)).toBe(false) // no year: never matches
  })
  it('handles relative dates', () => {
    expect(test(lib[0], { field: 'addedAt', op: 'inLast', value: 1, unit: 'days' }, now)).toBe(true)
    expect(test(lib[1], { field: 'addedAt', op: 'inLast', value: 1, unit: 'months' }, now)).toBe(false)
    expect(test(lib[2], { field: 'lastPlayed', op: 'notInLast', value: 7 }, now)).toBe(true) // never played
  })
  it('evaluates all/any, skips videos, limits with an order', () => {
    const jazz: Rules = { match: 'all', rules: [{ field: 'genre', op: 'is', value: 'jazz' }] }
    expect(evaluate(lib, jazz, now).map((x) => x.id)).toEqual([1, 2])
    const any: Rules = { match: 'any', rules: [{ field: 'loved', op: 'isTrue', value: '' }, { field: 'lossless', op: 'isFalse', value: '' }] }
    expect(evaluate(lib, any, now).map((x) => x.id)).toEqual([1, 3])
    const top: Rules = { match: 'all', rules: [], limit: { count: 2, by: 'mostPlayed' } }
    expect(evaluate(lib, top, now).map((x) => x.id)).toEqual([1, 3])
  })
  it('keeps random limits stable for a seed', () => {
    const r: Rules = { match: 'all', rules: [], limit: { count: 2, by: 'random' }, seed: 42 }
    expect(evaluate(lib, r, now)).toEqual(evaluate([...lib].reverse(), r, now))
  })
  it('describes rules in plain words', () => {
    expect(describe({ match: 'all', rules: [{ field: 'genre', op: 'contains', value: 'jazz' }, { field: 'loved', op: 'isTrue', value: '' }], limit: { count: 25, by: 'random' } }))
      .toBe('Genre contains “jazz” · Loved is true, limited to 25')
  })
})
