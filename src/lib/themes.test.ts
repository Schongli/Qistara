import { describe, expect, it } from 'vitest'
import { LIGHT, OBSIDIAN, PRESETS, presetVars } from './themes'
import { contrast, readableAccent } from './color'

describe('theme presets', () => {
  it('keep text and secondary text readable on both surfaces', () => {
    for (const p of [LIGHT, OBSIDIAN, ...PRESETS]) {
      for (const page of [p.bg, p.bg2]) {
        expect(contrast(p.text, page), `${p.id} text`).toBeGreaterThanOrEqual(7)
        expect(contrast(p.text2, page), `${p.id} text-2`).toBeGreaterThanOrEqual(4.5)
      }
      // the accent is made readable against the page the same way a picked one is
      expect(contrast(readableAccent(p.accent, p.bg)!, p.bg)).toBeGreaterThanOrEqual(4.5)
    }
  })

  it('set every surface token', () => {
    const v = presetVars(PRESETS[1])
    expect(v['--bg']).toBe('#191724')
    expect(v['--bar']).toBe('rgba(25, 23, 36, 0.72)')
    expect(Object.keys(v)).toEqual(['--bg', '--bg-2', '--text', '--text-2', '--text-3', '--fill', '--fill-2', '--sep', '--bar', '--sidebar'])
  })
})
