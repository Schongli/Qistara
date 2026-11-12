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
