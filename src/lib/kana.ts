// Kana helpers for learning Japanese lyrics: hiragana, Hepburn romaji and furigana placement.

const BASE: Record<string, string> = {
  あ: 'a', い: 'i', う: 'u', え: 'e', お: 'o',
  か: 'ka', き: 'ki', く: 'ku', け: 'ke', こ: 'ko', が: 'ga', ぎ: 'gi', ぐ: 'gu', げ: 'ge', ご: 'go',
  さ: 'sa', し: 'shi', す: 'su', せ: 'se', そ: 'so', ざ: 'za', じ: 'ji', ず: 'zu', ぜ: 'ze', ぞ: 'zo',
  た: 'ta', ち: 'chi', つ: 'tsu', て: 'te', と: 'to', だ: 'da', ぢ: 'ji', づ: 'zu', で: 'de', ど: 'do',
  な: 'na', に: 'ni', ぬ: 'nu', ね: 'ne', の: 'no',
  は: 'ha', ひ: 'hi', ふ: 'fu', へ: 'he', ほ: 'ho', ば: 'ba', び: 'bi', ぶ: 'bu', べ: 'be', ぼ: 'bo',
  ぱ: 'pa', ぴ: 'pi', ぷ: 'pu', ぺ: 'pe', ぽ: 'po',
  ま: 'ma', み: 'mi', む: 'mu', め: 'me', も: 'mo', や: 'ya', ゆ: 'yu', よ: 'yo',
  ら: 'ra', り: 'ri', る: 'ru', れ: 're', ろ: 'ro', わ: 'wa', ゐ: 'i', ゑ: 'e', を: 'o', ゔ: 'vu',
}
const YOON: Record<string, string> = { ゃ: 'a', ゅ: 'u', ょ: 'o' }
const SMALL: Record<string, string> = { ぁ: 'a', ぃ: 'i', ぅ: 'u', ぇ: 'e', ぉ: 'o', ゎ: 'wa' }

/** Katakana → hiragana; everything else (ー, kanji, Latin) stays. */
export const toHiragana = (s: string) => s.replace(/[ァ-ヶ]/g, (c) => String.fromCharCode(c.charCodeAt(0) - 0x60))

export const hasKanji = (s: string) => /[㐀-鿿豈-﫿々〆]/.test(s)

// a syllable's consonant part: ki → k, shi → sh, fu → f, u → w (for うぃ = wi)
const stem = (r: string) => (r === 'u' ? 'w' : r.replace(/[aiueo]$/, ''))

/** Hepburn romaji without macrons: じゅう → juu, こんや → kon'ya, きって → kitte. */
export function toRomaji(input: string): string {
  const s = [...toHiragana(input)]
  const syl: string[] = []
  for (let i = 0; i < s.length; i++) {
    const c = s[i]
    const next = s[i + 1]
    let r = BASE[c]
    if (r === undefined) {
      syl.push(YOON[c] ? 'y' + YOON[c] : (SMALL[c] ?? c)) // a lone small kana, っ, ん, ー or anything else
      continue
    }
    if (next in YOON && r.length > 1 && r.endsWith('i')) {
      const st = stem(r)
      r = /(sh|ch|j)$/.test(st) ? st + YOON[next] : st + 'y' + YOON[next]
      i++
    } else if (next in SMALL && next !== 'ゎ') {
      r = stem(r) + SMALL[next]
      i++
    }
    syl.push(r)
  }
  let out = ''
  syl.forEach((r, k) => {
    const next = syl[k + 1] ?? ''
    if (r === 'っ') out += /^[b-df-hj-np-tv-z]/.test(next) ? (next.startsWith('ch') ? 't' : next[0]) : ''
    else if (r === 'ん') out += /^[aiueoy]/.test(next) ? "n'" : 'n'
    else if (r === 'ー') out += /[aiueo]$/.exec(out)?.[0] ?? '-'
    else out += r
  })
  return out
}

export interface Ruby { text: string; rt?: string }

/**
 * Splits a word so readings sit over its kanji only: 引き出し + ひきだし → 引(ひ) き 出(だ) し.
 * `reading` is hiragana; a word without kanji (or without a reading) comes back whole.
 */
export function furigana(surface: string, reading: string): Ruby[] {
  if (!reading || !hasKanji(surface)) return [{ text: surface }]
  const runs = surface.match(/[㐀-鿿豈-﫿々〆]+|[^㐀-鿿豈-﫿々〆]+/g) ?? [surface]
  const escape = (k: string) => toHiragana(k).replace(/[.*+?^${}()|[\]\\]/g, '\\$&')
  const re = new RegExp(`^${runs.map((r) => (hasKanji(r) ? '(.+?)' : escape(r))).join('')}$`)
  const m = re.exec(toHiragana(reading))
  if (!m) return [{ text: surface, rt: reading }]
  let g = 1
  return runs.map((r) => (hasKanji(r) ? { text: r, rt: m[g++] } : { text: r }))
}
