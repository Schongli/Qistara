import { describe, expect, it } from 'vitest'
import { combo, display, SHORTCUTS, shortcutsFor } from './shortcuts'

const k = (key: string, mods: Partial<Record<'ctrlKey' | 'metaKey' | 'shiftKey' | 'altKey', boolean>> = {}) =>
  ({ key, ctrlKey: false, metaKey: false, shiftKey: false, altKey: false, ...mods })

describe('shortcuts', () => {
  it('spell a key press the way the table does', () => {
    expect(combo(k(' '))).toBe('Space')
    expect(combo(k('l', { ctrlKey: true }))).toBe('Ctrl+L')
    expect(combo(k('L', { ctrlKey: true, shiftKey: true }))).toBe('Ctrl+Shift+L')
    expect(combo(k('ArrowRight', { shiftKey: true }))).toBe('Shift+Right')
    expect(combo(k('ArrowUp', { ctrlKey: true }))).toBe('Ctrl+Up')
    expect(combo(k(',', { ctrlKey: true }))).toBe('Ctrl+,')
    expect(combo(k('/', { ctrlKey: true }))).toBe('Ctrl+/')
    expect(combo(k('ArrowLeft', { altKey: true }))).toBe('Alt+Left')
    expect(combo(k('Escape'))).toBe('Esc')
  })
  it('treat ⌘ as Ctrl, and keep "?" as typed', () => {
    expect(combo(k('s', { metaKey: true }))).toBe('Ctrl+S')
    expect(combo(k('?', { shiftKey: true }))).toBe('?')
  })
  it('have no two app shortcuts on the same keys', () => {
    const app = SHORTCUTS.filter((s) => s.group !== 'Video').map((s) => s.keys)
    expect(new Set(app).size).toBe(app.length)
    for (const s of SHORTCUTS) expect(combo(parse(s.keys)), s.keys).toBe(s.keys)
  })
  it('leave out the video ones where video isn’t shown in the window (macOS, Linux)', () => {
    expect(shortcutsFor(true)).toBe(SHORTCUTS)
    const without = shortcutsFor(false)
    expect(without.some((s) => s.group === 'Video' || s.keys === 'Ctrl+Shift+V')).toBe(false)
    expect(without.map((s) => s.keys)).toContain('Space')
  })
  it('read naturally on each OS', () => {
    expect(display('Ctrl+Shift+L')).toBe('Ctrl+Shift+L')
    expect(display('Shift+Right')).toBe('Shift+→')
    expect(display('Ctrl+Shift+L', true)).toBe('⌘⇧L')
  })
})

/** The key press a table entry stands for. */
function parse(keys: string) {
  const parts = keys.split('+')
  const key = parts.at(-1) === '' ? '+' : parts.pop()!
  const names: Record<string, string> = { Space: ' ', Up: 'ArrowUp', Down: 'ArrowDown', Left: 'ArrowLeft', Right: 'ArrowRight', Esc: 'Escape' }
  return k(names[key] ?? (key.length === 1 ? key.toLowerCase() : key), { ctrlKey: parts.includes('Ctrl'), shiftKey: parts.includes('Shift'), altKey: parts.includes('Alt') })
}
