import { describe, expect, it } from 'vitest'
import { lyricLines, quranLines } from './notch'

describe('notch handoff', () => {
  it('shares synced lines with translations and word times', () => {
    const lyrics = {
      synced: true,
      lines: [
        { t: 1, end: 3, text: 'we fold', words: [{ t: 1, end: 2, text: 'we ' }, { t: 2, end: 3, text: 'fold' }] },
        { t: 3, end: 5, text: 'the light' },
      ],
    }
    expect(lyricLines(lyrics, ['kami melipat', null])).toEqual([
      { t: 1, end: 3, text: 'we fold', tr: 'kami melipat', words: [{ t: 1, end: 2, text: 'we ' }, { t: 2, end: 3, text: 'fold' }] },
      { t: 3, end: 5, text: 'the light' },
    ])
  })

  it('shares nothing for unsynced or missing lyrics', () => {
    expect(lyricLines({ synced: false, lines: [{ t: 0, end: 0, text: 'plain' }] }, null)).toEqual([])
    expect(lyricLines(null, null)).toEqual([])
  })

  it('turns a recitation into lines that run until the next unit', () => {
    const units = [{ text: 'istiadha' }, { text: 'ayah 1', tr: 'meaning 1' }, { text: 'ayah 2' }]
    expect(quranLines(units, [0, 4.5, 9], 20)).toEqual([
      { t: 0, end: 4.5, text: 'istiadha' },
      { t: 4.5, end: 9, text: 'ayah 1', tr: 'meaning 1' },
      { t: 9, end: 20, text: 'ayah 2' },
    ])
  })

  it('needs a start for every unit', () => {
    expect(quranLines([{ text: 'a' }, { text: 'b' }], null, 10)).toEqual([])
    expect(quranLines([{ text: 'a' }, { text: 'b' }], [0], 10)).toEqual([])
  })
})
