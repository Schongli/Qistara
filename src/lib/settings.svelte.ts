// Audio settings, saved in the library database. Bit-perfect is the default.
import { untrack } from 'svelte'
import { mpv, set } from './player.svelte'
import { queue } from './queue.svelte'
import { kvGet, kvSet } from './persist'

const defaults = { bitPerfect: true, soundCheck: false, passthrough: false, hwdec: true, onlineLyrics: true }
type Settings = typeof defaults

export const settings = $state<Settings>({ ...defaults })
// nothing is saved or applied until the stored values are in, so they are never overwritten by defaults
const state = $state({ loaded: false })
kvGet<Partial<Settings>>('settings').then((v) => {
  if (v) Object.assign(settings, v)
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
