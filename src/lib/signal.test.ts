import { describe, expect, it } from 'vitest'
import { ALSA_DEVICE, bitsOf, osOf, outputLabel, pickable, verdict, type SignalInput } from './signal'

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

describe('macOS and Linux', () => {
  it('passes CoreAudio only once hog mode is granted', () => {
    expect(verdict({ ...base, ao: 'coreaudio_exclusive' }).bitPerfect).toBe(true)
    expect(verdict({ ...base, ao: 'coreaudio' }).issues).toEqual(['Shared mode: macOS may resample'])
  })
  it('passes ALSA only straight to a hw: device', () => {
    expect(verdict({ ...base, ao: 'alsa', device: 'alsa/hw:CARD=Mojo,DEV=0' }).bitPerfect).toBe(true)
    for (const device of ['auto', 'alsa/default', 'alsa/plughw:CARD=Mojo,DEV=0', 'alsa/sysdefault:CARD=PCH'])
      expect(verdict({ ...base, ao: 'alsa', device }).issues).toEqual(['Not a hw: device: ALSA or PipeWire may resample'])
  })
  it('lists the same devices mpv.rs accepts', () => {
    for (const ok of ['alsa/default', 'alsa/hw:CARD=PCH,DEV=0', 'alsa/iec958:CARD=PCH', 'alsa/hdmi:CARD=NVidia,DEV=3']) expect(ALSA_DEVICE.test(ok)).toBe(true)
    for (const no of ["alsa/file:'/tmp/x',raw", 'alsa/dmix:CARD=PCH', 'alsa/hw:CARD=PCH,DEV=100', 'pulse/x', 'alsa/pipewire']) expect(ALSA_DEVICE.test(no)).toBe(false)
    expect([pickable('windows', 'wasapi/{x}'), pickable('windows', 'alsa/default'), pickable('macos', 'coreaudio/BuiltIn'), pickable('linux', 'auto')]).toEqual([true, false, true, true])
  })
  it('names the output and the OS', () => {
    expect([outputLabel('wasapi', true), outputLabel('coreaudio_exclusive', true), outputLabel('alsa', true, 'alsa/hw:CARD=X,DEV=0')]).toEqual(['WASAPI exclusive', 'CoreAudio hog mode', 'ALSA direct to hardware'])
    expect(osOf('Mozilla/5.0 (Windows NT 10.0; Win64; x64)')).toBe('windows')
    expect(osOf('Mozilla/5.0 (Macintosh; Intel Mac OS X 10_15_7) AppleWebKit')).toBe('macos')
    expect(osOf('Mozilla/5.0 (X11; Linux x86_64) AppleWebKit')).toBe('linux')
  })
})
