import { describe, expect, it } from 'vitest'
import { plan, type HostItem, type HostState, type Mine } from './session'

const item = (seq: number): HostItem => ({ seq, kind: 'file', title: `t${seq}`, artist: '', album: '', duration: 200 })
const host = (o: Partial<HostState> = {}): HostState => ({ cur: item(1), next: item(2), pos: 30, paused: false, ...o })
const me = (o: Partial<Mine> = {}): Mine => ({ curSeq: 1, nextSeq: 2, time: 30, duration: 200, paused: false, ...o })

describe('following the host', () => {
  it('does nothing when already in step', () => {
    expect(plan(me({ time: 30.4 }), host(), 0)).toEqual([])
  })
  it('loads the host song where the host is, counting the time since it said so', () => {
    expect(plan(me({ curSeq: null, nextSeq: null }), host(), 1.5)).toEqual([{ do: 'load', at: 31.5 }])
    expect(plan(me({ curSeq: null, nextSeq: null }), host({ paused: true }), 9)).toEqual([{ do: 'load', at: 30 }, { do: 'pause', paused: true }])
  })
  it('seeks only past the drift that can be heard', () => {
    expect(plan(me({ time: 29.2 }), host(), 0)).toEqual([])
    expect(plan(me({ time: 25 }), host(), 0)).toEqual([{ do: 'seek', at: 30 }])
  })
  it('pauses and plays with the host', () => {
    expect(plan(me(), host({ paused: true }), 3)).toEqual([{ do: 'pause', paused: true }])
    expect(plan(me({ paused: true }), host(), 0)).toEqual([{ do: 'pause', paused: false }])
  })
  it('lines up the host next song for a gapless change', () => {
    expect(plan(me({ nextSeq: null }), host(), 0)).toEqual([{ do: 'next' }])
    expect(plan(me(), host({ next: null }), 0)).toEqual([{ do: 'next' }])
  })
  it('lets its own gapless change happen instead of cutting in', () => {
    // the host is on song 2 already; this player is 1 s from the end of song 1 with song 2 lined up
    expect(plan(me({ time: 199 }), host({ cur: item(2), next: item(3), pos: 0.5 }), 0)).toEqual([])
    // but far from the end, it jumps
    expect(plan(me({ time: 120 }), host({ cur: item(2), next: item(3), pos: 0.5 }), 0)).toEqual([{ do: 'load', at: 0.5 }])
  })
  it('stops when the host stops', () => {
    expect(plan(me(), host({ cur: null, next: null }), 0)).toEqual([{ do: 'stop' }])
    expect(plan(me({ curSeq: null }), host({ cur: null }), 0)).toEqual([])
  })
})
