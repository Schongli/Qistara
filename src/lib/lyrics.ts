// Lyrics parsing: LRC, enhanced LRC (word timing) and TTML (Apple Music's format).

export interface Word { t: number; end: number; text: string }
export interface Line { t: number; end: number; text: string; words?: Word[] }
export interface Lyrics { synced: boolean; lines: Line[] }

// Stamps as players write them: [mm:ss], [mm:ss.x]–[mm:ss.xxx], [mm:ss:xx], [mm:ss,xx], [h:mm:ss.xx]
const STAMP = String.raw`\d{1,3}:\d{1,2}(?::\d{1,2})?(?:[.,:]\d{1,3})?`
const LINE_TAG = new RegExp(`\\[(${STAMP})\\]`, 'g')
const WORD_TAG = new RegExp(`<(${STAMP})>`, 'g')
/** A metadata tag such as [ti:…], [ar:…], [by:…], also with spaces or a full-width colon. */
export const META_TAG = /^\s*\[\s*[a-z#][\w#-]*\s*[:：][^\]]*\]\s*$/i
const LINES = /\r\n|\r|\n/

/** Seconds of one stamp; null when it isn't a real time. */
export function stampTime(v: string): number | null {
  let [main, frac = ''] = v.split(/[.,]/)
  const parts = main.split(':')
  // mm:ss:xx (centiseconds after a colon) and h:mm:ss:xx
  if (!frac && parts.length >= 3) frac = parts.pop()!
  if (parts.length > 3 || parts.some((p) => !p)) return null
  const t = parts.reduce((acc, p) => acc * 60 + Number(p), 0) + (frac ? Number(`0.${frac}`) : 0)
  return Number.isFinite(t) ? t : null
}

export function parseLrc(text: string): Lyrics {
  const offset = Number(text.match(/^\s*\[\s*offset\s*[:：]\s*([+-]?\d+)\s*\]/im)?.[1] ?? 0) / 1000
  const lines: Line[] = []
  for (const raw of text.split(LINES)) {
    const stamps = [...raw.matchAll(LINE_TAG)]
    if (!stamps.length) continue
    const body = raw.replace(LINE_TAG, '').trim()
    // [00:00.00][ar:Someone]: a tag behind a stamp is still a tag
    if (META_TAG.test(body)) continue
    const words = parseWords(body, offset)
    for (const st of stamps) {
      const s = stampTime(st[1])
      if (s === null) continue
      const t = Math.max(0, s - offset)
      lines.push({ t, end: t, text: body.replace(WORD_TAG, '').replace(/\s+/g, ' ').trim(), ...(words && { words }) })
    }
  }
  if (!lines.length) {
    const plain = text
      .split(LINES)
      // stamps nothing could read are noise, not lyrics
      .map((l) => l.replace(/^(\s*\[[\d:.,\s+-]*\])+/, '').trim())
      .filter((l) => l && !META_TAG.test(l))
    return { synced: false, lines: plain.map((text) => ({ t: 0, end: 0, text })) }
  }
  lines.sort((a, b) => a.t - b.t)
  return { synced: true, lines: closeLines(lines) }
}

// "<00:01.00>Hello <00:01.50>world <00:02.10>" → timed words; the last tag ends the line
function parseWords(body: string, offset: number): Word[] | undefined {
  const tags = [...body.matchAll(WORD_TAG)].map((m) => ({ m, t: stampTime(m[1]) })).filter((x) => x.t !== null)
  if (tags.length < 2) return undefined
  const words: Word[] = []
  for (let i = 0; i < tags.length; i++) {
    const { m } = tags[i]
    const start = m.index! + m[0].length
    const text = body.slice(start, tags[i + 1]?.m.index ?? body.length)
    if (!text.trim()) continue
    const t = Math.max(0, tags[i].t! - offset)
    const end = tags[i + 1] ? Math.max(t, tags[i + 1].t! - offset) : t + 0.5
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

/**
 * TTML times: clock "1:02.5", "00:01:02.500", "00:01:02:12" (frames), or offsets "62.5s", "62500ms", "1.5m",
 * "0.1h", "1500f", "10000000t". NaN when it can't be read.
 */
export function ttmlTime(v: string | undefined, rate = { tick: 1, frame: 30 }): number {
  if (!v) return 0
  v = v.trim()
  const off = v.match(/^(\d+(?:\.\d+)?)(h|ms|m|s|f|t)$/)
  if (off) {
    const n = Number(off[1])
    return { h: n * 3600, m: n * 60, s: n, ms: n / 1000, f: n / rate.frame, t: n / rate.tick }[off[2]]!
  }
  if (!/^\d+(?::\d+){0,3}(?:\.\d+)?$/.test(v)) return NaN
  const parts = v.split(':').map(Number)
  const frames = parts.length === 4 ? parts.pop()! / rate.frame : 0
  return parts.reduce((acc, p) => acc * 60 + p, 0) + frames
}

const ENTITIES: Record<string, string> = { lt: '<', gt: '>', quot: '"', apos: "'", nbsp: ' ', amp: '&' }
const decode = (s: string) =>
  s.replace(/&(?:#(\d{1,7})|#x([0-9a-f]{1,6})|(lt|gt|quot|apos|nbsp|amp));/gi, (m, dec, hex, name) => {
    if (name) return ENTITIES[name.toLowerCase()]
    const cp = dec ? Number(dec) : parseInt(hex, 16)
    return cp > 0 && cp <= 0x10ffff ? String.fromCodePoint(cp) : m
  })
const attr = (tag: string, name: string) => {
  const m = tag.match(new RegExp(`(?:^|\\s)${name}\\s*=\\s*(?:"([^"]*)"|'([^']*)')`))
  return m ? (m[1] ?? m[2]) : undefined
}
const ROLE = /ttm:role\s*=\s*["']x-(?:bg|translation|roman)["']/

/** Removes background-vocal and translation spans, with everything nested in them. */
function stripRoles(inner: string): string {
  let out = ''
  let i = 0
  const open = /<span\b[^>]*>/g
  for (let m; (m = open.exec(inner)); ) {
    if (!ROLE.test(m[0])) continue
    out += inner.slice(i, m.index)
    // walk to the matching </span>
    const tags = /<span\b[^>]*>|<\/span\s*>/g
    tags.lastIndex = m.index + m[0].length
    let depth = 1, t
    while (depth && (t = tags.exec(inner))) depth += t[0][1] === '/' ? -1 : 1
    i = t ? tags.lastIndex : inner.length
    open.lastIndex = i
  }
  return out + inner.slice(i)
}

export const MAX_TTML = 2_000_000

export function parseTtml(xml: string): Lyrics {
  if (xml.length > MAX_TTML) return { synced: false, lines: [] }
  // tt:p, tt:span… are the same elements
  xml = xml.replace(/<(\/?)[\w-]+:(tt|body|div|p|span|br)\b/g, '<$1$2')
  const rate = {
    tick: Number(xml.match(/ttp:tickRate\s*=\s*["'](\d+)["']/)?.[1]) || 1,
    frame: Number(xml.match(/ttp:frameRate\s*=\s*["'](\d+)["']/)?.[1]) || 30,
  }
  const time = (v: string | undefined) => {
    if (v === undefined) return undefined
    const t = ttmlTime(v, rate)
    return Number.isFinite(t) ? t : undefined
  }
  const lines: Line[] = []
  for (const p of xml.matchAll(/<p\b([^>]*)>([\s\S]*?)<\/p\s*>/g)) {
    const pAttrs = p[1]
    const inner = stripRoles(p[2]).replace(/<br\s*\/?>/g, ' ')
    const words: Word[] = []
    // leaf spans only: a wrapper's own text is its children's
    for (const sp of inner.matchAll(/<span\b([^>]*)>((?:(?!<span\b)[\s\S])*?)<\/span\s*>(\s?)/g)) {
      const t = time(attr(sp[1], 'begin'))
      if (t === undefined) continue
      const end = time(attr(sp[1], 'end')) ?? (time(attr(sp[1], 'dur')) ?? NaN) + t
      words.push({ t, end, text: decode(sp[2].replace(/<[^>]+>/g, '')) + sp[3] })
    }
    words.forEach((w, i) => Number.isFinite(w.end) || (w.end = words[i + 1]?.t ?? w.t + 0.5))
    const text = decode(inner.replace(/<[^>]+>/g, '')).replace(/\s+/g, ' ').trim()
    if (!text) continue
    const t = time(attr(pAttrs, 'begin')) ?? (words.length ? Math.min(...words.map((w) => w.t)) : 0)
    const end = time(attr(pAttrs, 'end')) ?? ((d) => (d === undefined ? undefined : t + d))(time(attr(pAttrs, 'dur'))) ?? words[words.length - 1]?.end
    lines.push({ t, end: end ?? NaN, text, ...(words.length > 1 && { words }) })
  }
  const synced = lines.some((l) => l.t > 0)
  if (!synced) return { synced, lines: lines.map((l) => ({ ...l, end: 0 })) }
  lines.sort((a, b) => a.t - b.t)
  // a line without an end runs to the next one, so it never reads as a gap
  lines.forEach((l, i) => Number.isFinite(l.end) || (l.end = lines[i + 1]?.t ?? l.t + 6))
  return { synced, lines }
}

export function parseLyrics(kind: string, text: string): Lyrics {
  return kind === 'ttml' || /^\s*<\?xml|<(?:[\w-]+:)?tt[\s>]/.test(text) ? parseTtml(text) : parseLrc(text)
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
