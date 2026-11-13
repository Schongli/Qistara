// Matches songs from Spotify, Apple Music or Tidal to files in your library, so an imported playlist plays your own lossless copies.
import { rank, songKey } from './dupes'

export interface RemoteTrack { title: string; artists: string[]; album: string; duration: number; isrc: string | null }

type Local = {
  id: number; kind: string; title: string; artist: string; albumArtist: string; album: string; duration: number
  lossless: boolean; sampleRate: number | null; bitDepth: number | null; bitrate: number | null
}

/** songKey, minus "(feat. X)", "[with X]" and " - feat. X" credits that Spotify puts in titles. */
export const titleKey = (s: string) =>
  songKey(s.replace(/[([]\s*(?:feat|ft|featuring|with)\b[^)\]]*[)\]]/gi, '').replace(/\s+-\s+(?:feat|ft)\b.*$/i, ''))

const hasArtist = (artists: string[], t: Local) => {
  const mine = ` ${songKey(`${t.artist} ${t.albumArtist}`)} `
  return artists.some((a) => {
    const k = songKey(a)
    return k !== '' && mine.includes(` ${k} `) // whole words, so "Ra" never matches "Sara"
  })
}

function pick<T extends Local>(s: RemoteTrack, byTitle: Map<string, T[]>): T | null {
  const found = (byTitle.get(titleKey(s.title)) ?? []).filter((t) => hasArtist(s.artists, t))
  const within = (secs: number) => found.filter((t) => !t.duration || !s.duration || Math.abs(t.duration - s.duration) <= secs)
  // a different master can run a few seconds longer; allow 10 s only when there's no other choice
  const pool = within(3).length ? within(3) : found.length === 1 ? within(10) : []
  const album = songKey(s.album)
  const same = (t: T) => Number(album !== '' && songKey(t.album) === album)
  return pool.sort((a, b) => same(b) - same(a) || rank(b) - rank(a))[0] ?? null
}

/** Library ids for the Spotify songs (in order) and the songs that aren't in the library. */
export function matchTracks<T extends Local>(songs: RemoteTrack[], library: T[]) {
  const byTitle = new Map<string, T[]>()
  for (const t of library) {
    if (t.kind !== 'audio' || !t.title) continue
    const k = titleKey(t.title)
    byTitle.set(k, [...(byTitle.get(k) ?? []), t])
  }
  const ids: number[] = []
  const missing: RemoteTrack[] = []
  for (const s of songs) {
    const t = pick(s, byTitle)
    if (t) ids.push(t.id)
    else missing.push(s)
  }
  return { ids, missing }
}
