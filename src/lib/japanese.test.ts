import { describe, expect, it } from 'vitest'
import { gloss, isJapanese, lineRomaji, posName, withGaps, type JpWord } from './japanese'

// IPADIC output for 君の名前を呼んでいた, as japanese.rs returns it
const w = (text: string, pos: string, pos1: string, base: string, reading: string, form = ''): JpWord => ({ text, pos, pos1, base, reading, form })
const line = [
  w('君', '名詞', '代名詞', '君', 'キミ'),
  w('の', '助詞', '連体化', 'の', 'ノ'),
  w('名前', '名詞', '一般', '名前', 'ナマエ'),
  w('を', '助詞', '格助詞', 'を', 'ヲ'),
  w('呼ん', '動詞', '自立', '呼ぶ', 'ヨン', '連用タ接続'),
  w('で', '助詞', '接続助詞', 'で', 'デ'),
  w('い', '動詞', '非自立', 'いる', 'イ', '連用形'),
  w('た', '助動詞', '', 'た', 'タ', '基本形'),
]

describe('japanese', () => {
  it('spots Japanese lyrics by their kana', () => {
    expect(isJapanese(['Hello', '夜空に星が降る'])).toBe(true)
    expect(isJapanese(['我爱你'])).toBe(false) // Chinese has no kana
    expect(isJapanese(['Hello'])).toBe(false)
  })

  it('writes a line in romaji, with particles read as spoken and endings joined', () => {
    expect(lineRomaji(line)).toBe('kimi no namae o yonde ita')
    expect(lineRomaji([w('僕', '名詞', '代名詞', '僕', 'ボク'), w('は', '助詞', '係助詞', 'は', 'ハ'), w('、', '記号', '読点', '、', '、'), w('I', '名詞', '固有名詞', 'I', '')])).toBe('boku wa, I')
  })

  it('names parts of speech in English', () => {
    expect(posName(line[0])).toBe('pronoun')
    expect(posName(line[4])).toBe('verb')
    expect(posName(line[6])).toBe('helper verb')
    expect(posName(line[7])).toBe('auxiliary')
    expect(posName(w('きれい', '名詞', '形容動詞語幹', 'きれい', 'キレイ'))).toBe('na-adjective')
    expect(posName(w('love', '名詞', '固有名詞', 'love', ''))).toBe('foreign word')
  })

  it('keeps the spaces between words and makes only words tappable', () => {
    const words = [w('I', '名詞', '固有名詞', 'I', ''), w('love', '名詞', '固有名詞', 'love', ''), w('、', '記号', '読点', '、', '、'), w('だけど', '接続詞', '', 'だけど', 'ダケド')]
    const pieces = withGaps('I love、 だけど', words)
    expect(pieces.map((p) => p.text).join('')).toBe('I love、 だけど')
    expect(pieces.map((p) => [p.text, p.tap])).toEqual([['I', true], [' ', false], ['love', true], ['、', false], [' ', false], ['だけど', true]])
  })

  it('explains grammar words', () => {
    expect(gloss(line[3])).toMatch(/object/)
    expect(gloss(line[5])).toMatch(/te-form/)
    expect(gloss(line[6])).toMatch(/te iru/)
    expect(gloss(line[7])).toMatch(/past/)
    expect(gloss(w('た', '助動詞', '', 'た', 'タラ', '仮定形'))).toMatch(/if/)
    expect(gloss(line[2])).toBe('')
  })
})
