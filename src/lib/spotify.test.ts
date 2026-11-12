import { describe, expect, it } from 'vitest'
import { matchTracks, titleKey, type SpotifyTrack } from './spotify'

let id = 0
const local = (title: string, artist: string, o: Partial<{ album: string; duration: number; lossless: boolean; sampleRate: number; kind: string; albumArtist: string }> = {}) => ({
  id: ++id, kind: 'audio', title, artist, albumArtist: '', album: '', duration: 200, lossless: true, sampleRate: 44100, bitDepth: 16, bitrate: null, ...o,
})
const sp = (title: string, artists: string[], o: Partial<SpotifyTrack> = {}): SpotifyTrack => ({ title, artists, album: '', duration: 200, isrc: null, ...o })

describe('spotify matching', () => {
  it('ignores remaster and featuring notes in titles', () => {
    expect(titleKey('Heroes - 2017 Remaster')).toBe('heroes')
    expect(titleKey('Stay (feat. Justin Bieber)')).toBe('stay')
    expect(titleKey('Stay [with Justin Bieber]')).toBe('stay')
    expect(titleKey('Mr. Brightside')).toBe('mr brightside')
  })

  it('matches by title, artist and length', () => {
    const lib = [local('Heroes', 'David Bowie'), local('Heroes', 'Someone Else')]
    const r = matchTracks([sp('Heroes - 2017 Remaster', ['David Bowie'])], lib)
    expect(r.ids).toEqual([lib[0].id])
    expect(r.missing).toEqual([])
  })

  it('finds any credited artist inside the local artist field', () => {
    const lib = [local('Stay', 'The Kid LAROI & Justin Bieber')]
    expect(matchTracks([sp('Stay (with Justin Bieber)', ['The Kid LAROI', 'Justin Bieber'])], lib).ids).toEqual([lib[0].id])
    // whole words only: "Ra" is not inside "Sara Bareilles"
    expect(matchTracks([sp('Stay', ['Ra'])], [local('Stay', 'Sara Bareilles')]).ids).toEqual([])
  })

  it('needs the length to agree', () => {
    const lib = [local('Song', 'A', { duration: 200 }), local('Song', 'A', { duration: 400 })]
    expect(matchTracks([sp('Song', ['A'], { duration: 401 })], lib).ids).toEqual([lib[1].id])
    // a lone candidate may be up to 10 s off (different master), never more
    const one = [local('Solo', 'B', { duration: 200 })]
    expect(matchTracks([sp('Solo', ['B'], { duration: 208 })], one).ids).toEqual([one[0].id])
    expect(matchTracks([sp('Solo', ['B'], { duration: 230 })], one).missing).toHaveLength(1)
  })

  it('prefers the same album, then the best quality', () => {
    const mp3 = local('Song', 'A', { album: 'Hits', lossless: false })
    const flac = local('Song', 'A', { album: 'Hits', lossless: true, sampleRate: 96000 })
    const other = local('Song', 'A', { album: 'Live', lossless: true, sampleRate: 192000 })
    expect(matchTracks([sp('Song', ['A'], { album: 'Hits' })], [mp3, other, flac]).ids).toEqual([flac.id])
  })

  it('skips videos and reports what is missing', () => {
    const lib = [local('Clip', 'A', { kind: 'video' })]
    const r = matchTracks([sp('Clip', ['A'])], lib)
    expect(r.ids).toEqual([])
    expect(r.missing.map((t) => t.title)).toEqual(['Clip'])
  })
})
