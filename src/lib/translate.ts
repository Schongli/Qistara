// Lyrics translation helpers for Google Translate's free endpoint (translate_a/single, client=gtx).

/** Languages offered in Settings, as [Google code, English name]. */
export const LANGUAGES: [string, string][] = [
  ['ar', 'Arabic'], ['bn', 'Bengali'], ['zh-CN', 'Chinese (Simplified)'], ['zh-TW', 'Chinese (Traditional)'],
  ['cs', 'Czech'], ['da', 'Danish'], ['nl', 'Dutch'], ['en', 'English'], ['tl', 'Filipino'], ['fi', 'Finnish'],
  ['fr', 'French'], ['de', 'German'], ['el', 'Greek'], ['he', 'Hebrew'], ['hi', 'Hindi'], ['hu', 'Hungarian'],
  ['id', 'Indonesian'], ['it', 'Italian'], ['ja', 'Japanese'], ['jw', 'Javanese'], ['ko', 'Korean'], ['ms', 'Malay'],
  ['no', 'Norwegian'], ['fa', 'Persian'], ['pl', 'Polish'], ['pt', 'Portuguese'], ['ro', 'Romanian'], ['ru', 'Russian'],
  ['es', 'Spanish'], ['su', 'Sundanese'], ['sv', 'Swedish'], ['ta', 'Tamil'], ['th', 'Thai'], ['tr', 'Turkish'],
  ['uk', 'Ukrainian'], ['ur', 'Urdu'], ['vi', 'Vietnamese'],
]

export const languageName = (code: string) => LANGUAGES.find(([c]) => c === code)?.[1] ?? code

/** Target for a UI locale: Chinese keeps its script, everything else uses the base code ("id-ID" → "id"). */
export function targetFor(locale: string): string {
  const l = locale.toLowerCase()
  if (l.startsWith('zh')) return /tw|hk|mo|hant/.test(l) ? 'zh-TW' : 'zh-CN'
  return l.split('-')[0]
}

/** Regional variants count as the same language; Simplified and Traditional Chinese don't. */
export const sameLanguage = (a: string, b: string) =>
  a.toLowerCase() === b.toLowerCase() || (!a.startsWith('zh') && a.split('-')[0] === b.split('-')[0])

/** Distinct non-empty lines in order, so a repeated chorus is sent once. */
export const uniqueLines = (lines: string[]) => [...new Set(lines.map((l) => l.trim()).filter(Boolean))]

/** Groups lines into newline-joined requests of at most `max` characters; an over-long line goes alone. */
export function batches(texts: string[], max = 4000): string[][] {
  const out: string[][] = []
  let size = 0
  for (const t of texts) {
    const last = out[out.length - 1]
    if (last && size + 1 + t.length <= max) {
      last.push(t)
      size += 1 + t.length
    } else {
      out.push([t])
      size = t.length
    }
  }
  return out
}

/** Reply shape: [[[translated, original, …], …], null, detectedLanguage, …]. */
export function parseGtx(j: unknown): { text: string; source: string } {
  if (!Array.isArray(j) || !Array.isArray(j[0])) throw new Error('unexpected reply from Google Translate')
  const text = (j[0] as unknown[]).map((s) => (Array.isArray(s) && typeof s[0] === 'string' ? s[0] : '')).join('')
  return { text, source: typeof j[2] === 'string' ? j[2] : '' }
}

/**
 * Translations per sent line, rebuilt from the reply's pieces: each piece is [translation, original], and a line ends
 * where its original ends in a newline. This holds even when Google drops or adds newlines in the translation, so a
 * batch needs one request. Null when the pieces don't cover exactly the sent lines.
 */
export function alignSegments(sent: string[], j: unknown): string[] | null {
  if (!Array.isArray(j) || !Array.isArray(j[0])) return null
  const out: string[] = []
  let cur = ''
  for (const s of j[0] as unknown[]) {
    if (!Array.isArray(s) || typeof s[0] !== 'string' || typeof s[1] !== 'string') continue
    cur += s[0]
    const breaks = s[1].split('\n').length - 1
    if (breaks > 1) return null // one piece over several lines: its translation can't be split
    if (breaks === 1) {
      out.push(cur.trim())
      cur = ''
    }
  }
  if (cur.trim() || out.length === sent.length - 1) out.push(cur.trim())
  return out.length === sent.length ? out : null
}

/** Seconds to wait from Rust's busy answer ("google-busy:<seconds>"), or null for any other error. */
export function busyFor(e: unknown): number | null {
  const m = /^google-busy:(\d+)/.exec(String(e))
  return m ? Number(m[1]) : null
}

/** Translations for the sent lines, in order, or null when the reply has a different number of lines. */
export function align(sent: string[], reply: string): string[] | null {
  const out = reply.replace(/\n+$/, '').split('\n').map((s) => s.trim())
  return out.length === sent.length ? out : null
}

const fold = (s: string) => s.trim().toLowerCase()

/** One entry per lyric line; null where there is nothing to show (empty, unknown or unchanged). */
export const mapBack = (lines: string[], dict: Record<string, string>) =>
  lines.map((l) => {
    const t = dict[l.trim()]
    return t && fold(t) !== fold(l) ? t : null
  })

/** FNV-1a, 32-bit hex: a short cache key for a set of lyrics. */
export function hash(s: string): string {
  let h = 0x811c9dc5
  for (let i = 0; i < s.length; i++) h = Math.imul(h ^ s.charCodeAt(i), 0x01000193)
  return (h >>> 0).toString(16).padStart(8, '0')
}
