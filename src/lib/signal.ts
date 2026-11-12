// Bit-perfect verdict: compares what the decoder produced with what WASAPI receives.

export interface AudioParams {
  format: string
  samplerate: number
  'channel-count': number
}

export interface SignalInput {
  audioIn: AudioParams | null
  audioOut: AudioParams | null
  ao: string | null
  exclusive: boolean
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

  if (s.ao !== 'wasapi' || !s.exclusive) issues.push('Shared mode: Windows mixer may resample')
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
