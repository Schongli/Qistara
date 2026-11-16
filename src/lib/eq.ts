// Parametric EQ: bands → mpv/FFmpeg lavfi filters, and the RBJ biquad response for drawing the curve.

export type BandType = 'peak' | 'lowshelf' | 'highshelf' | 'lowcut' | 'highcut'
export interface Band { type: BandType; f: number; g: number; q: number; on: boolean }
export interface Eq { enabled: boolean; autoPreamp: boolean; preamp: number; bands: Band[] }

export const MAX_BANDS = 10
export const TYPES: [BandType, string][] = [['peak', 'Peak'], ['lowshelf', 'Low Shelf'], ['highshelf', 'High Shelf'], ['lowcut', 'Low Cut'], ['highcut', 'High Cut']]
export const hasGain = (t: BandType) => t !== 'lowcut' && t !== 'highcut'

const num = (n: number) => +n.toFixed(3)

/** One FFmpeg filter for a band (FFmpeg's biquads use the same RBJ cookbook as `response`). */
export function filterOf(b: Band): string {
  const f = num(b.f), w = num(b.q), g = num(b.g)
  switch (b.type) {
    case 'peak': return `equalizer=f=${f}:t=q:w=${w}:g=${g}`
    case 'lowshelf': return `lowshelf=f=${f}:t=q:w=${w}:g=${g}`
    case 'highshelf': return `highshelf=f=${f}:t=q:w=${w}:g=${g}`
    case 'lowcut': return `highpass=f=${f}:p=2:t=q:w=${w}`
    case 'highcut': return `lowpass=f=${f}:p=2:t=q:w=${w}`
  }
}

/** mpv `af` value: labelled filters (@eq0…, @pre) so drags can retune them with af-command. */
export function afString(eq: Eq): string {
  if (!eq.enabled) return ''
  const parts = eq.bands.flatMap((b, i) => (b.on ? [`@eq${i}:lavfi=[${filterOf(b)}]`] : []))
  if (!parts.length) return ''
  const pre = preampOf(eq)
  return [`@pre:lavfi=[volume=${num(pre)}dB]`, ...parts].join(',')
}

function coeffs(b: Band, fs: number) {
  const w0 = (2 * Math.PI * Math.min(b.f, fs * 0.49)) / fs
  const cos = Math.cos(w0), sin = Math.sin(w0)
  const alpha = sin / (2 * Math.max(0.05, b.q))
  const A = Math.pow(10, b.g / 40)
  const s = 2 * Math.sqrt(A) * alpha
  switch (b.type) {
    case 'peak': return [1 + alpha * A, -2 * cos, 1 - alpha * A, 1 + alpha / A, -2 * cos, 1 - alpha / A]
    case 'lowshelf': return [
      A * (A + 1 - (A - 1) * cos + s), 2 * A * (A - 1 - (A + 1) * cos), A * (A + 1 - (A - 1) * cos - s),
      A + 1 + (A - 1) * cos + s, -2 * (A - 1 + (A + 1) * cos), A + 1 + (A - 1) * cos - s]
    case 'highshelf': return [
      A * (A + 1 + (A - 1) * cos + s), -2 * A * (A - 1 + (A + 1) * cos), A * (A + 1 + (A - 1) * cos - s),
      A + 1 - (A - 1) * cos + s, 2 * (A - 1 - (A + 1) * cos), A + 1 - (A - 1) * cos - s]
    case 'lowcut': return [(1 + cos) / 2, -(1 + cos), (1 + cos) / 2, 1 + alpha, -2 * cos, 1 - alpha]
    case 'highcut': return [(1 - cos) / 2, 1 - cos, (1 - cos) / 2, 1 + alpha, -2 * cos, 1 - alpha]
  }
}

/** Gain in dB of one band at frequency `f`. */
export function response(b: Band, f: number, fs = 48000): number {
  const [b0, b1, b2, a0, a1, a2] = coeffs(b, fs)
  const w = (2 * Math.PI * f) / fs
  const c1 = Math.cos(w), c2 = Math.cos(2 * w), s1 = Math.sin(w), s2 = Math.sin(2 * w)
  const num = (b0 + b1 * c1 + b2 * c2) ** 2 + (b1 * s1 + b2 * s2) ** 2
  const den = (a0 + a1 * c1 + a2 * c2) ** 2 + (a1 * s1 + a2 * s2) ** 2
  return 10 * Math.log10(num / den)
}

/** Sum of all enabled bands (without preamp) at `f`. */
export const total = (eq: Eq, f: number) => eq.bands.reduce((s, b) => (b.on ? s + response(b, f) : s), 0)

/** Log-spaced 20 Hz – 20 kHz. */
export const freqs = (n = 160) => Array.from({ length: n }, (_, i) => 20 * Math.pow(1000, i / (n - 1)))

/** Auto preamp lowers the level by the curve's highest boost so the EQ never clips. */
export function preampOf(eq: Eq): number {
  if (!eq.autoPreamp) return eq.preamp
  // band centres are sampled too, so a peak's exact maximum is never missed between grid points
  const at = [...freqs(240), ...eq.bands.filter((b) => b.on).map((b) => b.f)]
  return -Math.max(0, ...at.map((f) => total(eq, f)))
}

export const FLAT: Eq = {
  enabled: false, autoPreamp: true, preamp: 0,
  bands: [
    { type: 'lowshelf', f: 105, g: 0, q: 0.7, on: true },
    { type: 'peak', f: 250, g: 0, q: 1, on: true },
    { type: 'peak', f: 1000, g: 0, q: 1, on: true },
    { type: 'peak', f: 4000, g: 0, q: 1, on: true },
    { type: 'highshelf', f: 10000, g: 0, q: 0.7, on: true },
  ],
}

const gains = (g: number[]): Band[] => FLAT.bands.map((b, i) => ({ ...b, g: g[i] }))
export const PRESETS: Record<string, Band[]> = {
  Flat: gains([0, 0, 0, 0, 0]),
  'Bass Boost': gains([6, 2, 0, 0, 0]),
  'Vocal Presence': gains([-2, -1, 2, 3, 0]),
  Loudness: gains([5, 0, -1, 1, 4]),
  'Treble Lift': gains([0, 0, 0, 2, 5]),
  'Bass Reducer': gains([-6, -2, 0, 0, 0]),
}

/** Equalizer APO / AutoEQ "ParametricEQ.txt": `Filter 1: ON PK Fc 105 Hz Gain -2.1 dB Q 0.70`. */
export function parseApo(text: string): Eq | null {
  const type: Record<string, BandType> = { PK: 'peak', PEQ: 'peak', LSC: 'lowshelf', LS: 'lowshelf', HSC: 'highshelf', HS: 'highshelf', HP: 'lowcut', LP: 'highcut' }
  const bands: Band[] = []
  for (const m of text.matchAll(/Filter\s*\d*:\s*(ON|OFF)\s+(\w+)\s+Fc\s+([\d.]+)\s*Hz(?:\s+Gain\s+([-\d.]+)\s*dB)?(?:\s+Q\s+([\d.]+))?/gi)) {
    const t = type[m[2].toUpperCase()]
    const band = { type: t, f: +m[3], g: +(m[4] ?? 0), q: +(m[5] ?? 0.707), on: m[1].toUpperCase() === 'ON' }
    // "1.2.3 Hz" reads as NaN, which mpv would refuse
    if (t && [band.f, band.g, band.q].every(Number.isFinite)) bands.push(band)
  }
  if (!bands.length) return null
  const pre = text.match(/Preamp:\s*([-\d.]+)\s*dB/i)
  const preamp = pre ? +pre[1] : NaN
  return { enabled: true, autoPreamp: !Number.isFinite(preamp), preamp: Number.isFinite(preamp) ? preamp : 0, bands: bands.slice(0, MAX_BANDS) }
}
