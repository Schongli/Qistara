// Saved words and their review schedule (Leitner boxes: each correct answer waits longer).

export interface VocabItem {
  id: string
  word: string // dictionary form
  reading: string // hiragana for Japanese, '' otherwise
  romaji: string
  pos: string
  meanings: string[]
  lang: string
  // the lyric it came from, so review can replay it
  line: string
  translation: string
  path: string
  title: string
  artist: string
  t: number
  added: number
  box: number
  due: number
}

export const DAY = 86_400_000
/** Days until the next review for each box. */
export const INTERVALS = [0, 1, 3, 7, 14, 30]

export const vocabId = (lang: string, word: string) => `${lang}:${word}`

export function review(item: VocabItem, knew: boolean, now = Date.now()): VocabItem {
  const box = knew ? Math.min(item.box + 1, INTERVALS.length - 1) : 0
  return { ...item, box, due: now + INTERVALS[box] * DAY }
}

export const dueItems = (items: VocabItem[], now = Date.now()) => items.filter((i) => i.due <= now).sort((a, b) => a.due - b.due)
