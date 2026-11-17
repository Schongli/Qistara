import { describe, expect, it } from 'vitest'
import { activeLine, isGap, parseLrc, parseLyrics, parseTtml, ttmlTime } from './lyrics'

describe('LRC', () => {
  it('parses timed lines, repeated stamps and offset', () => {
    const l = parseLrc('[ar:Someone]\n[offset:+500]\n[00:01.50]First\n[00:10.00][00:20.00]Chorus\n\n[01:02.25] Last ')
    expect(l.synced).toBe(true)
    expect(l.lines.map((x) => [x.t, x.text])).toEqual([[1, 'First'], [9.5, 'Chorus'], [19.5, 'Chorus'], [61.75, 'Last']])
    expect(l.lines[0].end).toBe(9.5)
  })
  it('parses enhanced LRC word timing', () => {
    const l = parseLrc('[00:01.00]<00:01.00>Hello <00:01.50>bright <00:02.10>world<00:03.00>')
    expect(l.lines[0].text).toBe('Hello bright world')
    expect(l.lines[0].words).toEqual([
      { t: 1, end: 1.5, text: 'Hello ' },
      { t: 1.5, end: 2.1, text: 'bright ' },
      { t: 2.1, end: 3, text: 'world' },
    ])
    expect(l.lines[0].end).toBe(3)
  })
  it('falls back to plain text', () => {
    const l = parseLrc('[ti:Song]\nLine one\n  Line two ')
    expect(l).toEqual({ synced: false, lines: [{ t: 0, end: 0, text: 'Line one' }, { t: 0, end: 0, text: 'Line two' }] })
  })
})

describe('TTML', () => {
  it('reads clock formats', () => {
    expect([ttmlTime('1:02.5'), ttmlTime('00:01:02.500'), ttmlTime('62.5s'), ttmlTime('62500ms'), ttmlTime(undefined)]).toEqual([62.5, 62.5, 62.5, 62.5, 0])
  })
  it('parses lines with word spans and skips background vocals', () => {
    const xml = `<?xml version="1.0"?><tt xmlns="http://www.w3.org/ns/ttml"><body><div>
      <p begin="00:01.000" end="00:03.000"><span begin="00:01.000" end="00:01.600">Rock</span> <span begin="00:01.600" end="00:02.400">&amp;</span> <span begin="00:02.400" end="00:03.000">roll</span><span ttm:role="x-bg"><span begin="00:02.5" end="00:03">(ooh)</span></span></p>
      <p begin="00:12.000" end="00:14.000">Plain line</p>
    </div></body></tt>`
    const l = parseTtml(xml)
    expect(l.synced).toBe(true)
    expect(l.lines.map((x) => [x.t, x.end, x.text])).toEqual([[1, 3, 'Rock & roll'], [12, 14, 'Plain line']])
    expect(l.lines[0].words?.map((w) => [w.t, w.text])).toEqual([[1, 'Rock '], [1.6, '& '], [2.4, 'roll']])
    expect(parseLyrics('lrc', xml).lines.length).toBe(2) // detected as TTML by content
  })
})

describe('playback helpers', () => {
  const lines = parseLrc('[00:05.00]a\n[00:10.00]b\n[00:30.00]c').lines
  it('finds the active line', () => {
    expect([activeLine(lines, 0), activeLine(lines, 5), activeLine(lines, 12), activeLine(lines, 99)]).toEqual([-1, 0, 1, 2])
  })
  it('detects instrumental gaps from explicit ends, and LRC breaks as empty lines', () => {
    expect(isGap(undefined, lines[0])).toBe(true) // 5 s intro
    expect(isGap(lines[0], lines[1])).toBe(false)
    const ttml = parseTtml('<p begin="1" end="3">x</p><p begin="12s" end="14s">y</p>').lines
    expect(isGap(ttml[0], ttml[1])).toBe(true)
    const brk = parseLrc('[00:05.00]a\n[00:12.00]\n[00:30.00]b').lines
    expect(brk.map((l) => [l.text, l.end - l.t])).toEqual([['a', 7], ['', 18], ['b', 6]])
  })
})

describe('hardening', () => {
  it('reads every common stamp form', () => {
    const l = parseLrc('[00:01]a\n[00:02.5]b\n[00:03:25]c\n[00:04,75]d\n[1:00:05.10]e\n[00:06.123]f')
    expect(l.lines.map((x) => [x.t, x.text])).toEqual([[1, 'a'], [2.5, 'b'], [3.25, 'c'], [4.75, 'd'], [6.123, 'f'], [3605.1, 'e']])
  })
  it('never shows metadata tags as lyrics', () => {
    const l = parseLrc('[ti:Song]\n[ ar : Someone ]\n[by：uploader]\n[00:00.00][al:Album]\n[re:x]\n[ve:1.0]\n[00:01.00]Real line')
    expect(l.lines.map((x) => x.text)).toEqual(['Real line'])
    const plain = parseLrc('[ti:Song]\n[ ar : Someone ]\n[by：uploader]\n[length: 03:20]\nLine')
    expect(plain.lines.map((x) => x.text)).toEqual(['Line'])
    // stamps nobody can read don't leak into plain text
    expect(parseLrc('[00:01.50.123.4]Hello').lines.map((x) => x.text)).toEqual(['Hello'])
  })
  it('splits old Mac line breaks and reads a spaced offset', () => {
    const l = parseLrc('[offset: -1000]\r[00:01.00]a\r[00:02.00]b')
    expect(l.lines.map((x) => [x.t, x.text])).toEqual([[2, 'a'], [3, 'b']])
  })
  it('keeps word times at or after zero', () => {
    const l = parseLrc('[offset:+2000]\n[00:01.00]<00:01.00>Hi <00:01.50>there<00:02.00>')
    expect(l.lines[0].words!.every((w) => w.t >= 0 && w.end >= w.t)).toBe(true)
  })
  it('drops every word of a background-vocal block', () => {
    const xml = `<tt><body><p begin="1s" end="3s"><span begin="1s" end="2s">Rock</span> <span begin="2s" end="3s">roll</span><span ttm:role="x-bg"><span begin="2.5s" end="2.7s">(ooh</span> <span begin="2.7s" end="3s">yeah)</span></span></p></body></tt>`
    const l = parseTtml(xml)
    expect(l.lines[0].text).toBe('Rock roll')
    expect(l.lines[0].words!.map((w) => w.text.trim())).toEqual(['Rock', 'roll'])
  })
  it('handles missing ends, dur, entities, <br/>, single quotes and prefixed tags', () => {
    const xml = `<tt:tt xmlns:tt="http://www.w3.org/ns/ttml"><tt:body><tt:p begin='1s' dur='2s'>It&#8217;s&nbsp;me<tt:br/>again &#x27;ok&#x27;</tt:p><tt:p begin="10s">Next</tt:p></tt:body></tt:tt>`
    const l = parseLyrics('lrc', xml)
    expect(l.synced).toBe(true)
    expect(l.lines.map((x) => [x.t, x.end, x.text])).toEqual([[1, 3, 'It’s me again \'ok\''], [10, 16, 'Next']])
    expect(isGap(l.lines[0], l.lines[1])).toBe(true)
    // no end at all: the line runs to the next, so no gap dots under every line
    const noEnd = parseTtml('<p begin="1s">a</p><p begin="3s">b</p>').lines
    expect(isGap(noEnd[0], noEnd[1])).toBe(false)
  })
  it('reads ticks, frames and offset units', () => {
    expect(ttmlTime('10000000t', { tick: 10000000, frame: 30 })).toBe(1)
    expect([ttmlTime('1.5m'), ttmlTime('0.5h'), ttmlTime('30f'), ttmlTime('00:00:01:15')]).toEqual([90, 1800, 1, 1.5])
    expect(ttmlTime('garbage')).toBeNaN()
    const l = parseTtml('<tt ttp:tickRate="1000"><p begin="1500t" end="2000t">x</p></tt>')
    expect([l.lines[0].t, l.lines[0].end]).toEqual([1.5, 2])
  })
  it('times an untimed <p> from its spans', () => {
    const l = parseTtml('<p><span begin="4s" end="5s">a</span> <span begin="5s" end="6s">b</span></p>')
    expect([l.synced, l.lines[0].t, l.lines[0].end]).toEqual([true, 4, 6])
  })
})
