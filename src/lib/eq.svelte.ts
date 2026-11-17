// The live EQ: persisted, pushed into mpv, and retuned in place while a node is dragged.
import { untrack } from 'svelte'
import { cmd, mpv, set } from './player.svelte'
import { afString, clampBand, FLAT, hasGain, preampOf, type Band, type Eq } from './eq'
import { kvLoad, kvSet } from './persist'

export const eq = $state<Eq>(structuredClone(FLAT))
export const userPresets = $state<Record<string, Band[]>>({})
const state = $state({ loaded: false })
Promise.all([kvLoad<Eq>('eq'), kvLoad<Record<string, Band[]>>('eq.presets')]).then(([e, p]) => {
  if (e) Object.assign(eq, e)
  if (p) Object.assign(userPresets, p)
  state.loaded = true
})

export function savePreset(name: string) {
  if (!state.loaded) return // the stored presets aren't in yet: saving now would drop them
  userPresets[name] = $state.snapshot(eq.bands)
  kvSet('eq.presets', $state.snapshot(userPresets))
}

// what changes the filter chain's shape (needs a rebuild) vs. only its numbers (retune in place)
const shape = (e: Eq) => JSON.stringify([e.enabled, e.bands.map((b) => [b.type, b.on])])
let applied = ''
let frame = 0

/** The sample rate the EQ works at: what reaches the output, else the file's own. */
export const eqRate = () => mpv.audioOut?.samplerate || mpv.audioIn?.samplerate || 48000

async function push() {
  frame = 0
  const fs = eqRate()
  const e = $state.snapshot(eq) as Eq
  if (!state.loaded) return
  kvSet('eq', e)
  if (!mpv.ready) return
  // only numbers mpv can use go out, whatever a field holds mid-edit
  e.bands = e.bands.map((b) => clampBand(b, fs))
  const s = shape(e)
  if (s !== applied || !e.enabled) {
    applied = s
    return set('af', afString(e, fs))
  }
  // same chain: send only the new numbers to the labelled filters, no audio reinit
  try {
    await cmd('af-command', 'pre', 'volume', `${preampOf(e, fs).toFixed(2)}dB`)
    for (const [i, b] of e.bands.entries()) {
      if (!b.on) continue
      await cmd('af-command', `eq${i}`, 'f', String(b.f))
      await cmd('af-command', `eq${i}`, 'w', String(b.q))
      if (hasGain(b.type)) await cmd('af-command', `eq${i}`, 'g', String(b.g))
    }
  } catch {
    await set('af', afString(e, fs)) // a filter that can't retune live is simply rebuilt
  }
}

$effect.root(() => {
  $effect(() => {
    JSON.stringify(eq) // track every field
    mpv.ready
    state.loaded
    untrack(() => {
      // at most one update per frame while dragging
      if (!frame) frame = requestAnimationFrame(() => void push())
    })
  })
})
