import { describe, expect, it } from 'vitest'
import { byName, quality, time, totalTime } from './format'
import { autoplayPicks, indexAfterMove, insertAfter, move, nextIndex, shuffleRest } from './queue'
import { project, rubberband, velocity } from './gesture'
import { matchTracks } from './musicbrainz'
import { loadfileArgs, mpvRelease } from './loadfile'

describe('gesture', () => {
  it('projects a flick like scroll deceleration', () => {
    expect(project(0)).toBe(0)
    expect(project(1000)).toBeCloseTo(499, 0)
    expect(project(-500)).toBeCloseTo(-249.5, 0)
  })
  it('rubber-bands progressively and keeps direction', () => {
    expect(rubberband(0, 800)).toBe(0)
    expect(rubberband(100, 800)).toBeLessThan(100)
    expect(rubberband(400, 800) / 400).toBeLessThan(rubberband(100, 800) / 100)
    expect(rubberband(-100, 800)).toBeCloseTo(-rubberband(100, 800))
  })
  it('measures velocity over the last 100 ms', () => {
    expect(velocity([{ t: 0, y: 0 }, { t: 400, y: 0 }, { t: 450, y: 50 }, { t: 500, y: 100 }])).toBe(1000)
    expect(velocity([{ t: 0, y: 5 }])).toBe(0)
  })
})

describe('quality', () => {
  it('uses Apple tiers', () => {
    expect(quality({ codec: 'FLAC', lossless: true, sampleRate: 44100, bitDepth: 16 })).toEqual({ badge: 'Lossless', detail: '16-bit/44.1 kHz FLAC' })
    expect(quality({ codec: 'FLAC', lossless: true, sampleRate: 48000, bitDepth: 24 }).badge).toBe('Lossless')
    expect(quality({ codec: 'ALAC', lossless: true, sampleRate: 96000, bitDepth: 24 })).toEqual({ badge: 'Hi-Res Lossless', detail: '24-bit/96 kHz ALAC' })
    expect(quality({ codec: 'MP3', lossless: false, bitrate: 320 })).toEqual({ badge: null, detail: '320 kbps MP3' })
    expect(quality({ codec: 'DSF', lossless: true }).badge).toBe('DSD')
  })
})

describe('format', () => {
  it('formats times', () => {
    expect([time(0), time(65.9), time(3725), time(NaN)]).toEqual(['0:00', '1:05', '1:02:05', '0:00'])
    expect([totalTime(2520), totalTime(4500)]).toEqual(['42 min', '1 hr 15 min'])
  })
  it('sorts names ignoring leading articles', () => {
    expect(['The Beatles', 'Coldplay', 'Adele', 'Björk'].sort(byName)).toEqual(['Adele', 'The Beatles', 'Björk', 'Coldplay'])
  })
})

describe('queue', () => {
  it('inserts Play Next items right after the current one', () => {
    expect(insertAfter([1, 2, 3], 0, [9, 8])).toEqual([1, 9, 8, 2, 3])
  })
  it('shuffles only upcoming items and keeps all of them', () => {
    const out = shuffleRest([1, 2, 3, 4, 5], 1, () => 0)
    expect(out.slice(0, 2)).toEqual([1, 2])
    expect(out.slice().sort()).toEqual([1, 2, 3, 4, 5])
  })
  it('advances, repeats and ends', () => {
    expect(nextIndex(3, 0, 'off', false)).toBe(1)
    expect(nextIndex(3, 2, 'off', false)).toBe(-1)
    expect(nextIndex(3, 2, 'all', false)).toBe(0)
    expect(nextIndex(3, 1, 'one', false)).toBe(1)
    expect(nextIndex(3, 1, 'one', true)).toBe(2)
    expect(nextIndex(0, -1, 'all', true)).toBe(-1)
  })
  it('reorders and keeps the current index on the same item', () => {
    const items = ['a', 'b', 'c', 'd'], cur = 1 // 'b'
    for (const [from, to] of [[0, 3], [3, 0], [1, 3], [2, 3], [0, 0]]) {
      expect(move(items, from, to)[indexAfterMove(cur, from, to)]).toBe('b')
    }
  })
})

describe('musicbrainz matching', () => {
  const list = [
    { disc: 1, position: 1, title: 'A', artist: '', length: 0 },
    { disc: 1, position: 2, title: 'B', artist: '', length: 0 },
    { disc: 2, position: 1, title: 'C', artist: '', length: 0 },
  ]
  it('matches by disc and track number, then by order', () => {
    expect(matchTracks([{ disc: 2, track: 1 }, { disc: null, track: 2 }, { disc: null, track: null }], list).map((t) => t?.title)).toEqual(['C', 'B', 'C'])
    expect(matchTracks([{ disc: 1, track: 9 }, { disc: 1, track: 9 }, { disc: 1, track: 9 }, { disc: 1, track: 9 }], list).map((t) => t?.title ?? null)).toEqual(['A', 'B', 'C', null])
  })
})

describe('autoplay', () => {
  const t = (id: number, o: Partial<{ kind: string; artist: string; albumArtist: string; genre: string; album: string; loved: boolean; rating: number; lastPlayed: number | null }> = {}) => ({
    id, kind: 'audio', artist: 'A', albumArtist: '', genre: 'Jazz', album: 'X', loved: false, rating: 0, lastPlayed: null, ...o,
  })
  const seed = t(1, { album: 'Seed' })
  const now = 1_000_000
  it('prefers the same artist and genre, skips queued songs, videos and ad-hoc files', () => {
    const all = [seed, t(2, { artist: 'B', genre: 'Rock' }), t(3), t(4, { artist: 'B' }), t(5, { kind: 'video' }), t(-9)]
    expect(autoplayPicks(all, seed, new Set([1]), 10, () => 0, now).map((x) => x.id)).toEqual([3, 4, 2])
  })
  it('pushes down songs played in the last day and caps the count', () => {
    const all = [t(2, { lastPlayed: now - 60 }), t(3, { artist: 'Z', genre: 'Z' }), t(4)]
    expect(autoplayPicks(all, seed, new Set(), 2, () => 0, now).map((x) => x.id)).toEqual([4, 3])
  })
})

describe('loadfile', () => {
  it('reads the release from mpv-version', () => {
    expect(mpvRelease('mpv 0.35.1')).toEqual([0, 35])
    expect(mpvRelease('mpv 0.37.0')).toEqual([0, 37])
    expect(mpvRelease('mpv v0.37.0-412-gabcdef0')).toEqual([0, 37])
    expect(mpvRelease('mpv v0.38.0')).toEqual([0, 38])
    expect(mpvRelease('mpv 0.39.0-dirty')).toEqual([0, 39])
    expect(mpvRelease('mpv 1.0.0')).toEqual([1, 0])
    expect(mpvRelease('mpv git-1a2b3c4')).toBeNull()
    expect(mpvRelease(null)).toBeNull()
  })
  it('puts the options third before mpv 0.38 and after an index from 0.38', () => {
    expect(loadfileArgs('/a.flac', 'replace', '', 'mpv 0.37.0')).toEqual(['/a.flac', 'replace'])
    for (const v of ['mpv 0.35.1', 'mpv 0.37.0', 'mpv v0.37.0-412-gabcdef0']) {
      expect(loadfileArgs('/a.flac', 'replace', 'start=12', v)).toEqual(['/a.flac', 'replace', 'start=12'])
    }
    for (const v of ['mpv v0.38.0', 'mpv 0.39.0', 'mpv 1.0.0', 'mpv git-1a2b3c4', null]) {
      expect(loadfileArgs('/a.flac', 'append', 'start=12', v)).toEqual(['/a.flac', 'append', -1, 'start=12'])
    }
  })
})
