import { describe, expect, it } from 'vitest'
import { QURAN_TEXT, quranLang } from './qurantext'

describe("Qur'an mode text", () => {
  it('says everything in both languages', () => {
    expect(Object.keys(QURAN_TEXT.id).sort()).toEqual(Object.keys(QURAN_TEXT.en).sort())
    for (const v of Object.values(QURAN_TEXT.id)) expect(v.trim()).not.toBe('')
  })
  it('follows the system unless set', () => {
    expect([quranLang('auto', 'id-ID'), quranLang('auto', 'en-US'), quranLang('id', 'en-US'), quranLang('en', 'id')]).toEqual(['id', 'en', 'id', 'en'])
  })
})
