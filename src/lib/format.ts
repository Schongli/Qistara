export interface Quality {
  badge: 'Hi-Res Lossless' | 'Lossless' | 'DSD' | null
  detail: string // e.g. "24-bit/96 kHz FLAC" or "320 kbps MP3"
}

type QualityInput = { codec: string; lossless: boolean; sampleRate?: number | null; bitDepth?: number | null; bitrate?: number | null }

// Apple's tiers: Lossless up to 24-bit/48 kHz, Hi-Res Lossless above 48 kHz.
export function quality(t: QualityInput): Quality {
  const rate = t.sampleRate ? `${+(t.sampleRate / 1000).toFixed(1)} kHz` : ''
  if (t.codec === 'DSF' || t.codec === 'DFF') return { badge: 'DSD', detail: `DSD ${t.codec}` }
  if (t.lossless) {
    const bits = t.bitDepth ? `${t.bitDepth}-bit` : ''
    const spec = [bits, rate].filter(Boolean).join('/')
    return { badge: (t.sampleRate ?? 0) > 48000 ? 'Hi-Res Lossless' : 'Lossless', detail: [spec, t.codec].filter(Boolean).join(' ') }
  }
  return { badge: null, detail: [t.bitrate ? `${t.bitrate} kbps` : '', t.codec].filter(Boolean).join(' ') }
}

export function time(s: number | null | undefined): string {
  if (!s || !isFinite(s) || s < 0) return '0:00'
  const h = Math.floor(s / 3600), m = Math.floor((s % 3600) / 60), sec = Math.floor(s % 60)
  return h ? `${h}:${String(m).padStart(2, '0')}:${String(sec).padStart(2, '0')}` : `${m}:${String(sec).padStart(2, '0')}`
}

export function totalTime(s: number): string {
  const h = Math.floor(s / 3600), m = Math.round((s % 3600) / 60)
  return h ? `${h} hr ${m} min` : `${m} min`
}

export const plural = (n: number, word: string) => `${n.toLocaleString()} ${word}${n === 1 ? '' : 's'}`

export const collator = new Intl.Collator(undefined, { sensitivity: 'base', numeric: true })
// "The Beatles" sorts under B, as in every music library.
export const sortKey = (s: string) => s.replace(/^(the|a|an)\s+/i, '')
export const byName = (a: string, b: string) => collator.compare(sortKey(a), sortKey(b))

/** A size for people: "820 KB", "12 MB", "1.7 GB". */
export function mb(bytes: number): string {
  if (bytes < 1e6) return `${Math.max(0, Math.round(bytes / 1e3))} KB`
  if (bytes < 1e9) return `${Math.round(bytes / 1e6)} MB`
  return `${(bytes / 1e9).toFixed(1)} GB`
}
