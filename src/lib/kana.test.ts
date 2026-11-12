import { describe, expect, it } from 'vitest'
import { furigana, hasKanji, toHiragana, toRomaji } from './kana'

describe('kana', () => {
  it('turns katakana into hiragana and keeps the rest', () => {
    expect(toHiragana('ヨゾラ')).toBe('よぞら')
    expect(toHiragana('ラブレター')).toBe('らぶれたー')
    expect(toHiragana('abc 星')).toBe('abc 星')
  })

  it('writes Hepburn romaji', () => {
    expect(toRomaji('きみ')).toBe('kimi')
    expect(toRomaji('しんじる')).toBe('shinjiru')
    expect(toRomaji('ちゃん')).toBe('chan')
    expect(toRomaji('きって')).toBe('kitte')
    expect(toRomaji('まっちゃ')).toBe('matcha')
    expect(toRomaji('ふゆ')).toBe('fuyu')
    expect(toRomaji('じゅう')).toBe('juu')
    expect(toRomaji('こんや')).toBe("kon'ya")
    expect(toRomaji('しんあい')).toBe("shin'ai")
    expect(toRomaji('らぶれたー')).toBe('raburetaa')
    expect(toRomaji('ヴぁ')).toBe('va')
  })

  it('spots kanji', () => {
    expect(hasKanji('呼ん')).toBe(true)
    expect(hasKanji('ラブレター')).toBe(false)
  })

  it('puts readings over the kanji only', () => {
    expect(furigana('降る', 'ふる')).toEqual([{ text: '降', rt: 'ふ' }, { text: 'る' }])
    expect(furigana('名前', 'なまえ')).toEqual([{ text: '名前', rt: 'なまえ' }])
    expect(furigana('お願い', 'おねがい')).toEqual([{ text: 'お' }, { text: '願', rt: 'ねが' }, { text: 'い' }])
    expect(furigana('引き出し', 'ひきだし')).toEqual([{ text: '引', rt: 'ひ' }, { text: 'き' }, { text: '出', rt: 'だ' }, { text: 'し' }])
    expect(furigana('ラブレター', 'らぶれたー')).toEqual([{ text: 'ラブレター' }])
    expect(furigana('星', '')).toEqual([{ text: '星' }])
  })
})
