import { describe, expect, it } from 'vitest'
import { DEFAULT_SOURCES, firstLyrics, normalizeSources, serverSongId } from './lyricsources'
import { parseLrc } from './lyrics'

const ids = (s: { id: string; on: boolean }[]) => s.map((x) => `${x.id}${x.on ? '' : '-'}`).join(' ')

describe('lyrics sources', () => {
  it('starts with local files, the server and LRCLIB; NetEase and Kugou are opt-in', () => {
    expect(ids(normalizeSources(undefined))).toBe('local subsonic lrclib netease- kugou-')
    expect(normalizeSources(undefined)).not.toBe(DEFAULT_SOURCES)
  })

  it('keeps your order, drops unknown sources and adds new ones at the end', () => {
    const stored = [{ id: 'kugou', on: true }, { id: 'gone', on: true }, { id: 'local', on: false }, { id: 'kugou', on: false }]
    expect(ids(normalizeSources(stored))).toBe('kugou local- subsonic lrclib netease-')
  })

  it('carries "Find lyrics online: off" over once', () => {
    expect(ids(normalizeSources(undefined, false))).toBe('local subsonic- lrclib- netease- kugou-')
    expect(ids(normalizeSources([{ id: 'lrclib', on: true }], false))).toBe('lrclib local subsonic netease- kugou-')
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
