// Learn mode for lyrics: each line split into words you can tap, furigana and romaji for Japanese,
// a dictionary card per word, and saved words with spaced review (vocab.ts).
// Japanese is analysed offline (japanese.rs); word meanings come from Google Translate, one word at a time.
import { invoke } from '@tauri-apps/api/core'
import { untrack } from 'svelte'
import { lyr } from './lyrics.svelte'
import { mpv, seek, set } from './player.svelte'
import { queue } from './queue.svelte'
import { settings } from './settings.svelte'
import { targetLanguage, tr } from './translate.svelte'
import { fileTrack } from './library.svelte'
import { toast } from './nav.svelte'
import { kvGet, kvSet } from './persist'
import { toHiragana, toRomaji } from './kana'
import { gloss, isJapanese, lineRomaji, posName, withGaps, type JpWord, type Piece } from './japanese'
import { parseLookup, type Lookup } from './lookup'
import { review, vocabId, type VocabItem } from './vocab'
import type { Lyrics } from './lyrics'

/** pieces: each line split into words and gaps. romaji: one line each, Japanese only. */
export const learn = $state({ japanese: false, pieces: null as Piece[][] | null, romaji: null as string[] | null, error: '' })

let seq = 0
async function analyze(data: Lyrics | null, on: boolean) {
  const mine = ++seq
  Object.assign(learn, { japanese: false, pieces: null, romaji: null, error: '' })
  closeCard()
  if (!on || !data) return
  const lines = data.lines.map((l) => l.text)
  try {
    if (isJapanese(lines)) {
      const words = await invoke<JpWord[][]>('japanese_words', { lines })
      if (mine !== seq) return
      learn.japanese = true
      learn.pieces = lines.map((l, i) => withGaps(l, words[i] ?? []))
      learn.romaji = words.map(lineRomaji)
    } else {
      const seg = new Intl.Segmenter(undefined, { granularity: 'word' })
      learn.pieces = lines.map((l) => [...seg.segment(l)].map((s) => ({ text: s.segment, tap: !!s.isWordLike })))
    }
  } catch (e) {
    if (mine === seq) learn.error = String(e)
  }
}

$effect.root(() => {
  $effect(() => {
    const data = lyr.data
    const on = settings.learnLyrics
    untrack(() => analyze(data, on))
  })
})

// ---- the word card ----

export const card = $state({
  open: false,
  line: -1,
  piece: -1,
  text: '', // as sung
  sung: '', // hiragana of the sung form (Japanese), for furigana on the headline
  word: '', // dictionary form
  reading: '', // hiragana of the dictionary form (Japanese)
  romaji: '',
  pos: '',
  note: '', // grammar note for particles and verb endings; they get no dictionary lookup
  lookup: null as Lookup | null,
  busy: false,
  error: '',
})
let cardSeq = 0

export function closeCard() {
  cardSeq++
  card.open = false
}

const sourceLanguage = () => (learn.japanese ? 'ja' : tr.source || 'auto')
export const cardId = () => vocabId(sourceLanguage(), card.word)

/**
 * One word's meanings; cached, so a word is looked up once per language pair. Only the word is sent.
 * The request goes out from Rust (translate.rs): Google refuses the HTTP plugin's HTTP/2 requests with 429.
 */
async function lookup(q: string, sl: string): Promise<Lookup> {
  const tl = targetLanguage()
  const key = `dict:${sl}:${tl}:${q}`
  const hit = await kvGet<Lookup>(key)
  if (hit) return hit
  const raw = await invoke<string>('google_translate', { text: q, from: sl, to: tl, dictionary: true })
  const l = parseLookup(JSON.parse(raw))
  kvSet(key, l)
  return l
}

export async function openWord(line: number, piece: number) {
  const p = learn.pieces?.[line]?.[piece]
  if (!p?.tap) return
  const mine = ++cardSeq
  const jp = p.jp
  const word = jp ? jp.base : p.text.toLocaleLowerCase()
  Object.assign(card, { open: true, line, piece, text: p.text, sung: jp?.reading ? toHiragana(jp.reading) : '', word, reading: '', romaji: '', pos: jp ? posName(jp) : '', note: jp ? gloss(jp) : '', lookup: null, busy: true, error: '' })
  try {
    if (jp?.reading) {
      // read the dictionary form, not the sung form: 呼ん is よん, 呼ぶ is よぶ
      const r = word === p.text ? jp.reading : ((await invoke<JpWord[][]>('japanese_words', { lines: [word] }))[0] ?? []).map((w) => w.reading).join('')
      if (mine !== cardSeq) return
      card.reading = toHiragana(r)
      card.romaji = toRomaji(r)
    }
    if (!card.note) {
      const l = await lookup(word, sourceLanguage())
      if (mine !== cardSeq) return
      card.lookup = l
      if (!card.romaji && l.romaji && l.romaji.toLowerCase() !== word) card.romaji = l.romaji
    }
  } catch (e) {
    if (mine === cardSeq) card.error = e instanceof Error ? e.message : String(e)
  } finally {
    if (mine === cardSeq) card.busy = false
  }
}

/** Up to eight meanings, across parts of speech; the plain translation when there's no dictionary entry. */
export const meanings = (l: Lookup | null): string[] =>
  !l ? [] : l.senses.length ? l.senses.flatMap((s) => s.terms.slice(0, 4)).slice(0, 8) : l.translation ? [l.translation] : []

// ---- saved words ----

export const vocab = $state({ items: [] as VocabItem[], loaded: false })
// nothing is saved until the stored list is in, so it is never overwritten by an empty one
kvGet<VocabItem[]>('vocab').then((v) => {
  vocab.items = v ?? []
  vocab.loaded = true
})
const saveVocab = () => vocab.loaded && kvSet('vocab', $state.snapshot(vocab.items))

export const isSaved = (id: string) => vocab.items.some((i) => i.id === id)

export function toggleSaveCard() {
  if (!vocab.loaded) return
  const id = cardId()
  if (isSaved(id)) {
    vocab.items = vocab.items.filter((i) => i.id !== id)
  } else {
    const l = lyr.data?.lines[card.line]
    const t = queue.current
    const now = Date.now()
    vocab.items = [
      {
        id, word: card.word, reading: card.reading, romaji: card.romaji, pos: card.pos,
        meanings: card.note ? [card.note] : meanings(card.lookup), lang: sourceLanguage(),
        line: l?.text ?? '', translation: tr.lines?.[card.line] ?? '', path: t?.path ?? '', title: t?.title ?? '', artist: t?.artist ?? '',
        t: lyr.data?.synced ? (l?.t ?? 0) : 0, added: now, box: 0, due: now,
      },
      ...vocab.items,
    ]
  }
  saveVocab()
}

export function removeWord(id: string) {
  vocab.items = vocab.items.filter((i) => i.id !== id)
  saveVocab()
}

export function grade(id: string, knew: boolean) {
  vocab.items = vocab.items.map((i) => (i.id === id ? review(i, knew) : i))
  saveVocab()
}

/** Plays a saved word's lyric line: in the current song, or by starting that song at the line. */
export async function playLine(path: string, t: number) {
  if (!path) return
  if (queue.current?.path !== path) {
    const track = await fileTrack(path)
    if (!track) return toast('That song is no longer in your library.')
    await queue.play([track])
    // seeking before mpv has opened the file does nothing, so wait for it (up to 5 s)
    for (let i = 0; i < 50 && !(mpv.path === path && mpv.duration > 0); i++) await new Promise((r) => setTimeout(r, 100))
  }
  await seek(t)
  if (mpv.pause) await set('pause', false)
}
