// Lyrics parsing: LRC, enhanced LRC (word timing) and TTML (Apple Music's format).

export interface Word { t: number; end: number; text: string }
export interface Line { t: number; end: number; text: string; words?: Word[] }
export interface Lyrics { synced: boolean; lines: Line[] }

const LINE_TAG = /\[(\d{1,3}):(\d{1,2}(?:[.:]\d{1,3})?)\]/g
const WORD_TAG = /<(\d{1,3}):(\d{1,2}(?:[.:]\d{1,3})?)>/g

const mmss = (m: string, s: string) => Number(m) * 60 + Number(s.replace(':', '.'))

export function parseLrc(text: string): Lyrics {
  const offset = Number(text.match(/\[offset:\s*([+-]?\d+)\]/i)?.[1] ?? 0) / 1000
  const lines: Line[] = []
  for (const raw of text.split(/\r?\n/)) {
    const stamps = [...raw.matchAll(LINE_TAG)]
    if (!stamps.length) continue
    const body = raw.replace(LINE_TAG, '').trim()
    const words = parseWords(body, offset)
    for (const st of stamps) {
      const t = Math.max(0, mmss(st[1], st[2]) - offset)
      lines.push({ t, end: t, text: body.replace(WORD_TAG, '').replace(/\s+/g, ' ').trim(), ...(words && { words }) })
    }
  }
  if (!lines.length) {
    const plain = text.split(/\r?\n/).map((l) => l.trim()).filter((l) => l && !/^\[[a-z]+:.*\]$/i.test(l))
    return { synced: false, lines: plain.map((text) => ({ t: 0, end: 0, text })) }
  }
  lines.sort((a, b) => a.t - b.t)
  return { synced: true, lines: closeLines(lines) }
}

// "<00:01.00>Hello <00:01.50>world <00:02.10>" → timed words; the last tag ends the line
function parseWords(body: string, offset: number): Word[] | undefined {
  const tags = [...body.matchAll(WORD_TAG)]
  if (tags.length < 2) return undefined
  const words: Word[] = []
  for (let i = 0; i < tags.length; i++) {
    const start = tags[i].index! + tags[i][0].length
    const text = body.slice(start, tags[i + 1]?.index ?? body.length)
    if (!text.trim()) continue
    const t = mmss(tags[i][1], tags[i][2]) - offset
    const end = tags[i + 1] ? mmss(tags[i + 1][1], tags[i + 1][2]) - offset : t + 0.5
    words.push({ t, end, text })
  }
  return words.length ? words : undefined
}

/** Each line ends where the next begins (or at its last word), so gaps can be detected. */
function closeLines(lines: Line[]): Line[] {
  return lines.map((l, i) => {
    const next = lines[i + 1]?.t ?? l.t + 6
    const lastWord = l.words?.[l.words.length - 1]?.end
    return { ...l, end: Math.min(next, lastWord ?? next) }
  })
}

// TTML clock times: "1:02.5", "00:01:02.500", "62.5s", "62500ms"
export function ttmlTime(v: string | undefined): number {
  if (!v) return 0
  if (v.endsWith('ms')) return Number(v.slice(0, -2)) / 1000
  if (v.endsWith('s')) return Number(v.slice(0, -1))
  return v.split(':').reduce((acc, p) => acc * 60 + Number(p), 0)
}

const decode = (s: string) =>
  s.replace(/&lt;/g, '<').replace(/&gt;/g, '>').replace(/&quot;/g, '"').replace(/&apos;|&#39;/g, "'").replace(/&amp;/g, '&')
const attr = (tag: string, name: string) => tag.match(new RegExp(`\\b${name}="([^"]*)"`))?.[1]

export function parseTtml(xml: string): Lyrics {
  const lines: Line[] = []
  for (const p of xml.matchAll(/<p\b([^>]*)>([\s\S]*?)<\/p>/g)) {
    const [, pAttrs, inner] = p
    const words: Word[] = []
    for (const sp of inner.matchAll(/<span\b([^>]*)>([\s\S]*?)<\/span>(\s?)/g)) {
      const b = attr(sp[1], 'begin')
      // background vocals and translations nest their own spans; keep only timed leaf words
      if (!b || /<span/.test(sp[2]) || /ttm:role="x-(bg|translation)"/.test(sp[1])) continue
      words.push({ t: ttmlTime(b), end: ttmlTime(attr(sp[1], 'end')), text: decode(sp[2].replace(/<[^>]+>/g, '')) + sp[3] })
    }
    const text = decode(inner.replace(/<span[^>]*ttm:role="x-(?:bg|translation)"[\s\S]*?<\/span>/g, '').replace(/<[^>]+>/g, '')).replace(/\s+/g, ' ').trim()
    if (!text) continue
    lines.push({ t: ttmlTime(attr(pAttrs, 'begin')), end: ttmlTime(attr(pAttrs, 'end')), text, ...(words.length > 1 && { words }) })
  }
  const synced = lines.some((l) => l.t > 0)
  return { synced, lines: synced ? lines.sort((a, b) => a.t - b.t) : lines }
}

export function parseLyrics(kind: string, text: string): Lyrics {
  return kind === 'ttml' || /^\s*<\?xml|<tt[\s>]/.test(text) ? parseTtml(text) : parseLrc(text)
}

/** Index of the line being sung at `time`, or -1 before the first line. */
export function activeLine(lines: Line[], time: number): number {
  let lo = 0, hi = lines.length - 1, ans = -1
  while (lo <= hi) {
    const mid = (lo + hi) >> 1
    if (lines[mid].t <= time) (ans = mid), (lo = mid + 1)
    else hi = mid - 1
  }
  return ans
}

/** A pause long enough for Apple-style "• • •" between lines. */
export const isGap = (a: Line | undefined, b: Line | undefined, min = 5) => !!b && b.t - (a ? a.end : 0) >= min
