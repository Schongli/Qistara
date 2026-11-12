import { describe, expect, it } from 'vitest'
import { bitsOf, verdict, type SignalInput } from './signal'

const base: SignalInput = {
  audioIn: { format: 's32', samplerate: 96000, 'channel-count': 2 },
  audioOut: { format: 's32', samplerate: 96000, 'channel-count': 2 },
  ao: 'wasapi', exclusive: true, filters: 0, volume: 100, speed: 1, replayGain: false, sourceBits: 24,
}

describe('verdict', () => {
  it('passes an untouched exclusive path', () => {
    expect(verdict(base)).toEqual({ bitPerfect: true, issues: [] })
  })
  it('accepts 24-bit content carried in s24 or padded s32', () => {
    expect(verdict({ ...base, audioOut: { ...base.audioOut!, format: 's24' } }).bitPerfect).toBe(true)
    expect(verdict({ ...base, audioIn: { ...base.audioIn!, format: 's16p' }, sourceBits: 16 }).bitPerfect).toBe(true)
  })
  it('flags every step that breaks bit-perfection', () => {
    const v = verdict({
      ...base, exclusive: false, filters: 2, volume: 80, speed: 1.25, replayGain: true,
      audioOut: { format: 's16', samplerate: 48000, 'channel-count': 1 },
    })
    expect(v.bitPerfect).toBe(false)
    expect(v.issues).toEqual([
      'Shared mode: Windows mixer may resample', 'Resampled 96 kHz → 48 kHz', 'Downmixed 2ch → 1ch',
      'Processed (EQ)', 'Sound Check changes level', 'Software volume', 'Playback speed changed', 'Truncated to 16-bit',
    ])
  })
  it('treats mono duplicated to stereo as bit-perfect, but not 5.1 → stereo', () => {
    expect(verdict({ ...base, audioIn: { ...base.audioIn!, 'channel-count': 1 } }).bitPerfect).toBe(true)
    expect(verdict({ ...base, audioIn: { ...base.audioIn!, 'channel-count': 6 } }).issues).toEqual(['Downmixed 6ch → 2ch'])
  })
  it('treats bitstream passthrough as untouched', () => {
    expect(verdict({ ...base, audioIn: { format: 'floatp', samplerate: 48000, 'channel-count': 8 }, audioOut: { format: 'spdif-truehd', samplerate: 192000, 'channel-count': 8 } }).bitPerfect).toBe(true)
  })
  it('flags float/int conversion and missing playback', () => {
    expect(verdict({ ...base, audioIn: { ...base.audioIn!, format: 'floatp' } }).issues).toEqual(['Converted float → s32'])
    expect(verdict({ ...base, audioIn: null }).bitPerfect).toBe(false)
  })
  it('reads bit depth from mpv format names', () => {
    expect([bitsOf('u8'), bitsOf('s16p'), bitsOf('s24'), bitsOf('s32'), bitsOf('float'), bitsOf('doublep')]).toEqual([8, 16, 24, 32, 32, 64])
  })
})
