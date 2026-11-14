// Settings › Appearance, applied to the window: theme (or a preset palette), accent (picked or from the cover),
// zoom, row density, corners, blur, solid chrome and font. The CSS tokens are in app.css.
import { invoke } from '@tauri-apps/api/core'
import { getCurrentWindow } from '@tauri-apps/api/window'
import { getCurrentWebview } from '@tauri-apps/api/webview'
import { settings } from './settings.svelte'
import { queue } from './queue.svelte'
import { onAccent, readableAccent } from './color'
import { presetOf, presetVars } from './themes'

export const ACCENTS: [string, string][] = [
  ['Brass', '#e5a93c'], ['River', '#0a7f6f'], ['Ocean', '#0a64d6'], ['Indigo', '#5b4fc4'], ['Orchid', '#9b3fc8'],
  ['Rose', '#d6336c'], ['Ember', '#d9480f'], ['Moss', '#2f9e44'], ['Graphite', '#5b6470'],
]

const root = document.documentElement
const system = matchMedia('(prefers-color-scheme: dark)')
const scheme = $state({ systemDark: system.matches })
system.addEventListener('change', (e) => (scheme.systemDark = e.matches))
const preset = $derived(presetOf(settings.theme))
// the presets are all dark themes
export const isDark = () => !!preset || settings.theme === 'dark' || (settings.theme === 'auto' && scheme.systemDark)
const RADIUS = { square: 0, subtle: 0.5, rounded: 1, round: 1.75 }

const art = $state({ color: null as string | null })
const warn = (what: string) => (e: unknown) => console.warn(`appearance: ${what}:`, e)

$effect.root(() => {
  $effect(() => {
    const scheme = preset ? 'dark' : settings.theme === 'auto' ? null : (settings.theme as 'light' | 'dark')
    root.style.colorScheme = scheme ?? ''
    // the Mica tint and caption follow the window's theme, not the page's
    getCurrentWindow().setTheme(scheme).catch(warn('theme'))
  })
  $effect(() => {
    const vars = preset ? presetVars(preset) : {}
    // inline values beat the .solid / .noblur rules in app.css, so a preset applies them itself
    if (preset && (settings.solidChrome || !settings.blurUi)) vars['--bar'] = preset.bg
    if (preset && settings.solidChrome) vars['--sidebar'] = preset.bg2
    for (const k of ['--bg', '--bg-2', '--text', '--text-2', '--text-3', '--fill', '--fill-2', '--sep', '--bar', '--sidebar']) {
      if (vars[k]) root.style.setProperty(k, vars[k])
      else root.style.removeProperty(k)
    }
  })
  $effect(() => {
    getCurrentWebview().setZoom(settings.zoom).catch(warn('zoom'))
  })
  $effect(() => {
    const cover = settings.matchArtwork ? queue.current?.art : null
    art.color = null
    if (cover) invoke<string | null>('art_color', { art: cover }).then((c) => queue.current?.art === cover && (art.color = c), warn('artwork colour'))
  })
  $effect(() => {
    const picked = (settings.matchArtwork && art.color) || settings.accent || preset?.accent
    const accent = picked && readableAccent(picked, preset?.bg ?? (isDark() ? '#111318' : '#ffffff'))
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
