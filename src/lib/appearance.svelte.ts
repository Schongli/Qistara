// Settings › Appearance, applied to the window: theme, accent (picked or from the cover), zoom,
// row density, solid chrome and font. The CSS tokens are in app.css.
import { invoke } from '@tauri-apps/api/core'
import { getCurrentWindow } from '@tauri-apps/api/window'
import { getCurrentWebview } from '@tauri-apps/api/webview'
import { settings } from './settings.svelte'
import { queue } from './queue.svelte'
import { onAccent, readableAccent } from './color'

export const ACCENTS: [string, string][] = [
  ['River', '#0a7f6f'], ['Ocean', '#0a64d6'], ['Indigo', '#5b4fc4'], ['Orchid', '#9b3fc8'],
  ['Rose', '#d6336c'], ['Ember', '#d9480f'], ['Gold', '#b8860b'], ['Moss', '#2f9e44'], ['Graphite', '#5b6470'],
]

const root = document.documentElement
const system = matchMedia('(prefers-color-scheme: dark)')
const scheme = $state({ systemDark: system.matches })
system.addEventListener('change', (e) => (scheme.systemDark = e.matches))
export const isDark = () => settings.theme === 'dark' || (settings.theme === 'auto' && scheme.systemDark)

const art = $state({ color: null as string | null })
const warn = (what: string) => (e: unknown) => console.warn(`appearance: ${what}:`, e)

$effect.root(() => {
  $effect(() => {
    root.style.colorScheme = settings.theme === 'auto' ? '' : settings.theme
    // the Mica tint and caption follow the window's theme, not the page's
    getCurrentWindow().setTheme(settings.theme === 'auto' ? null : settings.theme).catch(warn('theme'))
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
    const picked = (settings.matchArtwork && art.color) || settings.accent
    const accent = picked && readableAccent(picked, isDark() ? '#1e1e1f' : '#ffffff')
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
    if (settings.font === 'system') root.style.setProperty('--font', "'Segoe UI Variable Text', 'Segoe UI', system-ui, sans-serif")
    else root.style.removeProperty('--font')
  })
})
