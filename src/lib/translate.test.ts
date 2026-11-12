import { describe, expect, it } from 'vitest'
import { align, batches, hash, mapBack, parseGtx, sameLanguage, targetFor, uniqueLines } from './translate'

// trimmed from a real reply of translate.googleapis.com/translate_a/single?client=gtx&dt=t
const reply = [
  [
    ['Halo kegelapan, teman lamaku\n', 'Hello darkness, my old friend\n', null, null, 3],
    ['Saya datang untuk berbicara lagi', 'I have come to talk again', null, null, 3],
  ],
  null,
  'en',
]

describe('translate', () => {
  it('reads the translated text and the detected language', () => {
    expect(parseGtx(reply)).toEqual({ text: 'Halo kegelapan, teman lamaku\nSaya datang untuk berbicara lagi', source: 'en' })
    expect(() => parseGtx({ error: 'nope' })).toThrow()
  })

  it('pairs lines only when the counts agree', () => {
    expect(align(['a', 'b'], 'A\nB')).toEqual(['A', 'B'])
    expect(align(['a', 'b'], 'A\nB\n')).toEqual(['A', 'B'])
    expect(align(['a', 'b'], 'A B')).toBeNull()
  })

  it('sends each distinct line once', () => {
    expect(uniqueLines(['La la', '', 'Chorus', ' La la ', 'Chorus'])).toEqual(['La la', 'Chorus'])
  })

  it('keeps batches under the size limit without splitting a line', () => {
    const lines = ['a'.repeat(6), 'b'.repeat(6), 'c'.repeat(6), 'd'.repeat(30)]
    expect(batches(lines, 14)).toEqual([[lines[0], lines[1]], [lines[2]], [lines[3]]])
    expect(batches([], 14)).toEqual([])
  })

  it('maps translations back to every line and hides unchanged ones', () => {
    const dict = { 'La la': 'La la', Chorus: 'Paduan suara', 'Hello': 'Halo' }
    expect(mapBack(['Hello', '', 'Chorus', ' La la', 'Chorus', 'Unknown'], dict)).toEqual(['Halo', null, 'Paduan suara', null, 'Paduan suara', null])
  })

  it('picks a target from the UI language', () => {
    expect(targetFor('id-ID')).toBe('id')
    expect(targetFor('en')).toBe('en')
    expect(targetFor('zh-TW')).toBe('zh-TW')
    expect(targetFor('zh-Hant-HK')).toBe('zh-TW')
    expect(targetFor('zh')).toBe('zh-CN')
  })

  it('treats regional variants as the same language, except Chinese scripts', () => {
    expect(sameLanguage('en', 'en')).toBe(true)
    expect(sameLanguage('pt', 'pt-PT')).toBe(true)
    expect(sameLanguage('zh-CN', 'zh-TW')).toBe(false)
    expect(sameLanguage('ja', 'id')).toBe(false)
  })

  it('hashes stably', () => {
    expect(hash('abc')).toBe(hash('abc'))
    expect(hash('abc')).not.toBe(hash('abd'))
    expect(hash('abc')).toMatch(/^[0-9a-f]{8}$/)
  })
})
