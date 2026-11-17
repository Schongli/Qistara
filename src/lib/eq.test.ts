import { describe, expect, it } from 'vitest'
import { afString, clampBand, FLAT, filterOf, parseApo, preampOf, response, total, type Band, type Eq } from './eq'

const band = (o: Partial<Band>): Band => ({ type: 'peak', f: 1000, g: 0, q: 1, on: true, ...o })

describe('EQ response (RBJ cookbook)', () => {
  it('a peak band hits its gain exactly at its centre and fades far away', () => {
    expect(response(band({ g: 6 }), 1000)).toBeCloseTo(6, 4)
    expect(response(band({ g: -9 }), 1000)).toBeCloseTo(-9, 4)
    expect(Math.abs(response(band({ g: 6 }), 30))).toBeLessThan(0.1)
  })
  it('shelves reach their gain on their side', () => {
    expect(response(band({ type: 'lowshelf', f: 100, g: 6, q: 0.7 }), 20)).toBeCloseTo(6, 0)
    expect(response(band({ type: 'lowshelf', f: 100, g: 6, q: 0.7 }), 10000)).toBeCloseTo(0, 1)
    expect(response(band({ type: 'highshelf', f: 8000, g: -4, q: 0.7 }), 20000)).toBeCloseTo(-4, 0)
  })
  it('cuts are −3 dB at the corner (Butterworth Q) and roll off', () => {
    expect(response(band({ type: 'lowcut', f: 80, q: Math.SQRT1_2 }), 80)).toBeCloseTo(-3.01, 1)
    expect(response(band({ type: 'lowcut', f: 80, q: Math.SQRT1_2 }), 20)).toBeLessThan(-20)
    expect(response(band({ type: 'highcut', f: 10000, q: Math.SQRT1_2 }), 1000)).toBeCloseTo(0, 1)
  })
})

describe('mpv filter string', () => {
  it('is empty when off or when every band is off', () => {
    expect(afString(FLAT)).toBe('')
    expect(afString({ ...FLAT, enabled: true, bands: FLAT.bands.map((b) => ({ ...b, on: false })) })).toBe('')
  })
  it('labels each band and puts an anti-clipping preamp first', () => {
    const eq: Eq = { enabled: true, autoPreamp: true, preamp: 0, bands: [band({ g: 6 }), band({ type: 'lowcut', f: 30, q: 0.707, on: false })] }
    expect(afString(eq)).toBe('@pre:lavfi=[volume=-6dB],@eq0:lavfi=[equalizer=f=1000:t=q:w=1:g=6]')
    expect(preampOf({ ...eq, bands: [band({ g: -4 })] })).toBe(-0) // cuts never need headroom
    expect(filterOf(band({ type: 'highcut', f: 12000.12345, q: 0.71 }))).toBe('lowpass=f=12000.123:p=2:t=q:w=0.71')
  })
})

describe('AutoEQ / Equalizer APO import', () => {
  it('parses ParametricEQ.txt', () => {
    const eq = parseApo(`Preamp: -6.4 dB
Filter 1: ON LSC Fc 105 Hz Gain 5.7 dB Q 0.70
Filter 2: ON PK Fc 2400 Hz Gain -3.1 dB Q 2.12
Filter 3: OFF HSC Fc 10000 Hz Gain 2.0 dB Q 0.70`)!
    expect(eq.preamp).toBe(-6.4)
    expect(eq.autoPreamp).toBe(false)
    expect(eq.bands).toEqual([
      { type: 'lowshelf', f: 105, g: 5.7, q: 0.7, on: true },
      { type: 'peak', f: 2400, g: -3.1, q: 2.12, on: true },
      { type: 'highshelf', f: 10000, g: 2, q: 0.7, on: false },
    ])
    expect(parseApo('nothing here')).toBeNull()
  })
})

describe('clamping', () => {
  it('keeps every number in range and never lets NaN or an emptied field through', () => {
    expect(clampBand(band({ f: 5, g: 40, q: 0 }))).toEqual(band({ f: 20, g: 24, q: 0.1 }))
    expect(clampBand(band({ f: 30000 }), 44100).f).toBe(19845)
    const prev = band({ f: 440, g: 3, q: 2 })
    expect(clampBand(band({ f: null as unknown as number, g: NaN, q: Infinity }), 48000, prev)).toEqual(band({ f: 440, g: 3, q: 2 }))
    expect(clampBand({ ...band({}), type: 'bogus' as Band['type'] }).type).toBe('peak')
  })
  it('never puts a bad number into the mpv filter chain', () => {
    const eq: Eq = { enabled: true, autoPreamp: false, preamp: NaN, bands: [band({ f: null as unknown as number, g: 99, q: -1 })] }
    const af = afString(eq)
    expect(af).not.toMatch(/NaN|null|Infinity/)
    expect(af).toBe('@pre:lavfi=[volume=0dB],@eq0:lavfi=[equalizer=f=1000:t=q:w=0.1:g=24]')
  })
  it('draws with the sample rate that plays', () => {
    const b = band({ type: 'highshelf', f: 12000, g: 6, q: 0.7 })
    // the same filter sounds different near Nyquist at 44.1 kHz than at 96 kHz
    expect(Math.abs(response(b, 18000, 44100) - response(b, 18000, 96000))).toBeGreaterThan(0.2)
    const eq: Eq = { enabled: true, autoPreamp: true, preamp: 0, bands: [b] }
    expect(total(eq, 1000, 96000)).toBeCloseTo(response(b, 1000, 96000), 6)
    expect(preampOf(eq, 44100)).toBeLessThanOrEqual(0)
  })
  it('clamps imported AutoEQ bands too', () => {
    expect(parseApo('Filter 1: ON PK Fc 5 Hz Gain 60 dB Q 50')!.bands[0]).toEqual(band({ f: 20, g: 24, q: 18 }))
  })
})
