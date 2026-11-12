import { describe, expect, it } from 'vitest'
import { findDuplicates, songKey } from './dupes'

const t = (id: number, o: Partial<{ title: string; artist: string; albumArtist: string; duration: number; lossless: boolean; sampleRate: number | null; bitDepth: number | null; bitrate: number | null; kind: string }> = {}) => ({
  id, kind: 'audio', title: 'Snow', artist: 'Aurora', albumArtist: '', duration: 200, lossless: false, sampleRate: 44100, bitDepth: null, bitrate: 320, ...o,
})

describe('duplicates', () => {
  it('normalises remaster and version tags', () => {
    expect(songKey('Snow (2011 Remaster)')).toBe('snow')
    expect(songKey('Snow - Remastered 2009')).toBe('snow')
    expect(songKey('Snow [Single Version]')).toBe('snow')
    expect(songKey("Don't Stop")).toBe('don t stop')
  })
  it('groups the same song, best quality first, and keeps different lengths apart', () => {
    const lib = [
      t(1),
      t(2, { lossless: true, sampleRate: 96000, bitDepth: 24, bitrate: null, title: 'Snow (Remaster)' }),
      t(3, { lossless: true, sampleRate: 44100, bitDepth: 16 }),
      t(4, { duration: 420 }), // the extended mix is a different track
      t(5, { title: 'Rain' }),
      t(6, { kind: 'video' }),
    ]
    expect(findDuplicates(lib).map((g) => g.map((x) => x.id))).toEqual([[2, 3, 1]])
  })
})
