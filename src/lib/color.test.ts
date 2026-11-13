import { describe, expect, it } from 'vitest'
import { contrast, onAccent, readableAccent } from './color'

describe('accent colours', () => {
  it('measures contrast like WCAG', () => {
    expect(contrast('#000000', '#ffffff')).toBeCloseTo(21, 0)
    expect(contrast('#0a7f6f', '#ffffff')).toBeGreaterThan(4.5)
  })

  it('keeps a readable accent as it is', () => {
    expect(readableAccent('#0a7f6f', '#ffffff')).toBe('#0a7f6f')
    expect(readableAccent('#2cc4a9', '#1e1e1f')).toBe('#2cc4a9')
  })

  it('darkens a pale accent on a light page and lightens a dark one on a dark page', () => {
    for (const [hex, page] of [['#ffd60a', '#ffffff'], ['#1a237e', '#1e1e1f'], ['#ff2d55', '#ffffff']]) {
      const out = readableAccent(hex, page)!
      expect(contrast(out, page)).toBeGreaterThanOrEqual(4.5)
    }
  })

  it('rejects anything that is not a hex colour', () => {
    expect(readableAccent('red', '#ffffff')).toBeNull()
    expect(readableAccent('#12345', '#ffffff')).toBeNull()
  })

  it('picks the button text that reads best', () => {
    expect(onAccent('#0a7f6f')).toBe('#ffffff')
    expect(onAccent('#2cc4a9')).toBe('#04231e')
  })
})
