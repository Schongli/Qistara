import { describe, expect, it } from 'vitest'
import { detect, languageOf, mozCode, pairName, route } from './mt'

const offered = [
  { from: 'ja', to: 'en' }, { from: 'en', to: 'id' }, { from: 'id', to: 'en' }, { from: 'en', to: 'ja' }, { from: 'ar', to: 'en' },
]

describe('translate on this PC', () => {
  it('maps the settings language codes to Mozilla’s', () => {
    expect(mozCode('id')).toBe('id')
    expect(mozCode('zh-CN')).toBe('zh')
    expect(mozCode('zh-TW')).toBe('zh_hant')
    expect(mozCode('no')).toBe('nb')
    expect(mozCode('pt-PT')).toBe('pt')
  })

  it('names languages and pairs in English', () => {
    expect(languageOf('ja')).toBe('Japanese')
    expect(languageOf('zh_hant')).toBe('Traditional Chinese')
    expect(pairName({ from: 'id', to: 'en' })).toBe('Indonesian → English')
  })

  it('goes direct, through English, or says there is no way', () => {
    expect(route('ja', 'en', offered)).toEqual([{ from: 'ja', to: 'en' }])
    expect(route('ja', 'id', offered)).toEqual([{ from: 'ja', to: 'en' }, { from: 'en', to: 'id' }])
    expect(route('id', 'id', offered)).toEqual([])
    expect(route('ar', 'ja', offered)).toEqual([{ from: 'ar', to: 'en' }, { from: 'en', to: 'ja' }])
    expect(route('ko', 'en', offered)).toBeNull()
    expect(route('en', 'ar', offered)).toBeNull()
  })

  it('knows the language of lyrics from their script or letter patterns', () => {
    expect(detect(['君の名前を呼んでいる', '夜空に星が輝いている'])).toBe('ja')
    expect(detect(['사랑해요', '보고 싶어요'])).toBe('ko')
    expect(detect(['我爱你', '月亮代表我的心'])).toBe('zh')
    expect(detect(['ฉันรักเธอ'])).toBe('th')
    expect(detect([
      'Aku ingin bertemu denganmu lagi', 'Malam ini bintang bersinar terang', 'Kau selalu ada di dalam hatiku', 'Jangan pergi dariku',
    ])).toBe('id')
    expect(detect([
      'I can hear the ocean calling me home', 'Every tide brings back a memory of you', 'We were young and the night was ours',
    ])).toBe('en')
    expect(detect([
      'Quiero verte otra vez esta noche', 'Bajo las estrellas de la ciudad', 'Tu corazón y el mío', 'Nunca te voy a olvidar',
    ])).toBe('es')
    expect(detect(['أحبك يا قلبي', 'في الليل والنهار أفكر فيك دائما'])).toBe('ar')
    expect(detect(['', '♪ ♪', '123'])).toBeNull()
  })
})
