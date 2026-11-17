// Themes for Settings › Appearance. Automatic, Light and Obsidian are app.css's own light-dark() tokens; every
// other palette is set inline over them (appearance.svelte.ts). Families have variants (a dark and a light one,
// or several shades), picked under the family's card. Names are Qistara's own; several palettes follow well-known
// open-source schemes (credited in the README), the rest are original.

export interface Palette {
  id: string
  label: string // the variant's name ("Dusk"), or the family's when it has one variant
  family: string
  dark: boolean
  bg: string // page
  bg2: string // cards, sidebar
  text: string
  text2: string
  text3: string
  accent: string // the theme's own accent, used while no accent is picked
}
export interface Family { id: string; label: string; note?: string; variants: Palette[] }

type Row = [id: string, label: string, dark: boolean, bg: string, bg2: string, text: string, text2: string, text3: string, accent: string]
const fam = (id: string, label: string, rows: Row[], note?: string): Family => ({
  id, label, note,
  variants: rows.map(([vid, vl, dark, bg, bg2, text, text2, text3, accent]) => ({ id: vid, label: vl, family: id, dark, bg, bg2, text, text2, text3, accent })),
})

/** Only for the theme cards' little previews: the real values are in app.css. */
export const LIGHT: Palette = { id: 'light', label: 'Light', family: 'qistara', dark: false, bg: '#ffffff', bg2: '#f5f5f7', text: '#1d1d1f', text2: '#6e6e73', text3: '#8a8a8f', accent: '#8f5e0e' }
export const OBSIDIAN: Palette = { id: 'dark', label: 'Obsidian', family: 'qistara', dark: true, bg: '#111318', bg2: '#181b22', text: '#f1f3f9', text2: '#8e95a5', text3: '#6c7384', accent: '#e5a93c' }

export const FAMILIES: Family[] = [
  fam('wibufest', 'WibuFest', [
    ['wibufest-crimson', 'Crimson', true, '#1a0709', '#24090d', '#fbeef0', '#e0b9bf', '#b5848b', '#c8102e'],
    ['wibufest-shrine', 'Shrine', false, '#ffffff', '#fbf0f1', '#2a0a0e', '#6b3a41', '#8f5f66', '#8b0d1f'],
  ], 'The festival’s crimson and shrine white'),
  fam('pastel', 'Pastel', [
    ['pastel-day', 'Day', false, '#eff1f5', '#e6e9ef', '#3f4259', '#5c5f77', '#6c6f85', '#8839ef'],
    ['pastel-dusk', 'Dusk', true, '#303446', '#292c3c', '#c6d0f5', '#a5adce', '#949cbb', '#ca9ee6'],
    ['pastel-evening', 'Evening', true, '#24273a', '#1e2030', '#cad3f5', '#a5adcb', '#939ab7', '#c6a0f6'],
    ['pastel-night', 'Night', true, '#1e1e2e', '#181825', '#cdd6f4', '#a6adc8', '#9399b2', '#cba6f7'],
  ], 'After Catppuccin'),
  fam('moonbloom', 'Moonbloom', [
    ['moonbloom-main', 'Main', true, '#191724', '#1f1d2e', '#e0def4', '#908caa', '#7f7b99', '#c4a7e7'],
    ['moonbloom-haze', 'Haze', true, '#232136', '#2a273f', '#e0def4', '#a19dbb', '#8a86a3', '#c4a7e7'],
    ['moonbloom-dawn', 'Dawn', false, '#faf4ed', '#fffaf3', '#4a456a', '#696590', '#8a85a0', '#907aa9'],
  ], 'After Rosé Pine'),
  fam('timber', 'Timber', [
    ['timber-dark', 'Dark', true, '#282828', '#32302f', '#ebdbb2', '#bdae93', '#a89984', '#fabd2f'],
    ['timber-light', 'Light', false, '#fbf1c7', '#f2e5bc', '#3c3836', '#504945', '#7c6f64', '#b57614'],
  ], 'After Gruvbox'),
  fam('sundial', 'Sundial', [
    ['sundial-dark', 'Dark', true, '#002b36', '#073642', '#eee8d5', '#93a1a1', '#839496', '#268bd2'],
    ['sundial-light', 'Light', false, '#fdf6e3', '#eee8d5', '#073642', '#4f6168', '#6c7f86', '#268bd2'],
  ], 'After Solarized'),
  fam('fjord', 'Fjord', [
    ['fjord-night', 'Night', true, '#2e3440', '#3b4252', '#eceff4', '#d8dee9', '#a3acbd', '#88c0d0'],
    ['fjord-snow', 'Snow', false, '#eceff4', '#e5e9f0', '#2e3440', '#4c566a', '#646e82', '#5e81ac'],
  ], 'After Nord'),
  fam('nocturne', 'Nocturne', [['nocturne', 'Nocturne', true, '#282a36', '#21222c', '#f8f8f2', '#c5c8d6', '#9ea3bd', '#bd93f9']], 'After Dracula'),
  fam('neon', 'Neon City', [
    ['neon-night', 'Night', true, '#1a1b26', '#16161e', '#c0caf5', '#a9b1d6', '#8089b3', '#7aa2f7'],
    ['neon-storm', 'Storm', true, '#24283b', '#1f2335', '#c0caf5', '#a9b1d6', '#8a92bd', '#7dcfff'],
    ['neon-day', 'Day', false, '#e1e2e7', '#d5d6db', '#343b58', '#4c5374', '#646b8a', '#2e7de9'],
  ], 'After Tokyo Night'),
  fam('evergreen', 'Evergreen', [
    ['evergreen-dark', 'Dark', true, '#2d353b', '#343f44', '#e0d6bf', '#b4ab91', '#9da9a0', '#a7c080'],
    ['evergreen-light', 'Light', false, '#fdf6e3', '#f4f0d9', '#3e484d', '#5c6a72', '#77857f', '#8da101'],
  ], 'After Everforest'),
  fam('ukiyo', 'Ukiyo', [
    ['ukiyo-wave', 'Wave', true, '#1f1f28', '#16161d', '#dcd7ba', '#c8c093', '#938e7d', '#7e9cd8'],
    ['ukiyo-lotus', 'Lotus', false, '#f2ecbc', '#e7dba0', '#383860', '#545464', '#6b6a5e', '#4d699b'],
  ], 'After Kanagawa'),
  fam('slate', 'Slate', [['slate', 'Slate', true, '#282c34', '#21252b', '#e2e5ea', '#abb2bf', '#8b919c', '#61afef']], 'After One Dark'),
  fam('andalus', 'Andalus', [
    ['andalus-night', 'Night', true, '#1d1412', '#261a17', '#f3e6d8', '#c9b29c', '#a38d7a', '#d9a441'],
    ['andalus-sand', 'Sand', false, '#f6eedf', '#efe3cd', '#3b2a20', '#6b5444', '#86705f', '#b5562f'],
  ], 'Terracotta and gold'),
  fam('madinah', 'Madinah', [['madinah', 'Madinah', true, '#0f1a2b', '#142238', '#eef1f6', '#b4bfd1', '#8794ab', '#d4af37']], 'Night blue and gold'),
  fam('sakura', 'Sakura', [['sakura', 'Sakura', false, '#fff5f7', '#fde8ee', '#3d2a33', '#6e5560', '#8d7280', '#d6336c']], 'Blossom pink'),
  fam('matcha', 'Matcha', [['matcha', 'Matcha', true, '#1a211b', '#212a22', '#e6eedf', '#b7c4ad', '#93a18b', '#9fca73']], 'Tea green'),
  fam('pitch', 'Pitch Black', [['pitch', 'Pitch Black', true, '#000000', '#0b0b0b', '#f2f2f2', '#b3b3b3', '#8c8c8c', '#e5a93c']], 'True black for OLED screens'),
]

export const PRESETS: Palette[] = FAMILIES.flatMap((f) => f.variants)
export type ThemeId = 'auto' | 'light' | 'dark' | string
export const presetOf = (id: string) => PRESETS.find((p) => p.id === id)
export const familyOf = (id: string) => FAMILIES.find((f) => f.variants.some((v) => v.id === id))

/** 0.3's theme ids, renamed in 0.4. */
export const RENAMED: Record<string, string> = {
  'catppuccin-frappe': 'pastel-dusk', 'rose-pine': 'moonbloom-main', 'gruvbox-dark': 'timber-dark', 'solarized-dark': 'sundial-dark',
}

const rgba = (hex: string, a: number) => `rgba(${[1, 3, 5].map((i) => parseInt(hex.slice(i, i + 2), 16)).join(', ')}, ${a})`

/** The CSS tokens a palette sets on the root; app.css supplies everything else. */
export function presetVars(p: Pick<Palette, 'bg' | 'bg2' | 'text' | 'text2' | 'text3'>): Record<string, string> {
  return {
    '--bg': p.bg,
    '--bg-2': p.bg2,
    '--text': p.text,
    '--text-2': p.text2,
    '--text-3': p.text3,
    '--fill': rgba(p.text, 0.06),
    '--fill-2': rgba(p.text, 0.1),
    '--sep': rgba(p.text, 0.12),
    '--bar': rgba(p.bg, 0.72),
    '--sidebar': rgba(p.bg2, 0.4),
  }
}

// ---- adaptive theme: the whole palette from the cover of what's playing ----

const hsl = (hex: string): [number, number, number] => {
  const [r, g, b] = [1, 3, 5].map((i) => parseInt(hex.slice(i, i + 2), 16) / 255)
  const max = Math.max(r, g, b), min = Math.min(r, g, b), l = (max + min) / 2
  if (max === min) return [0, 0, l]
  const d = max - min
  const s = l > 0.5 ? d / (2 - max - min) : d / (max + min)
  const h = max === r ? (g - b) / d + (g < b ? 6 : 0) : max === g ? (b - r) / d + 2 : (r - g) / d + 4
  return [h * 60, s, l]
}
const hex = (h: number, s: number, l: number) => {
  const f = (n: number) => {
    const k = (n + h / 30) % 12
    const c = l - s * Math.min(l, 1 - l) * Math.max(-1, Math.min(k - 3, 9 - k, 1))
    return Math.round(c * 255).toString(16).padStart(2, '0')
  }
  return `#${f(0)}${f(8)}${f(4)}`
}

/**
 * A palette in the cover's hue: deep and quiet surfaces (dark) or pale ones (light), text in a whisper of the same
 * hue, and the cover's colour as the accent (made readable later, like any accent).
 */
export function adaptivePalette(color: string, dark: boolean): Palette {
  const [h, s] = hsl(color)
  const tint = Math.min(s, 0.6)
  const p = dark
    ? { bg: hex(h, tint * 0.45, 0.1), bg2: hex(h, tint * 0.4, 0.135), text: hex(h, 0.25, 0.95), text2: hex(h, 0.14, 0.76), text3: hex(h, 0.1, 0.6) }
    : { bg: hex(h, tint * 0.55, 0.965), bg2: hex(h, tint * 0.45, 0.925), text: hex(h, 0.3, 0.12), text2: hex(h, 0.16, 0.32), text3: hex(h, 0.12, 0.42) }
  return { id: 'adaptive', label: 'Adaptive', family: 'adaptive', dark, accent: color, ...p }
}
