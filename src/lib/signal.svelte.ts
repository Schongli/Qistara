// The live bit-perfect verdict for what is playing now (Signal Path popover, player-bar badge, the logo's pivot dot).
import { mpv } from './player.svelte'
import { queue } from './queue.svelte'
import { settings } from './settings.svelte'
import { verdict } from './signal'

const v = $derived(verdict({
  audioIn: mpv.audioIn, audioOut: mpv.audioOut, ao: mpv.ao, exclusive: settings.bitPerfect, device: mpv.device ?? '',
  filters: mpv.af.length, volume: mpv.volume, speed: mpv.speed,
  replayGain: !settings.bitPerfect && settings.soundCheck, sourceBits: queue.current?.bitDepth ?? undefined,
}))
export const signal = { get verdict() { return v } }
