// Theme presets for Settings › Appearance. Automatic, Light and Qistara Obsidian are app.css's own light-dark()
// tokens; the community palettes below are dark themes set inline over them (appearance.svelte.ts).

export interface Palette {
  id: string
  label: string
  bg: string // page
  bg2: string // cards, sidebar
  text: string
  text2: string
  text3: string
  accent: string // the theme's own accent, used while no accent is picked
}

/** Only for the theme cards' little previews: the real values are in app.css. */
export const LIGHT: Palette = { id: 'light', label: 'Light', bg: '#ffffff', bg2: '#f5f5f7', text: '#1d1d1f', text2: '#6e6e73', text3: '#a1a1a6', accent: '#8f5e0e' }
export const OBSIDIAN: Palette = { id: 'dark', label: 'Qistara Obsidian', bg: '#111318', bg2: '#181b22', text: '#f1f3f9', text2: '#8e95a5', text3: '#646b7b', accent: '#e5a93c' }

export const PRESETS: Palette[] = [
  { id: 'catppuccin-frappe', label: 'Catppuccin Frappé', bg: '#303446', bg2: '#292c3c', text: '#c6d0f5', text2: '#a5adce', text3: '#838ba7', accent: '#ca9ee6' },
  { id: 'rose-pine', label: 'Rosé Pine', bg: '#191724', bg2: '#1f1d2e', text: '#e0def4', text2: '#908caa', text3: '#6e6a86', accent: '#c4a7e7' },
  { id: 'gruvbox-dark', label: 'Gruvbox Dark', bg: '#282828', bg2: '#32302f', text: '#ebdbb2', text2: '#bdae93', text3: '#928374', accent: '#fabd2f' },
  { id: 'solarized-dark', label: 'Solarized Dark', bg: '#002b36', bg2: '#073642', text: '#eee8d5', text2: '#93a1a1', text3: '#657b83', accent: '#268bd2' },
]

export type ThemeId = 'auto' | 'light' | 'dark' | (typeof PRESETS)[number]['id']
export const presetOf = (id: string) => PRESETS.find((p) => p.id === id)

const rgba = (hex: string, a: number) => `rgba(${[1, 3, 5].map((i) => parseInt(hex.slice(i, i + 2), 16)).join(', ')}, ${a})`

/** The CSS tokens a preset sets on the root; app.css supplies everything else. */
export function presetVars(p: Palette): Record<string, string> {
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
