import { describe, expect, it } from 'vitest'
import { adaptivePalette, FAMILIES, LIGHT, OBSIDIAN, PRESETS, presetVars, RENAMED, type Palette } from './themes'
import { contrast, readableAccent } from './color'

function readable(p: Palette) {
  for (const page of [p.bg, p.bg2]) {
    expect(contrast(p.text, page), `${p.id} text`).toBeGreaterThanOrEqual(7)
    expect(contrast(p.text2, page), `${p.id} text-2`).toBeGreaterThanOrEqual(4.5)
    expect(contrast(p.text3, page), `${p.id} text-3`).toBeGreaterThanOrEqual(3)
  }
  // the accent is made readable against the page the same way a picked one is
  expect(contrast(readableAccent(p.accent, p.bg)!, p.bg), `${p.id} accent`).toBeGreaterThanOrEqual(4.5)
}

describe('theme presets', () => {
  it('keep every text level readable on both surfaces', () => {
    for (const p of [LIGHT, OBSIDIAN, ...PRESETS]) readable(p)
  })
  it('have unique ids, a dark flag that matches the page, and many to choose from', () => {
    const ids = PRESETS.map((p) => p.id)
    expect(new Set(ids).size).toBe(ids.length)
    expect(PRESETS.length).toBeGreaterThanOrEqual(25)
    for (const p of PRESETS) expect(p.dark, p.id).toBe(contrast(p.bg, '#000000') < contrast(p.bg, '#ffffff'))
    expect(FAMILIES[0].label).toBe('WibuFest')
  })
  it('carry 0.3 theme ids over to their new names', () => {
    for (const [old, now] of Object.entries(RENAMED)) expect(PRESETS.some((p) => p.id === now), old).toBe(true)
  })
  it('set every surface token', () => {
    const v = presetVars(PRESETS.find((p) => p.id === 'moonbloom-main')!)
    expect(v['--bg']).toBe('#191724')
    expect(v['--bar']).toBe('rgba(25, 23, 36, 0.72)')
    expect(Object.keys(v)).toEqual(['--bg', '--bg-2', '--text', '--text-2', '--text-3', '--fill', '--fill-2', '--sep', '--bar', '--sidebar'])
  })
})

describe('adaptive theme', () => {
  it('stays readable for any cover colour, dark and light', () => {
    for (let h = 0; h < 360; h += 1) {
      for (const [s, l] of [[1, 0.5], [0.6, 0.35], [0.4, 0.7]]) {
        const c = `hsl(${h}, ${s * 100}%, ${l * 100}%)`
        const el = { r: 0, g: 0, b: 0 }
        // the cover colour as #rrggbb (as art_color gives it)
        const k = (n: number) => { const a = s * Math.min(l, 1 - l); const x = (n + h / 30) % 12; return Math.round((l - a * Math.max(-1, Math.min(x - 3, 9 - x, 1))) * 255) }
        Object.assign(el, { r: k(0), g: k(8), b: k(4) })
        const hex = '#' + [el.r, el.g, el.b].map((v) => v.toString(16).padStart(2, '0')).join('')
        for (const dark of [true, false]) {
          const p = adaptivePalette(hex, dark)
          expect(p.dark).toBe(dark)
          readable({ ...p, id: `${c} ${dark ? 'dark' : 'light'}` })
        }
      }
    }
  })
  it('takes the cover hue for its surfaces', () => {
    const p = adaptivePalette('#c8102e', true)
    const [r, g, b] = [1, 3, 5].map((i) => parseInt(p.bg.slice(i, i + 2), 16))
    expect(r).toBeGreaterThan(g)
    expect(r).toBeGreaterThan(b)
  })
})
