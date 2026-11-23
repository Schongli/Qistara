import { describe, expect, it } from 'vitest'
import { parseVideoId } from './videolink'

describe('music video links', () => {
  it('finds the video in any YouTube link', () => {
    for (const s of [
      'https://www.youtube.com/watch?v=dQw4w9WgXcQ', 'https://youtu.be/dQw4w9WgXcQ?si=abc', 'https://music.youtube.com/watch?v=dQw4w9WgXcQ&list=RD',
      'https://www.youtube.com/watch?feature=share&v=dQw4w9WgXcQ', 'https://www.youtube.com/shorts/dQw4w9WgXcQ', ' dQw4w9WgXcQ ',
      'https://www.youtube.com/embed/dQw4w9WgXcQ',
    ]) expect(parseVideoId(s), s).toBe('dQw4w9WgXcQ')
  })
  it('says no to anything else', () => {
    for (const s of ['', 'not a link', 'https://youtu.be/short', 'https://www.youtube.com/watch?v=dQw4w9WgXcQx1', 'https://example.com/']) {
      expect(parseVideoId(s), s).toBeNull()
    }
  })
})
