import { describe, expect, it } from 'vitest'
import { DEFAULT_SOURCES, firstLyrics, lookupSong, normalizeSources, serverSongId, youtubeSongId } from './lyricsources'
import { parseLrc } from './lyrics'

const ids = (s: { id: string; on: boolean }[]) => s.map((x) => `${x.id}${x.on ? '' : '-'}`).join(' ')

describe('lyrics sources', () => {
  it('starts with local files, the server, LRCLIB and YT Music; NetEase and Kugou are opt-in', () => {
    expect(ids(normalizeSources(undefined))).toBe('local subsonic lrclib ytmusic netease- kugou-')
    expect(normalizeSources(undefined)).not.toBe(DEFAULT_SOURCES)
  })

  it('keeps your order, drops unknown sources and adds new ones at the end', () => {
    const stored = [{ id: 'kugou', on: true }, { id: 'gone', on: true }, { id: 'local', on: false }, { id: 'kugou', on: false }]
    expect(ids(normalizeSources(stored))).toBe('kugou local- subsonic lrclib ytmusic netease-')
  })

  it('carries "Find lyrics online: off" over once', () => {
    expect(ids(normalizeSources(undefined, false))).toBe('local subsonic- lrclib- ytmusic- netease- kugou-')
    expect(ids(normalizeSources([{ id: 'lrclib', on: true }], false))).toBe('lrclib local subsonic ytmusic netease- kugou-')
  })

  it('takes the first source that has lyrics, or holds out for word timing', async () => {
    const found: Record<string, string> = {
      lrclib: '[00:01.00]line synced',
      kugou: '[00:01.00]<00:01.00>word <00:01.50>synced<00:02.00>',
    }
    const get = async (id: string) => (found[id] ? parseLrc(found[id]) : null)
    const order = normalizeSources([{ id: 'local', on: true }, { id: 'lrclib', on: true }, { id: 'kugou', on: true }])
    expect((await firstLyrics(order, get, false))?.source).toBe('lrclib')
    expect((await firstLyrics(order, get, true))?.source).toBe('kugou')
    // nothing word-synced anywhere: the first line-synced result still counts
    delete found.kugou
    expect((await firstLyrics(order, get, true))?.source).toBe('lrclib')
    expect(await firstLyrics(order.map((s) => ({ ...s, on: false })), get, false)).toBeNull()
  })

  it('finds a server song id in its stream address', () => {
    expect(serverSongId('https://m.example.com/rest/stream?u=me&t=x&s=y&id=tr-42&format=raw')).toBe('tr-42')
    expect(serverSongId('https://m.example.com/navidrome/rest/stream.view?id=7')).toBe('7')
    expect(serverSongId('D:\\Music\\a.flac')).toBeNull()
    expect(serverSongId('ytdl://https://www.youtube.com/watch?v=abc&id=9')).toBeNull()
  })
})

describe('lyrics for YouTube songs', () => {
  const ask = (title: string, artist = 'MindaRyn', album = 'YT Music') => lookupSong({ title, artist, album, duration: 271 })
  it('asks with the song title, not the video’s', () => {
    expect(ask('Like Flames').title).toBe('Like Flames')
    expect(ask('Like Flames (Official Video)').title).toBe('Like Flames')
    expect(ask('MindaRyn - Like Flames [Official Music Video]').title).toBe('Like Flames')
    expect(ask('MindaRyn『Like Flames』Music Video').title).toBe('Like Flames')
    expect(ask('【MV】Like Flames').title).toBe('Like Flames')
    expect(ask('Like Flames | TV Anime OP').title).toBe('Like Flames')
    // brackets that are part of the song stay
    expect(ask('Like Flames (feat. Someone)').title).toBe('Like Flames (feat. Someone)')
    expect(ask('(I Can’t Get No) Satisfaction', 'The Rolling Stones').title).toBe('(I Can’t Get No) Satisfaction')
    // a dash that isn't "artist - song" stays
    expect(ask('Love - Live', 'Other').title).toBe('Love - Live')
  })
  it('sends no placeholder album and no " - Topic" channel', () => {
    expect(ask('x').album).toBe('')
    expect(ask('x', 'a', 'YouTube').album).toBe('')
    expect(ask('x', 'a', 'Real Album').album).toBe('Real Album')
    expect(ask('x', 'MindaRyn - Topic').artist).toBe('MindaRyn')
  })
  it('knows a YouTube song by its video id', () => {
    expect(youtubeSongId('ytdl://https://www.youtube.com/watch?v=abcdefghijk')).toBe('abcdefghijk')
    expect(youtubeSongId('/music/a.flac')).toBeNull()
    expect(youtubeSongId('ytdl://https://www.youtube.com/watch?v=abc')).toBeNull()
  })
  it('offers YT Music lyrics, on, to people who had the old list', () => {
    expect(normalizeSources([{ id: 'lrclib', on: true }]).find((s) => s.id === 'ytmusic')?.on).toBe(true)
    expect(DEFAULT_SOURCES.some((s) => s.id === 'ytmusic')).toBe(true)
  })
})
