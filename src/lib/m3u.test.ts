import { describe, expect, it } from 'vitest'
import { buildM3u, normalize, parseM3u } from './m3u'

describe('M3U', () => {
  it('reads extended M3U with absolute, relative, UNC, file:// and URL entries', () => {
    const text = '\uFEFF#EXTM3U\r\n#EXTINF:215,Aurora Lane - Snow\r\nD:\\Music\\a.flac\r\n# a comment\r\n../Other/b.mp3\r\n\\\\nas\\share\\c.flac\r\nfile:///D:/Music/My%20Song.flac\r\nhttps://ice1.somafm.com/groovesalad-128-mp3\r\n'
    expect(parseM3u(text, 'D:\\Music\\Lists')).toEqual([
      { path: 'D:\\Music\\a.flac', title: 'Aurora Lane - Snow', duration: 215 },
      { path: 'D:\\Music\\Other\\b.mp3' },
      { path: '\\\\nas\\share\\c.flac' },
      { path: 'D:\\Music\\My Song.flac' },
      { path: 'https://ice1.somafm.com/groovesalad-128-mp3' },
    ])
  })
  it('normalises slashes and dot segments', () => {
    expect(normalize('D:/Music/./A/../b.flac')).toBe('D:\\Music\\b.flac')
  })
  it('writes a playlist other players can read back', () => {
    const out = buildM3u([{ path: 'D:\\Music\\a.flac', title: 'Snow', artist: 'Aurora Lane', duration: 214.6 }, { path: 'D:\\x.mp3', title: 'X', artist: '', duration: 0 }])
    expect(out).toBe('#EXTM3U\r\n#EXTINF:215,Aurora Lane - Snow\r\nD:\\Music\\a.flac\r\n#EXTINF:-1,X\r\nD:\\x.mp3\r\n')
    expect(parseM3u(out, 'C:\\').map((e) => e.path)).toEqual(['D:\\Music\\a.flac', 'D:\\x.mp3'])
  })
})
