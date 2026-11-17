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

describe('M3U hardening', () => {
  it('survives a stray % and reads file URLs with a host', () => {
    const text = 'file:///D:/Music/100%.flac\nfile://localhost/D:/Music/a.flac\nfile://nas/share/b.flac\n'
    expect(parseM3u(text, 'D:\\Lists').map((e) => e.path)).toEqual(['D:\\Music\\100%.flac', 'D:\\Music\\a.flac', '\\\\nas\\share\\b.flac'])
  })
  it('resolves drive-root paths and never climbs above the drive or share', () => {
    expect(parseM3u('\\Music\\a.flac\n..\\..\\..\\..\\b.flac', 'E:\\Lists\\Mine').map((e) => e.path)).toEqual(['E:\\Music\\a.flac', 'E:\\b.flac'])
    expect(normalize('\\\\nas\\music\\..\\..\\x.flac')).toBe('\\\\nas\\music\\x.flac')
  })
  it('reads POSIX playlists on Linux and macOS', () => {
    const text = '#EXTM3U\n/home/me/Music/a.flac\n../b.flac\nfile:///Users/me/My%20Song.flac\nfile://localhost/srv/c.flac\n'
    expect(parseM3u(text, '/home/me/Music/Lists').map((e) => e.path)).toEqual(['/home/me/Music/a.flac', '/home/me/Music/b.flac', '/Users/me/My Song.flac', '/srv/c.flac'])
    expect(normalize('/a/../../b', true)).toBe('/b')
  })
  it('splits old Mac line breaks and keeps tags with line breaks on one line', () => {
    expect(parseM3u('#EXTM3U\rD:\\a.flac\rD:\\b.flac', 'C:\\').length).toBe(2)
    const out = buildM3u([{ path: 'D:\\a.flac', title: 'Two\nlines', artist: 'A\r\nB', duration: 1 }])
    expect(out.split('\r\n')).toEqual(['#EXTM3U', '#EXTINF:1,A B - Two lines', 'D:\\a.flac', ''])
  })
})
