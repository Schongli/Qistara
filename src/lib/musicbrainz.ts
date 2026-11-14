// MusicBrainz release lookup (1 request per second, per their rules) and Cover Art Archive URLs.
import { fetch } from '@tauri-apps/plugin-http'

import { UA } from './ua'
let last = 0

async function get<T>(path: string): Promise<T> {
  const wait = last + 1100 - Date.now()
  if (wait > 0) await new Promise((r) => setTimeout(r, wait))
  last = Date.now()
  const r = await fetch(`https://musicbrainz.org/ws/2/${path}`, { headers: { 'User-Agent': UA, Accept: 'application/json' } })
  const j = (await r.json().catch(() => ({}))) as T & { error?: string }
  // MusicBrainz reports overload and bad queries as {"error": "..."}
  if (!r.ok || j.error) throw new Error(j.error ?? `MusicBrainz answered ${r.status}`)
  return j
}

export interface Release {
  id: string
  title: string
  artist: string
  date: string
  country: string
  tracks: number
  format: string
}

export interface ReleaseTrack { disc: number; position: number; title: string; artist: string; length: number }
export interface ReleaseDetail extends Release { genre: string; year: string; tracklist: ReleaseTrack[] }

type Credit = { name: string; joinphrase?: string }[]
const credit = (c?: Credit) => (c ?? []).map((a) => a.name + (a.joinphrase ?? '')).join('')
// Lucene query syntax: quote values and escape what would break the quote
const q = (s: string) => `"${s.replace(/[\\"]/g, '\\$&')}"`

export async function searchReleases(album: string, artist: string): Promise<Release[]> {
  const query = [album && `release:${q(album)}`, artist && `artist:${q(artist)}`].filter(Boolean).join(' AND ')
  const j = await get<{ releases: { id: string; title: string; date?: string; country?: string; 'track-count': number; 'artist-credit'?: Credit; media?: { format?: string }[] }[] }>(
    `release/?query=${encodeURIComponent(query)}&fmt=json&limit=10`,
  )
  return j.releases.map((r) => ({
    id: r.id, title: r.title, artist: credit(r['artist-credit']), date: r.date ?? '', country: r.country ?? '',
    tracks: r['track-count'], format: [...new Set((r.media ?? []).map((m) => m.format).filter(Boolean))].join(' + '),
  }))
}

export async function release(id: string): Promise<ReleaseDetail> {
  const r = await get<{
    id: string; title: string; date?: string; country?: string; 'artist-credit'?: Credit; genres?: { name: string; count: number }[]
    media: { position: number; format?: string; tracks: { position: number; title: string; length?: number; 'artist-credit'?: Credit }[] }[]
  }>(`release/${id}?inc=recordings+artist-credits+genres&fmt=json`)
  const tracklist = r.media.flatMap((m) =>
    m.tracks.map((t) => ({ disc: m.position, position: t.position, title: t.title, artist: credit(t['artist-credit']), length: (t.length ?? 0) / 1000 })),
  )
  const genre = [...(r.genres ?? [])].sort((a, b) => b.count - a.count)[0]?.name ?? ''
  return {
    id: r.id, title: r.title, artist: credit(r['artist-credit']), date: r.date ?? '', country: r.country ?? '',
    tracks: tracklist.length, format: '', genre: genre.replace(/\b\w/g, (c) => c.toUpperCase()), year: (r.date ?? '').slice(0, 4), tracklist,
  }
}

export const coverUrl = (releaseId: string) => `https://coverartarchive.org/release/${releaseId}/front-500`

/** Pairs library files with release tracks: by disc + track number first, else by position. */
export function matchTracks<T extends { disc: number | null; track: number | null }>(files: T[], list: ReleaseTrack[]): (ReleaseTrack | null)[] {
  return files.map((f, i) => list.find((t) => t.disc === (f.disc ?? 1) && t.position === f.track) ?? list[i] ?? null)
}
