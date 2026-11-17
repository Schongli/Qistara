// Bit-perfect verdict: compares what the decoder produced with what the output (WASAPI, CoreAudio, ALSA) receives.

export type Os = 'windows' | 'macos' | 'linux'
export const osOf = (ua: string): Os => (/Windows/.test(ua) ? 'windows' : /Macintosh|Mac OS X/.test(ua) ? 'macos' : 'linux')
/** The one output per OS that can skip the system mixer: WASAPI exclusive, CoreAudio hog mode, ALSA hw:. */
export const AO: Record<Os, string> = { windows: 'wasapi', macos: 'coreaudio', linux: 'alsa' }
/** ALSA devices Qistara lets you pick: plain hardware PCMs (the same rule as device() in mpv.rs). */
export const ALSA_DEVICE = /^alsa\/(default|sysdefault|hw|plughw|front|iec958|hdmi)(:CARD=\w{1,32}(,DEV=\d{1,2})?)?$/
export function pickable(os: Os, name: string): boolean {
  if (name === 'auto') return true
  if (os === 'windows') return name.startsWith('wasapi/')
  if (os === 'macos') return /^coreaudio\/.{1,256}$/.test(name)
  return ALSA_DEVICE.test(name)
}
/** "WASAPI exclusive", "CoreAudio hog mode", "ALSA hw:"… for the Signal Path. */
export function outputLabel(ao: string | null, exclusive: boolean, device = ''): string {
  if (ao === 'wasapi') return `WASAPI ${exclusive ? 'exclusive' : 'shared'}`
  if (ao === 'coreaudio_exclusive') return 'CoreAudio hog mode'
  if (ao === 'coreaudio') return 'CoreAudio shared'
  if (ao === 'alsa') return `ALSA ${/^alsa\/(hw|iec958|hdmi):/.test(device) ? 'direct to hardware' : 'through the system mixer'}`
  return ao ?? ''
}

export interface AudioParams {
  format: string
  samplerate: number
  'channel-count': number
}

export interface SignalInput {
  audioIn: AudioParams | null
  audioOut: AudioParams | null
  ao: string | null // current-ao: wasapi, coreaudio_exclusive (hog mode granted), coreaudio, alsa…
  exclusive: boolean // what was asked for; on WASAPI that is what you get
  device?: string
  filters: number // entries in mpv's af chain (EQ etc.)
  volume: number
  speed: number
  replayGain: boolean
  sourceBits?: number // real bit depth from tags (24-bit FLAC decodes into s32)
}

export interface Verdict {
  bitPerfect: boolean
  issues: string[]
}

const isFloat = (f: string) => /^(float|double)/.test(f)

export function bitsOf(format: string): number {
  const m = format.match(/^(?:u|s)(\d+)|^(float)|^(double)/)
  if (!m) return 0
  return m[1] ? Number(m[1]) : m[2] ? 32 : 64
}

export function verdict(s: SignalInput): Verdict {
  const issues: string[] = []
  const { audioIn: a, audioOut: o } = s
  if (!a || !o) return { bitPerfect: false, issues: ['Nothing playing'] }
  // S/PDIF/HDMI bitstream: the encoded stream reaches the receiver untouched
  if (o.format.startsWith('spdif')) return { bitPerfect: true, issues: [] }

  const direct = s.ao === 'wasapi' ? s.exclusive : s.ao === 'coreaudio_exclusive' || (s.ao === 'alsa' && /^alsa\/(hw|iec958|hdmi):/.test(s.device ?? ''))
  if (!direct) issues.push(
    s.ao === 'wasapi' ? 'Shared mode: Windows mixer may resample'
    : s.ao?.startsWith('coreaudio') ? 'Shared mode: macOS may resample'
    : s.ao === 'alsa' ? 'Not a hw: device: ALSA or PipeWire may resample'
    : 'Shared mode: the system mixer may resample',
  )
  if (a.samplerate !== o.samplerate) issues.push(`Resampled ${khz(a.samplerate)} → ${khz(o.samplerate)}`)
  // Mono sent to both speakers copies samples unchanged, so it stays bit-perfect.
  const monoToStereo = a['channel-count'] === 1 && o['channel-count'] === 2
  if (a['channel-count'] !== o['channel-count'] && !monoToStereo)
    issues.push(`${a['channel-count'] > o['channel-count'] ? 'Downmixed' : 'Upmixed'} ${a['channel-count']}ch → ${o['channel-count']}ch`)
  if (s.filters > 0) issues.push('Processed (EQ)')
  if (s.replayGain) issues.push('Sound Check changes level')
  if (Math.round(s.volume) !== 100) issues.push('Software volume')
  if (s.speed !== 1) issues.push('Playback speed changed')

  const fin = a.format.replace(/p$/, ''), fout = o.format.replace(/p$/, '')
  if (isFloat(fin) !== isFloat(fout)) issues.push(`Converted ${fin} → ${fout}`)
  else if (isFloat(fin) && bitsOf(fout) < bitsOf(fin)) issues.push(`Truncated ${fin} → ${fout}`)
  else if (!isFloat(fin) && bitsOf(fout) < (s.sourceBits ?? bitsOf(fin))) issues.push(`Truncated to ${bitsOf(fout)}-bit`)

  return { bitPerfect: issues.length === 0, issues }
}

export const khz = (hz: number) => `${+(hz / 1000).toFixed(1)} kHz`
