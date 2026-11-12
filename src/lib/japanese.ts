// Japanese lyric words (from japanese.rs, IPADIC) turned into things a learner can read:
// English parts of speech, short notes for grammar words, and romaji for a whole line.
import { toRomaji } from './kana'

/** One word: IPADIC tags (pos, pos1), conjugation form, dictionary form, katakana reading ('' = not in the dictionary). */
export interface JpWord { text: string; pos: string; pos1: string; form: string; base: string; reading: string }

export const isJapanese = (lines: string[]) => lines.some((l) => /[぀-ヿ]/.test(l))

const POS: Record<string, string> = {
  名詞: 'noun', 動詞: 'verb', 形容詞: 'i-adjective', 副詞: 'adverb', 助詞: 'particle', 助動詞: 'auxiliary',
  接続詞: 'conjunction', 連体詞: 'pre-noun adjective', 感動詞: 'interjection', 接頭詞: 'prefix', 記号: 'symbol', フィラー: 'filler',
}

export function posName(w: JpWord): string {
  if (!w.reading && !/[぀-ヿ㐀-鿿]/.test(w.text)) return 'foreign word'
  if (w.pos1 === '接尾') return 'suffix'
  if (w.pos === '動詞' && w.pos1 === '非自立') return 'helper verb'
  if (w.pos === '名詞') return { 代名詞: 'pronoun', 固有名詞: 'proper noun', 数: 'number', 形容動詞語幹: 'na-adjective' }[w.pos1] ?? 'noun'
  return POS[w.pos] ?? 'word'
}

const PARTICLES: Record<string, string> = {
  は: 'topic marker: "as for …"', が: 'subject marker', を: 'object marker: what the verb acts on', に: 'to / at / in (target, time, place)',
  へ: 'toward', で: 'at / by / with (where or how something happens)', と: 'and / with / quotes what was said', の: "links nouns: possessive 's",
  も: 'also / too / even', から: 'from / because', まで: 'until / as far as', より: 'than / from', や: 'and (among others)',
  か: 'question marker / or', よ: 'emphasis: "I tell you"', ね: 'seeks agreement: "right?"', な: 'emphasis, or "don\'t …" after a verb',
  けど: 'but / although', けれど: 'but / although', のに: 'even though', ので: 'because / so', ば: 'conditional: "if …"',
  ても: 'even if', でも: 'even / but', たり: 'doing things like …', ながら: 'while …ing', し: "and what's more",
  って: 'casual quote or topic: "(they) say"', さ: 'casual emphasis', ぞ: 'strong emphasis', わ: 'soft emphasis', かな: 'I wonder',
  だけ: 'only / just', しか: 'only (with a negative verb)', ばかり: 'nothing but', など: 'and so on', こそ: 'precisely / indeed',
}
const TE = 'te-form: links to the next verb ("and then", "…ing")'

const AUXILIARIES: Record<string, string> = {
  た: 'past tense', だ: 'plain "is / am / are"', です: 'polite "is / am / are"', ます: 'polite verb ending', ない: 'negative: "not"',
  ぬ: 'negative: "not" (old style)', ん: 'negative: "not" (casual)', たい: 'want to …', う: 'volitional: "let\'s … / I will …"',
  よう: 'volitional: "let\'s … / I will …"', まい: '"will not" / "let\'s not"', れる: 'passive or "can …"', られる: 'passive or "can …"',
  せる: 'causative: "make / let …"', させる: 'causative: "make / let …"', らしい: 'seems / apparently', そうだ: 'looks like / I hear',
}

const HELPERS: Record<string, string> = {
  いる: 'after te-form: "…ing" or an ongoing state (te iru)', ある: 'after te-form: a state left by an action (te aru)',
  しまう: 'after te-form: completely, or regrettably (te shimau)', ちゃう: 'casual "te shimau": completely / regrettably',
  いく: 'after te-form: goes on …ing (te iku)', くる: 'after te-form: has been / comes to (te kuru)', みる: 'after te-form: try …ing (te miru)',
  おく: 'after te-form: do in advance (te oku)', くれる: 'after te-form: do for me (te kureru)', あげる: 'after te-form: do for someone (te ageru)',
  もらう: 'after te-form: have someone do (te morau)',
}

/** A short note for grammar words (particles, verb endings); '' for content words, which get a dictionary lookup. */
export function gloss(w: JpWord): string {
  if (w.pos === '助詞') return w.pos1 === '接続助詞' && (w.base === 'て' || w.base === 'で') ? TE : (PARTICLES[w.base] ?? '')
  if (w.pos === '助動詞') return w.base === 'た' && w.form === '仮定形' ? 'conditional: "if / when …" (tara)' : (AUXILIARIES[w.base] ?? '')
  if (w.pos === '動詞' && w.pos1 === '非自立') return HELPERS[w.base] ?? ''
  return ''
}

const PUNCT: Record<string, string> = { '、': ',', '。': '.', '！': '!', '？': '?', '「': '"', '」': '"', '『': '"', '』': '"', '・': ' ', '〜': '~', '…': '…' }

/** Romaji for one word; particles as pronounced (は → wa, へ → e, を → o). */
export function romajiOf(w: JpWord): string {
  if (w.pos === '記号') return [...w.text].map((c) => PUNCT[c] ?? c).join('')
  if (!w.reading) return w.text
  if (w.pos === '助詞') {
    if (w.text === 'は') return 'wa'
    if (w.text === 'へ') return 'e'
  }
  return toRomaji(w.reading)
}

// verb endings and connectors join the word before them: 呼ん + で → yonde, た → ita
const joins = (w: JpWord) => w.pos === '助動詞' || w.pos === '記号' || w.pos1 === '接尾' || (w.pos === '助詞' && w.pos1 === '接続助詞')

export const lineRomaji = (words: JpWord[]) => words.map((w, i) => (i && !joins(w) ? ' ' : '') + romajiOf(w)).join('')

/** A piece of a lyric line in Learn mode: words can be tapped, the gaps between them (spaces, punctuation) can't. */
export interface Piece { text: string; tap: boolean; jp?: JpWord }

/** Puts back what the analyzer dropped between words (spaces), so the line reads exactly as written. */
export function withGaps(line: string, words: JpWord[]): Piece[] {
  const out: Piece[] = []
  let at = 0
  for (const w of words) {
    const i = line.indexOf(w.text, at)
    if (i > at) out.push({ text: line.slice(at, i), tap: false })
    out.push({ text: w.text, tap: w.pos !== '記号', jp: w })
    if (i >= 0) at = i + w.text.length
  }
  if (at < line.length) out.push({ text: line.slice(at), tap: false })
  return out
}
