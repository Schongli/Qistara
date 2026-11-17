// Settings › Appearance, applied to the window: theme (or a preset palette), accent (picked or from the cover),
// zoom, row density, corners, blur, solid chrome and font. The CSS tokens are in app.css.
import { invoke } from '@tauri-apps/api/core'
import { getCurrentWindow } from '@tauri-apps/api/window'
import { getCurrentWebview } from '@tauri-apps/api/webview'
import { settings } from './settings.svelte'
import { queue } from './queue.svelte'
import { OS } from './player.svelte'
import { onAccent, readableAccent } from './color'
import { adaptivePalette, presetOf, presetVars, type Palette } from './themes'

export const ACCENTS: [string, string][] = [
  ['Brass', '#e5a93c'], ['River', '#0a7f6f'], ['Ocean', '#0a64d6'], ['Indigo', '#5b4fc4'], ['Orchid', '#9b3fc8'],
  ['Rose', '#d6336c'], ['Ember', '#d9480f'], ['Moss', '#2f9e44'], ['Graphite', '#5b6470'],
]

const root = document.documentElement
// macOS keeps its own traffic lights top-left (tauri.macos.conf.json); Linux and Windows get Caps.svelte
root.classList.add(`os-${OS}`)
const system = matchMedia('(prefers-color-scheme: dark)')
const scheme = $state({ systemDark: system.matches })
system.addEventListener('change', (e) => (scheme.systemDark = e.matches))
const preset = $derived(presetOf(settings.theme))
/** Whether the chosen theme is dark (Automatic follows the system). */
const chosenDark = () => (preset ? preset.dark : settings.theme === 'dark' || (settings.theme === 'auto' && scheme.systemDark))
const art = $state({ color: null as string | null })
/** Adaptive theme: the whole palette from the cover's colour, dark or light as the chosen theme is. */
const palette = $derived<Palette | undefined>(settings.adaptiveTheme && art.color ? adaptivePalette(art.color, chosenDark()) : preset)
export const isDark = () => (palette ? palette.dark : chosenDark())
const RADIUS = { square: 0, subtle: 0.5, rounded: 1, round: 1.75 }

const warn = (what: string) => (e: unknown) => console.warn(`appearance: ${what}:`, e)

$effect.root(() => {
  $effect(() => {
    const scheme = palette ? (palette.dark ? 'dark' : 'light') : settings.theme === 'auto' ? null : (settings.theme as 'light' | 'dark')
    root.style.colorScheme = scheme ?? ''
    // the Mica tint and caption follow the window's theme, not the page's
    getCurrentWindow().setTheme(scheme).catch(warn('theme'))
  })
  let lastPalette = ''
  $effect(() => {
    const p = palette
    const vars = p ? presetVars(p) : {}
    // inline values beat the .solid / .noblur rules in app.css, so a preset applies them itself
    if (p && (settings.solidChrome || !settings.blurUi)) vars['--bar'] = p.bg
    if (p && settings.solidChrome) vars['--sidebar'] = p.bg2
    // the adaptive theme changes with every song: let the colours ease over instead of jumping
    const key = p ? p.bg + p.text : ''
    if (lastPalette && key !== lastPalette && settings.adaptiveTheme && !matchMedia('(prefers-reduced-motion: reduce)').matches) {
      root.classList.add('fade')
      setTimeout(() => root.classList.remove('fade'), 700)
    }
    lastPalette = key
    for (const k of ['--bg', '--bg-2', '--text', '--text-2', '--text-3', '--fill', '--fill-2', '--sep', '--bar', '--sidebar']) {
      if (vars[k]) root.style.setProperty(k, vars[k])
      else root.style.removeProperty(k)
    }
  })
  $effect(() => {
    getCurrentWebview().setZoom(settings.zoom).catch(warn('zoom'))
  })
  $effect(() => {
    const cover = settings.matchArtwork || settings.adaptiveTheme ? queue.current?.art : null
    art.color = null
    if (cover) invoke<string | null>('art_color', { art: cover }).then((c) => queue.current?.art === cover && (art.color = c), warn('artwork colour'))
  })
  $effect(() => {
    const picked = ((settings.matchArtwork || settings.adaptiveTheme) && art.color) || settings.accent || preset?.accent
    const accent = picked && readableAccent(picked, palette?.bg ?? (isDark() ? '#111318' : '#ffffff'))
    if (accent) {
      root.style.setProperty('--accent', accent)
      root.style.setProperty('--on-accent', onAccent(accent))
    } else {
      root.style.removeProperty('--accent')
      root.style.removeProperty('--on-accent')
    }
  })
  $effect(() => {
    root.classList.toggle('solid', settings.solidChrome)
    root.classList.toggle('compact', settings.density === 'compact')
    root.classList.toggle('noblur', !settings.blurUi)
    root.style.setProperty('--rk', String(RADIUS[settings.corners] ?? 1))
    if (settings.font === 'system') root.style.setProperty('--font', "'Segoe UI Variable Text', 'Segoe UI', system-ui, sans-serif")
    else root.style.removeProperty('--font')
  })
})
