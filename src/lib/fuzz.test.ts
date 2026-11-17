// Fuzzing the parsers that read files and replies from outside (SECURITY.md): random mutations of real-looking
// inputs must never throw, and what comes out must stay sane. Seeded, so a failure reproduces.
import { describe, expect, it } from 'vitest'
import { META_TAG, parseLrc, parseTtml } from './lyrics'
import { buildM3u, parseM3u } from './m3u'
import { parseApo } from './eq'

function rng(seed: number) {
  let x = seed >>> 0 || 1
  return () => {
    x ^= x << 13
    x ^= x >>> 17
    x ^= x << 5
    return (x >>> 0) / 0x100000000
  }
}

const NASTY = ['[', ']', '<', '>', ':', '：', '.', ',', '\r', '\n', '\u0000', '\uFEFF', '&#', ';', '"', "'", '/', '\\', '%', '../', 'file://', '9', '-', '\u202E', '𝄞', 'ti:', '<span', '</span>', 'begin="', 'x-bg']

function mutate(seed: string, r: () => number): string {
  let s = seed
  const n = 1 + Math.floor(r() * 8)
  for (let i = 0; i < n; i++) {
    const at = Math.floor(r() * (s.length + 1))
    const op = r()
    if (op < 0.4) s = s.slice(0, at) + NASTY[Math.floor(r() * NASTY.length)] + s.slice(at)
    else if (op < 0.7) s = s.slice(0, at) + s.slice(at + 1 + Math.floor(r() * 6))
    else if (op < 0.85) s = s.slice(0, at) + s.slice(Math.floor(r() * s.length)) // splice in a copy of itself
    else s = s.slice(0, at) + String.fromCharCode(Math.floor(r() * 0x3000)) + s.slice(at)
  }
  return s
}

const LRC = [
  '[ti:Song]\n[ar:Someone]\n[offset:+500]\n[00:01.50]First\n[00:10.00][00:20.00]Chorus\n[01:02.25] Last',
  '[00:01.00]<00:01.00>Hello <00:01.50>bright <00:02.10>world<00:03.00>',
  '[by：x]\n[00:00:50]a\n[1:00:05.10]b\nplain\n',
]
const TTML = [
  `<?xml version="1.0"?><tt xmlns="http://www.w3.org/ns/ttml" ttp:tickRate="1000"><body><div><p begin="00:01.000" end="00:03.000"><span begin="00:01.000" end="00:01.600">Rock</span> <span begin="1600t" end="2400t">&amp;</span><span ttm:role="x-bg"><span begin="2.5s">(ooh</span> <span begin="2.7s">yeah)</span></span></p><p begin="12s" dur="2s">Plain&#8217;s<br/>line</p></div></body></tt>`,
]
const M3U = ['\uFEFF#EXTM3U\r\n#EXTINF:215,A - B\r\nD:\\Music\\a.flac\r\n../x/b.mp3\r\n\\\\nas\\s\\c.flac\r\nfile:///D:/My%20Song.flac\r\nhttps://x.org/s\r\n']

const ITER = 5000

describe('fuzz', () => {
  it('LRC: never throws, times are finite and sorted, tags never show as lyrics', () => {
    const r = rng(1)
    for (let i = 0; i < ITER; i++) {
      const l = parseLrc(mutate(LRC[i % LRC.length], r))
      for (let j = 0; j < l.lines.length; j++) {
        const line = l.lines[j]
        expect(Number.isFinite(line.t) && Number.isFinite(line.end)).toBe(true)
        if (j) expect(line.t).toBeGreaterThanOrEqual(l.lines[j - 1].t)
        expect(META_TAG.test(line.text)).toBe(false)
        for (const w of line.words ?? []) expect(w.t >= 0 && Number.isFinite(w.end)).toBe(true)
      }
    }
  })
  it('TTML: never throws, times are finite and sorted', () => {
    const r = rng(2)
    for (let i = 0; i < ITER; i++) {
      const l = parseTtml(mutate(TTML[0], r))
      for (let j = 0; j < l.lines.length; j++) {
        expect(Number.isFinite(l.lines[j].t) && Number.isFinite(l.lines[j].end)).toBe(true)
        if (l.synced && j) expect(l.lines[j].t).toBeGreaterThanOrEqual(l.lines[j - 1].t)
      }
    }
  })
  it('TTML: a huge file is refused instead of crawling', () => {
    const big = '<tt>' + '<p begin="1s">'.repeat(200_000)
    const t = performance.now()
    expect(parseTtml(big).lines).toEqual([])
    expect(performance.now() - t).toBeLessThan(500)
  })
  it('M3U: never throws, round-trips what it writes, never climbs above a drive', () => {
    const r = rng(3)
    for (let i = 0; i < ITER; i++) {
      const entries = parseM3u(mutate(M3U[0], r), i % 2 ? 'D:\\Music\\Lists' : '/home/me/Lists')
      for (const e of entries) {
        expect(typeof e.path).toBe('string')
        if (/^[a-z]:/i.test(e.path)) expect(e.path).not.toMatch(/\\\.\.(\\|$)/)
      }
      const out = buildM3u(entries.map((e) => ({ path: e.path, title: e.title ?? '', artist: '', duration: 0 })))
      expect(out.split('\r\n').filter((l) => l && !l.startsWith('#')).length).toBe(entries.length)
    }
  })
  it('EQ presets: never throws, every band is finite', () => {
    const r = rng(4)
    const apo = 'Preamp: -6.2 dB\nFilter 1: ON PK Fc 105 Hz Gain 4.5 dB Q 0.70\nFilter 2: ON LSC Fc 80 Hz Gain 3 dB Q 0.7\nFilter 3: ON HSC Fc 10000 Hz Gain -2 dB Q 0.7\n'
    for (let i = 0; i < ITER; i++) {
      const p = parseApo(mutate(apo, r))
      if (!p) continue
      expect(Number.isFinite(p.preamp)).toBe(true)
      for (const b of p.bands) expect([b.f, b.g, b.q].every(Number.isFinite)).toBe(true)
    }
  })
})
