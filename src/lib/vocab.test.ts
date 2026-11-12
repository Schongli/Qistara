import { describe, expect, it } from 'vitest'
import { DAY, dueItems, review, vocabId, type VocabItem } from './vocab'

const item = (word: string, box = 0, due = 0): VocabItem => ({
  id: vocabId('ja', word), word, reading: '', romaji: '', pos: 'verb', meanings: [], lang: 'ja',
  line: '', translation: '', path: '', title: '', artist: '', t: 0, added: 0, box, due,
})

describe('vocab review', () => {
  it('moves a known word up a box and further out', () => {
    const now = 1_000_000
    expect(review(item('呼ぶ'), true, now)).toMatchObject({ box: 1, due: now + DAY })
    expect(review(item('呼ぶ', 3), true, now)).toMatchObject({ box: 4, due: now + 14 * DAY })
    expect(review(item('呼ぶ', 5), true, now).box).toBe(5) // stays in the last box
  })

  it('sends a missed word back to the start, due now', () => {
    expect(review(item('呼ぶ', 4), false, 500)).toMatchObject({ box: 0, due: 500 })
  })

  it('lists due words, oldest first', () => {
    const items = [item('a', 1, 300), item('b', 0, 100), item('c', 2, 9999)]
    expect(dueItems(items, 1000).map((i) => i.word)).toEqual(['b', 'a'])
  })

  it('keys words by language and dictionary form', () => {
    expect(vocabId('ja', '呼ぶ')).toBe('ja:呼ぶ')
  })
})
