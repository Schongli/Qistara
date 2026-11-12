// One-word dictionary lookups through Google Translate's free endpoint (dt=bd: meanings by part of speech).

export interface Lookup {
  translation: string
  romaji: string // the source word's romanization, '' when Google gives none
  source: string
  senses: { pos: string; terms: string[] }[]
}

/** Reply shape: [[[translated, original, …], …, [null, null, null, romanization]], [[pos, [terms], …], …] | null, source, …]. */
export function parseLookup(j: unknown): Lookup {
  if (!Array.isArray(j) || !Array.isArray(j[0])) throw new Error('unexpected reply from Google Translate')
  const segs = j[0] as unknown[][]
  const translation = segs.map((s) => (typeof s?.[0] === 'string' ? s[0] : '')).join('').trim()
  const romaji = segs.map((s) => (s?.[0] == null && typeof s?.[3] === 'string' ? s[3] : '')).join('').trim()
  const senses = (Array.isArray(j[1]) ? (j[1] as unknown[][]) : [])
    .filter((e) => typeof e?.[0] === 'string' && Array.isArray(e[1]))
    .map((e) => ({ pos: e[0] as string, terms: (e[1] as unknown[]).filter((t): t is string => typeof t === 'string') }))
  return { translation, romaji, source: typeof j[2] === 'string' ? j[2] : '', senses }
}
