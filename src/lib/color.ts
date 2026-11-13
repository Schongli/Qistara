// Accent colours that stay readable: lighter or darker until they hold 4.5:1 against the page.

type RGB = [number, number, number]
const parse = (hex: string): RGB | null => {
  const m = /^#?([0-9a-f]{6})$/i.exec(hex.trim())
  return m ? [0, 2, 4].map((i) => parseInt(m[1].slice(i, i + 2), 16)) as RGB : null
}
const toHex = (c: RGB) => '#' + c.map((v) => Math.round(Math.min(255, Math.max(0, v))).toString(16).padStart(2, '0')).join('')
const lum = (c: RGB) => {
  const [r, g, b] = c.map((v) => {
    const x = v / 255
    return x <= 0.03928 ? x / 12.92 : ((x + 0.055) / 1.055) ** 2.4
  })
  return 0.2126 * r + 0.7152 * g + 0.0722 * b
}
export const contrast = (a: string, b: string) => {
  const [x, y] = [parse(a), parse(b)]
  if (!x || !y) return 1
  const [hi, lo] = [lum(x), lum(y)].sort((p, q) => q - p)
  return (hi + 0.05) / (lo + 0.05)
}
const mix = (c: RGB, to: number, t: number): RGB => c.map((v) => v + (to - v) * t) as RGB

/** The accent for a page colour: the picked colour, mixed towards white (dark page) or black (light
 *  page) only as far as it takes to read as text on it. Invalid input gives null. */
export function readableAccent(hex: string, page: string): string | null {
  const c = parse(hex)
  const p = parse(page)
  if (!c || !p) return null
  const toward = lum(p) < 0.5 ? 255 : 0
  for (let t = 0; t <= 1.0001; t += 0.05) {
    const out = toHex(mix(c, toward, t))
    if (contrast(out, page) >= 4.5) return out
  }
  return toHex([toward, toward, toward])
}

/** Text on an accent-filled button: white or near-black, whichever reads better. */
export const onAccent = (accent: string) => (contrast(accent, '#ffffff') >= contrast(accent, '#04231e') ? '#ffffff' : '#04231e')
