// Translate on this PC (translate_local.rs, mt.worker.ts): Mozilla's language codes, which packs a translation
// needs, and which language lyrics are in, all worked out offline.
import { francAll } from 'franc-min'

export interface Pair { from: string; to: string }

let names: Intl.DisplayNames | null = null
/** "ja" → "Japanese", "zh_hant" → "Traditional Chinese". */
export function languageOf(code: string): string {
  names ??= new Intl.DisplayNames(['en'], { type: 'language' })
  try {
    return names.of(code.replace('_hant', '-Hant')) ?? code
  } catch {
    return code
  }
}

/** "Japanese → English". */
export const pairName = (p: Pair) => `${languageOf(p.from)} → ${languageOf(p.to)}`

/** Settings' Google codes → Mozilla's: "zh-CN" → "zh", "zh-TW" → "zh_hant", "no" → "nb". */
export function mozCode(google: string): string {
  const g = google.toLowerCase()
  if (g === 'zh-cn' || g === 'zh') return 'zh'
  if (g === 'zh-tw' || g === 'zh_hant') return 'zh_hant'
  if (g === 'no') return 'nb'
  return g.split('-')[0]
}

/** The packs a translation needs: one, or two through English. [] when nothing needs translating, null when no way. */
export function route(from: string, to: string, offered: Pair[]): Pair[] | null {
  if (from === to) return []
  const has = (f: string, t: string) => offered.some((p) => p.from === f && p.to === t)
  if (has(from, to)) return [{ from, to }]
  if (from !== 'en' && to !== 'en' && has(from, 'en') && has('en', to)) return [{ from, to: 'en' }, { from: 'en', to }]
  return null
}

/** franc's ISO 639-3 codes for the languages Mozilla translates. */
const ISO3: Record<string, string> = {
  eng: 'en', ind: 'id', zlm: 'ms', spa: 'es', fra: 'fr', deu: 'de', ita: 'it', por: 'pt', nld: 'nl', tur: 'tr', pol: 'pl',
  ron: 'ro', ces: 'cs', hun: 'hu', swe: 'sv', vie: 'vi', tgl: 'tl', azj: 'az', bos: 'bs', hrv: 'hr', srp: 'sr', rus: 'ru',
  ukr: 'uk', bel: 'be', bul: 'bg', arb: 'ar', pes: 'fa', urd: 'ur', hin: 'hi', mar: 'mr',
}

/** Scripts that belong to one language Mozilla translates, with the share of letters that makes a text that language. */
const SCRIPTS: [RegExp, string, number][] = [
  [/[\p{sc=Hiragana}\p{sc=Katakana}]/gu, 'ja', 0.05],
  [/\p{sc=Hangul}/gu, 'ko', 0.2],
  [/\p{sc=Han}/gu, 'zh', 0.2],
  [/\p{sc=Thai}/gu, 'th', 0.2],
  [/\p{sc=Greek}/gu, 'el', 0.3],
  [/\p{sc=Hebrew}/gu, 'he', 0.3],
  [/\p{sc=Bengali}/gu, 'bn', 0.3],
  [/\p{sc=Tamil}/gu, 'ta', 0.3],
  [/\p{sc=Telugu}/gu, 'te', 0.3],
  [/\p{sc=Kannada}/gu, 'kn', 0.3],
  [/\p{sc=Malayalam}/gu, 'ml', 0.3],
  [/\p{sc=Gujarati}/gu, 'gu', 0.3],
]

/**
 * The language of a song's lyrics, in Mozilla's codes: by script first (kana before Han, so Japanese with kanji stays
 * Japanese), else by letter patterns over all the lines at once (one short line is too little to go on). Null when
 * it can't tell.
 */
export function detect(lines: string[]): string | null {
  const text = lines.join('\n')
  const letters = (text.match(/\p{L}/gu) ?? []).length
  if (!letters) return null
  for (const [re, code, share] of SCRIPTS) if ((text.match(re) ?? []).length / letters >= share) return code
  const [best] = francAll(text, { only: Object.keys(ISO3), minLength: 10 })
  return best && best[0] !== 'und' ? ISO3[best[0]] ?? null : null
}
