import { describe, expect, it } from 'vitest'
import { SURAHS, alignAyahs, detectRecitation, joinTanwin, normalizeArabic, type Pause } from './quran'

const t = (title: string, o: Partial<{ album: string; artist: string; albumArtist: string; path: string; track: number | null }> = {}) => ({
  title, album: '', artist: '', albumArtist: '', path: '', track: null, ...o,
})
const HAFS = 'مصحف القراءات العشر 1445هـ - 2024م - حفص عن عاصم'
const SHUBAH = 'مصحف القراءات العشر 1445هـ - 2024م | برواية شعبة عن عاصم'
const range = (a: number, b: number) => Array.from({ length: b - a + 1 }, (_, i) => a + i)

describe('quran detection', () => {
  it('has all 114 surahs with their ayah counts', () => {
    expect(SURAHS).toHaveLength(114)
    expect(SURAHS[1]).toMatchObject({ n: 2, ar: 'البقرة', ayahs: 286 })
    expect(SURAHS.reduce((s, x) => s + x.ayahs, 0)).toBe(6236)
  })

  it('joins the Shu\'bah tanwin alef back onto its word', () => {
    // 78:7 and 2:2 as the edition writes them
    expect(joinTanwin('وَٱلۡجِبَالَ أَوۡتَادࣰ ا').split(' ')).toHaveLength(2)
    expect(joinTanwin('لَا رَيۡبَۛ فِيهِۛ هُدࣰ ى لِّلۡمُتَّقِينَ')).toBe('لَا رَيۡبَۛ فِيهِۛ هُدࣰى لِّلۡمُتَّقِينَ')
    expect(joinTanwin('عَذَابٌ أَلِيمُۢ')).toBe('عَذَابٌ أَلِيمُۢ')
  })

  it('normalises Arabic spelling and marks', () => {
    expect(normalizeArabic('سُورَةُ آلِ عِمْرَانَ')).toBe('سوره ال عمران')
    expect(normalizeArabic('الإسراء')).toBe(normalizeArabic('الاسراء'))
    expect(normalizeArabic('ٱلْفَاتِحَةِ')).toBe('الفاتحه')
  })

  it('reads the Alafasy mushaf titles, both riwayat', () => {
    expect(detectRecitation(t('سورة الفاتحة برواية حفص عن عاصم 1445هـ - 2024م', { album: HAFS, track: 1 }))).toEqual({ surahs: [1], riwayah: 'hafs', partial: false })
    expect(detectRecitation(t('سورة آل عمران برواية شعبة عن عاصم 1445هـ 2024م', { album: SHUBAH, track: 3 }))).toEqual({ surahs: [3], riwayah: 'shubah', partial: false })
    expect(detectRecitation(t('جزء عم برواية شعبة عن عاصم 1445هـ 2024م', { album: SHUBAH, track: 78 }))).toEqual({ surahs: range(78, 114), riwayah: 'shubah', partial: false })
    expect(detectRecitation(t('سورة الإسراء - من صلاة القيام 1420هـ'))?.surahs).toEqual([17])
  })

  it('tells look-alike names apart', () => {
    expect(detectRecitation(t('سورة الحج'))?.surahs).toEqual([22])
    expect(detectRecitation(t('سورة الحجرات'))?.surahs).toEqual([49])
    expect(detectRecitation(t('سورة المؤمنون'))?.surahs).toEqual([23])
    expect(detectRecitation(t('Surah An-Nas'))?.surahs).toEqual([114])
    expect(detectRecitation(t('Surah An-Nasr'))?.surahs).toEqual([110])
  })

  it('reads Latin names in titles, filenames and Qur\'an albums', () => {
    expect(detectRecitation(t('Mishari Rashid Alafasy - Surah Imran - 2024'))?.surahs).toEqual([3])
    expect(detectRecitation(t('Surah Yasin'))?.surahs).toEqual([36])
    expect(detectRecitation(t('Surat Al-Maidah'))?.surahs).toEqual([5])
    expect(detectRecitation(t('Ad-Duha', { album: 'Juz 30 Album ibiidris', artist: 'ibiidris' }))?.surahs).toEqual([93])
    expect(detectRecitation(t('Juz Amma'))?.surahs).toEqual(range(78, 114))
  })

  it('folds the usual spelling variants of surah names', () => {
    const juz30 = (title: string) => detectRecitation(t(title, { album: 'Juz 30 Album ibiidris' }))?.surahs[0]
    const cases: [string, number][] = [
      ['Al-Burooj', 85], ['Al-Kawthur', 108], ['Quraish', 106], ['Al-Lahab', 111], ['Al-Lail', 92], ['Al Feel', 105],
      ['Al-Takweer', 81], ['Al-Teen', 95], ["Al-Mao'un", 107], ['Al-Kafiroon', 109], ['Ad-Duha', 93],
    ]
    for (const [title, n] of cases) expect(juz30(title), title).toBe(n)
  })

  it('marks a selection of ayahs as partial', () => {
    expect(detectRecitation(t('ما تيسر من سورة آل عمران - القارئ عمر بن ضياء الدين'))).toEqual({ surahs: [3], riwayah: 'hafs', partial: true })
  })

  it('uses the track number only inside a mushaf album', () => {
    expect(detectRecitation(t('Track 2', { album: 'Mushaf Hafs', track: 2 }))?.surahs).toEqual([2])
    expect(detectRecitation(t('Astaghfar Dua', { album: 'Duas for laylatul qadr ادعية ليلة القدر (Zikir)', artist: 'Quran; The Holy Quran', track: 1 }))).toBeNull()
  })

  it('leaves nasheeds and other songs alone', () => {
    expect(detectRecitation(t('LAILATUL QADR Dua', { album: 'Duas for laylatul qadr ادعية ليلة القدر (Zikir)', artist: 'Quran; The Holy Quran' }))).toBeNull()
    expect(detectRecitation(t('دعاء ليلة القدر', { album: 'مصحف' }))).toBeNull()
    expect(detectRecitation(t('Insha Allah - Arabic - Vocals Only Version', { album: 'Thank You Allah', artist: 'Maher Zain' }))).toBeNull()
    expect(detectRecitation(t('Al Quran', { album: 'Fartaqi', artist: 'Ahmed Bukhatir' }))).toBeNull()
    expect(detectRecitation(t('Nasheed Say to the Aggressor', { album: 'Conquer Your Goals With The Help of Allah' }))).toBeNull()
    expect(detectRecitation(t('Nasheed', { album: 'Juz 30 Album ibiidris' }))).toBeNull()
    expect(detectRecitation(t('Hello', { album: 'Adele 25' }))).toBeNull()
  })
})

// Alafasy's Al-Fatihah (mushaf 1445 H, Hafs): the pauses Confluence Player finds in the real file, and quran.com's
// murattal verse lengths as the template. Isti'adha, then 7 ayahs (the basmala is ayah 1).
const FATIHA_PAUSES: Pause[] = [[6.24, 7.12], [13.86, 15.06], [20.76, 21.46], [26.44, 27.02], [32.46, 33.06], [40.76, 41.52], [48.4, 49.1], [66.82, 73.6]]
const FATIHA_MURATTAL = [6.09, 5.59, 4.62, 4.65, 6.71, 5.59, 13.25]
const fatihaUnits = [{ weight: 5, flexible: true }, ...FATIHA_MURATTAL.map((weight) => ({ weight }))]
const FATIHA_STARTS = [0, 7.12, 15.06, 21.46, 27.02, 33.06, 41.52, 49.1]

describe('ayah alignment', () => {
  it('places every ayah of Al-Fatihah on its pause', () => {
    expect(alignAyahs(fatihaUnits, FATIHA_PAUSES, 73.6)).toEqual(FATIHA_STARTS)
  })

  it('ignores short pauses inside an ayah', () => {
    const extra: Pause[] = [...FATIHA_PAUSES, [9.5, 9.85], [36.2, 36.55], [55, 55.4], [60.1, 60.45]].sort((a, b) => a[0] - b[0]) as Pause[]
    expect(alignAyahs(fatihaUnits, extra, 73.6)).toEqual(FATIHA_STARTS)
  })

  it('lets an opening that was not recited take no time', () => {
    // no isti'adha: the recording starts straight at the basmala
    const pauses = FATIHA_PAUSES.slice(1).map(([s, e]) => [s - 7.12, e - 7.12] as Pause)
    const starts = alignAyahs(fatihaUnits, pauses, 73.6 - 7.12)
    expect(starts[0]).toBe(starts[1])
    expect(starts.slice(1).map((x) => Math.round(x * 100) / 100)).toEqual(FATIHA_STARTS.slice(1).map((x) => Math.round((x - 7.12) * 100) / 100))
  })

  it('follows a tapped start and keeps the rest consistent', () => {
    // a bad template (all ayahs equal) puts ayah 7 too early; tapping it fixes that and what follows
    const flat = [{ weight: 5, flexible: true }, ...FATIHA_MURATTAL.map(() => ({ weight: 6 }))]
    const fixed = alignAyahs(flat, FATIHA_PAUSES, 73.6, { 7: 49.3 })
    expect(fixed[7]).toBe(49.1) // snapped to the pause next to the tap
    expect(fixed).toEqual([...fixed].sort((a, b) => a - b))
  })

  it('spreads ayahs evenly when there are no pauses at all', () => {
    expect(alignAyahs([{ weight: 1 }, { weight: 1 }, { weight: 2 }], [], 40)).toEqual([0, 10, 20])
  })

  it('handles a long surah quickly', () => {
    // 286 ayahs over ~2.5 h, with an ayah pause after each and a waqf pause inside every third one
    const weights = Array.from({ length: 286 }, (_, i) => 8 + ((i * 37) % 23))
    const pauses: Pause[] = []
    let t = 6
    const truth: number[] = []
    for (const [i, w] of weights.entries()) {
      truth.push(t)
      const len = w * 1.23 * (0.9 + ((i * 13) % 20) / 100)
      if (i % 3 === 0) pauses.push([t + len / 2, t + len / 2 + 0.4])
      t += len
      pauses.push([t, t + 0.9])
      t += 0.9
    }
    const units = [{ weight: 5, flexible: true }, ...weights.map((weight) => ({ weight }))]
    const t0 = performance.now()
    const starts = alignAyahs(units, [[0, 0.5], ...pauses.slice(0, -1), [t - 0.9, t + 5]] as Pause[], t + 5)
    expect(performance.now() - t0).toBeLessThan(1500)
    const right = truth.filter((x, i) => Math.abs(starts[i + 1] - (i === 0 ? x : x)) < 0.05).length
    expect(right / truth.length).toBeGreaterThan(0.97)
  })
})
