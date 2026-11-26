import { describe, expect, it } from 'vitest'
import { endedEarly, streamedId } from './streamend'

describe('a streamed song that stops early', () => {
  it('is one that stopped well before its end, or by an error', () => {
    expect(endedEarly('eof', 95, 212)).toBe(true) // the stream dropped mid-song
    expect(endedEarly('eof', 211.4, 212)).toBe(false) // it played to the end
    expect(endedEarly('error', 0, 0)).toBe(true)
    expect(endedEarly('eof', 30, 0)).toBe(false) // no length known: can't tell
    expect(endedEarly('stop', 40, 212)).toBe(false) // you chose another song
    expect(endedEarly('quit', 40, 212)).toBe(false)
  })

  it('is known by the engine address mpv played', () => {
    expect(streamedId('http://127.0.0.1:52345/0f1e2d3c4b5a69788796a5b4c3d2e1f0/yt/dQw4w9WgXcQ')).toBe('dQw4w9WgXcQ')
    expect(streamedId('ytdl://https://www.youtube.com/watch?v=dQw4w9WgXcQ')).toBeNull()
    expect(streamedId('http://127.0.0.1:52345/0f1e/7')).toBeNull() // a listening session's song
    expect(streamedId('https://evil.example/tok/yt/dQw4w9WgXcQ')).toBeNull()
  })
})
