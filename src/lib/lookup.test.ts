import { describe, expect, it } from 'vitest'
import { parseLookup } from './lookup'

describe('word lookup', () => {
  it('reads meanings by part of speech and the romanization', () => {
    // trimmed from translate_a/single?client=gtx&sl=ja&tl=en&dt=t&dt=rm&dt=bd&q=呼ぶ
    const reply = [
      [['call', '呼ぶ', null, null, 3], [null, null, null, 'Yobu']],
      [['verb', ['call', 'call out', 'invite'], [['call', ['呼び出す'], null, 0.11]], '呼ぶ', 2]],
      'ja',
    ]
    expect(parseLookup(reply)).toEqual({ translation: 'call', romaji: 'Yobu', source: 'ja', senses: [{ pos: 'verb', terms: ['call', 'call out', 'invite'] }] })
  })

  it('copes with a word that has no dictionary entry', () => {
    const reply = [[['call', '呼ん', null, null, 3], [null, null, null, 'Yon']], null, 'ja']
    expect(parseLookup(reply)).toEqual({ translation: 'call', romaji: 'Yon', source: 'ja', senses: [] })
    expect(() => parseLookup('nope')).toThrow()
  })
})
