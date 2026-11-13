// Network streams and saved internet radio stations.
import { fetch } from '@tauri-apps/plugin-http'
import { kvGet, kvSet } from './persist'
import { queue } from './queue.svelte'
import type { Track } from './library.svelte'

export interface Station { name: string; url: string }

export const radio = $state({ stations: [] as Station[] })
kvGet<Station[]>('radio').then((s) => s && (radio.stations = s))
const save = () => kvSet('radio', $state.snapshot(radio.stations))

/** True for http(s)/rtsp/rtmp/… addresses that mpv can stream. */
export const isUrl = (s: string) => /^(https?|rtsps?|rtmps?|mms|srt|udp|rtp):\/\/\S+$/i.test(s.trim())

let ids = -1_000_000
/** A track that isn't in the library: a stream, a YouTube video, a song on your server. The negative id
 *  keeps it out of play counts, playlists and the restored queue. */
export function remoteTrack(t: Partial<Track> & { path: string; title: string }): Track {
  return {
    id: ids--, kind: 'audio', artist: '', albumArtist: '', album: '', genre: '', composer: '', year: null, track: null,
    disc: null, duration: 0, codec: 'Stream', sampleRate: null, bitDepth: null, channels: null, bitrate: null, lossless: false,
    art: null, rating: 0, loved: false, plays: 0, skips: 0, lastPlayed: null, addedAt: 0, ...t,
  }
}

export const streamTrack = (url: string, name = '') =>
  remoteTrack({ path: url.trim(), title: name || url.replace(/^\w+:\/\//, '').split(/[?#]/)[0], artist: name ? 'Internet Radio' : '' })

export const playUrl = (url: string, name = '') => queue.play([streamTrack(url, name)])

export function addStation(name: string, url: string) {
  radio.stations = [...radio.stations.filter((s) => s.url !== url), { name: name.trim() || url, url: url.trim() }]
  save()
}

export function removeStation(url: string) {
  radio.stations = radio.stations.filter((s) => s.url !== url)
  save()
}

export interface FoundStation { id: string; name: string; url: string; country: string; codec: string; bitrate: number; tags: string }

/** Radio Browser (radio-browser.info): the free, open directory of internet radio. */
export async function searchStations(q: string): Promise<FoundStation[]> {
  const p = new URLSearchParams({ name: q.trim(), limit: '40', hidebroken: 'true', order: 'clickcount', reverse: 'true' })
  const r = await fetch(`https://all.api.radio-browser.info/json/stations/search?${p}`, { headers: { 'User-Agent': 'ConfluencePlayer/0.2.0' } })
  if (!r.ok) throw new Error(`Radio Browser answered ${r.status}`)
  const list = (await r.json()) as { stationuuid: string; name: string; url_resolved: string; url: string; country: string; codec: string; bitrate: number; tags: string }[]
  return list
    .map((s) => ({ id: s.stationuuid, name: s.name.trim(), url: s.url_resolved || s.url, country: s.country, codec: s.codec, bitrate: s.bitrate, tags: s.tags }))
    .filter((s) => isUrl(s.url))
}
