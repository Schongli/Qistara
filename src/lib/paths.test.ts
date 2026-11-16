import { describe, expect, it } from 'vitest'
import { isUnder, joinPath, pathKey } from './paths'

describe('paths', () => {
  it('joins with the slash the folder uses', () => {
    expect([joinPath('C:\\data\\art\\', 'a.jpg'), joinPath('/home/me/.local/art/', 'a.jpg')]).toEqual(['C:\\data\\art\\a.jpg', '/home/me/.local/art/a.jpg'])
  })
  it('ignores case only for Windows paths', () => {
    expect(pathKey('D:\\Music\\A.flac')).toBe('d:\\music\\a.flac')
    expect(pathKey('/Music/A.flac')).toBe('/Music/A.flac')
  })
  it('tells whether a path is in a folder', () => {
    expect(isUnder('D:\\Music\\Downloads', 'd:\\music\\')).toBe(true)
    expect(isUnder('D:\\Musical', 'D:\\Music')).toBe(false)
    expect(isUnder('/home/me/Music/x', '/home/me/Music')).toBe(true)
    expect(isUnder('/home/me/music/x', '/home/me/Music')).toBe(false)
    expect(isUnder('/home/me/Music', '/home/me/Music/')).toBe(true)
  })
})
