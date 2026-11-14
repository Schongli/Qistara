// Audio settings, saved in the library database. Bit-perfect is the default.
import { untrack } from 'svelte'
import { mpv, set } from './player.svelte'
import { queue } from './queue.svelte'
import { kvLoad, kvSet } from './persist'
import type { Preset } from './downloads.svelte'
import { normalizeSources } from './lyricsources'
import type { ThemeId } from './themes'

// lyricsLanguage '' = the Windows display language
// quranTranslations: ids from quran.svelte.ts TRANSLATIONS, shown under each ayah
const defaults = {
  bitPerfect: true, soundCheck: false, passthrough: false, hwdec: true, translateLyrics: false, lyricsLanguage: '', learnLyrics: false,
  quranTranslations: ['kemenag'] as string[], quranTranslit: true,
  // Settings › Lyrics: where lyrics come from, in order; preferWordSync keeps asking for word timing
  lyricsSources: normalizeSources(undefined), preferWordSync: false,
  // Appearance (appearance.svelte.ts): accent '' = the default brass; zoom scales the whole UI
  theme: 'auto' as ThemeId, accent: '', matchArtwork: false, zoom: 1,
  corners: 'rounded' as 'square' | 'subtle' | 'rounded' | 'round', blurUi: true,
  iconPack: 'phosphor' as 'phosphor' | 'lucide' | 'iconoir' | 'remix' | 'solar',
  density: 'comfortable' as 'comfortable' | 'compact', solidChrome: false, font: 'inter' as 'inter' | 'system',
  hiddenNav: [] as string[],
  // YouTube sign-in for yt-dlp: '' guest, 'firefox' (its cookies), or the path of a cookies.txt file
  ytCookies: '',
  // Downloads: the folder ('' = Music\Qistara Downloads), format and whether new downloads join the library
  downloadFolder: '', downloadPreset: 'audioOriginal' as Preset, downloadToLibrary: true,
}
type Settings = typeof defaults

export const settings = $state<Settings>({ ...defaults })
// nothing is saved or applied until the stored values are in, so they are never overwritten by defaults
const state = $state({ loaded: false })
kvLoad<Partial<Settings> & { onlineLyrics?: boolean }>('settings').then((v) => {
  if (v) Object.assign(settings, v)
  // the one "Find lyrics online" switch became the source list
  settings.lyricsSources = normalizeSources(v?.lyricsSources, v?.onlineLyrics)
  delete (settings as { onlineLyrics?: boolean }).onlineLyrics
  state.loaded = true
})

const PASSTHROUGH = 'ac3,eac3,dts,dts-hd,truehd'

let lastExclusive: boolean | null = null

/** Pushes the settings into mpv. Bit-perfect: exclusive output, unity gain, no Sound Check. */
export async function applyAudio() {
  const s = settings
  await set('audio-exclusive', s.bitPerfect ? 'yes' : 'no')
  // mpv drops the output device on this change and doesn't always reopen it (seen with EQ
  // filters active), so reopen the current file in place: a short blip, never silence
  if (lastExclusive !== null && lastExclusive !== s.bitPerfect) await queue.reload()
  lastExclusive = s.bitPerfect
  await set('replaygain', !s.bitPerfect && s.soundCheck ? 'album' : 'no')
  await set('audio-spdif', s.passthrough ? PASSTHROUGH : '')
  await set('hwdec', s.hwdec ? 'auto-safe' : 'no')
  if (s.bitPerfect) await set('volume', 100)
}

/** Volume lives on the device in bit-perfect mode, so the samples are never scaled. */
export const volume = () => (settings.bitPerfect ? (mpv.aoVolume ?? 100) : mpv.volume)
export const setVolume = (v: number) => set(settings.bitPerfect ? 'ao-volume' : 'volume', Math.round(v))

let target: number | null = null
let settle: ReturnType<typeof setTimeout>
/** Steps the volume; quick repeats (wheel, held key) build on the last request, since mpv reports it back later. */
export function nudgeVolume(by: number) {
  target = Math.min(100, Math.max(0, (target ?? volume()) + by))
  setVolume(target)
  clearTimeout(settle)
  settle = setTimeout(() => (target = null), 400)
}

$effect.root(() => {
  $effect(() => {
    const snapshot = $state.snapshot(settings)
    if (!state.loaded) return
    untrack(() => {
      kvSet('settings', snapshot)
      if (mpv.ready) applyAudio()
    })
  })
  $effect(() => {
    if (mpv.ready && state.loaded) untrack(applyAudio)
  })
})
