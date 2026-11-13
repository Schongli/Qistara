// Qur'an Learn mode for the playing track: the surah's text with word meanings, transliteration and
// translations, and which ayah is being recited. Text comes from quran.com (Hafs) and the open
// fawazahmed0 editions (Shu'bah text, The Clear Quran) through quran.rs, cached in kv per surah.
// Ayah times: quran.com's own timings when the file is Alafasy's murattal recording, otherwise the
// pauses in the audio (quran.rs) placed with alignAyahs, plus any ayahs the listener tapped.
import { invoke } from '@tauri-apps/api/core'
import { untrack } from 'svelte'
import { queue } from './queue.svelte'
import { settings } from './settings.svelte'
import { kvGet, kvSet } from './persist'
import { alignAyahs, detectRecitation, hasBasmala, joinTanwin, normalizeArabic, type Recitation, type Unit } from './quran'
import type { Track } from './library.svelte'

export type TranslationId = 'kemenag' | 'saheeh' | 'clear'
export const TRANSLATIONS: { id: TranslationId; name: string; short: string; lang: string }[] = [
  { id: 'kemenag', name: 'Kemenag (Indonesian)', short: 'Kemenag', lang: 'id' },
  { id: 'saheeh', name: 'Saheeh International', short: 'Saheeh', lang: 'en' },
  { id: 'clear', name: 'The Clear Quran (Dr. Mustafa Khattab)', short: 'Clear Quran', lang: 'en' },
]

/** One word: Uthmani text, transliteration, meaning, and its time in the murattal recording. */
export interface QWord { ar: string; tr: string; mean: string; seg?: [number, number] }
/** words is null where the Shu'bah wording differs from Hafs, so no Hafs word data fits it. */
export interface QAyah { surah: number; n: number; ar: string; words: QWord[] | null; tr: Partial<Record<TranslationId, string>> }
export type QUnit = { kind: 'istiadha' } | { kind: 'basmala'; surah: number } | { kind: 'ayah'; ayah: QAyah }

export const ISTIADHA = 'أَعُوذُ بِٱللَّهِ مِنَ ٱلشَّيْطَٰنِ ٱلرَّجِيمِ'
export const BASMALA = 'بِسْمِ ٱللَّهِ ٱلرَّحْمَٰنِ ٱلرَّحِيمِ'

export const quran = $state({
  rec: null as Recitation | null,
  path: '',
  units: [] as QUnit[],
  /** start time of each unit, once known */
  starts: null as number[] | null,
  /** the file is quran.com's murattal recording, so words have their own times */
  exact: false,
  status: '' as '' | 'loading' | 'analysing' | 'ready' | 'error',
  error: '',
  /** "Fix sync" mode: the next ayah tapped is marked as starting now */
  fixing: false,
  anchors: {} as Record<number, number>,
  /** playing from a URL, not a file: no pause analysis */
  stream: false,
})

// ---- data ----

type Verses = { verses: { verse_number: number; words: { char_type_name: string; text_uthmani: string; transliteration?: { text: string | null }; translation?: { text: string | null } }[]; translations?: { resource_id: number; text: string }[] }[] }
type Timings = { audio_file: { timestamps: { verse_key: string; timestamp_from: number; timestamp_to: number; segments?: number[][] }[] } }
type Edition = { chapter: { verse: number; text: string }[] }

/** Translations carry footnote markers (<sup foot_note=…>1</sup>) and the odd tag. */
const clean = (s: string) => s.replace(/<sup[^>]*>.*?<\/sup>/g, '').replace(/<[^>]+>/g, '').replace(/\s+/g, ' ').trim()

interface SurahData {
  ayahs: { n: number; words: { ar: string; tr: string; mean: string }[]; kemenag: string; saheeh: string }[]
  timings: { from: number; to: number; segs: number[][] }[] | null
  shubah: string[] | null
  clear: string[] | null
}

async function cached<T>(key: string, get: () => Promise<T>): Promise<T> {
  const hit = await kvGet<T>(key)
  if (hit) return hit
  const v = await get()
  kvSet(key, v)
  return v
}

const fetchSource = <T>(source: object) => invoke<T>('quran_fetch', { source })

async function loadSurah(n: number, lang: 'id' | 'en', shubah: boolean, clear: boolean): Promise<SurahData> {
  const [verses, timings, shu, cq] = await Promise.all([
    cached(`quran:v1:verses:${lang}:${n}`, async () => {
      const v = await fetchSource<Verses>({ kind: 'verses', chapter: n, lang })
      return v.verses.map((x) => ({
        n: x.verse_number,
        words: x.words.filter((w) => w.char_type_name === 'word').map((w) => ({ ar: w.text_uthmani, tr: w.transliteration?.text ?? '', mean: w.translation?.text ?? '' })),
        kemenag: clean(x.translations?.find((t) => t.resource_id === 33)?.text ?? ''),
        saheeh: clean(x.translations?.find((t) => t.resource_id === 20)?.text ?? ''),
      }))
    }),
    cached(`quran:v1:timings:${n}`, async () => {
      const t = await fetchSource<Timings>({ kind: 'timings', chapter: n })
      return t.audio_file.timestamps.map((x) => ({ from: x.timestamp_from, to: x.timestamp_to, segs: x.segments ?? [] }))
    }).catch(() => null), // no template: letter counts stand in
    shubah ? cached(`quran:v1:shubah:${n}`, async () => (await fetchSource<Edition>({ kind: 'shubah', chapter: n })).chapter.map((x) => x.text)) : null,
    // one translation missing shouldn't hide the surah; Shu'bah text stays required, or Hafs would show under its name
    clear ? cached(`quran:v1:clear:${n}`, async () => (await fetchSource<Edition>({ kind: 'clearQuran', chapter: n })).chapter.map((x) => x.text)).catch(() => null) : null,
  ])
  return { ayahs: verses, timings, shubah: shu, clear: cq }
}

/** Arabic letters only, to compare Shu'bah and Hafs words and to weigh ayahs without timings. */
const letters = (s: string) => normalizeArabic(s).replace(/[^ء-ي]/g, '')

function buildAyah(surah: number, a: SurahData['ayahs'][number], d: SurahData, i: number, exact: boolean): QAyah {
  const seg = exact ? d.timings?.[i]?.segs : undefined
  let words: QWord[] | null = a.words.map((w, k) => {
    const s = seg?.find((x) => x[0] === k + 1)
    return { ...w, ...(s && { seg: [s[1] / 1000, s[2] / 1000] as [number, number] }) }
  })
  let ar = a.words.map((w) => w.ar).join(' ')
  const shu = d.shubah?.[i] && joinTanwin(d.shubah[i])
  if (shu) {
    ar = shu
    const tokens = shu.split(/\s+/).filter((x) => letters(x))
    // same number of words: keep Hafs meanings word by word, showing the Shu'bah spelling
    words = tokens.length === words.length ? words.map((w, k) => ({ ...w, ar: tokens[k] })) : null
  }
  return { surah, n: a.n, ar, words, tr: { kemenag: a.kemenag, saheeh: a.saheeh, clear: d.clear?.[i] ?? '' } }
}

// ---- sync ----

const unitWeight = (u: QUnit, d: Map<number, SurahData>) => {
  if (u.kind !== 'ayah') return 5
  const t = d.get(u.ayah.surah)?.timings?.[u.ayah.n - 1]
  return t ? (t.to - t.from) / 1000 : Math.max(1, letters(u.ayah.ar).length * 0.08)
}

let data = new Map<number, SurahData>()

async function sync(t: Track, mine: number) {
  const rec = quran.rec!
  if (rec.partial || quran.stream) {
    // a selection of ayahs, or a stream (YouTube) that can't be scanned for pauses: only taps say where we are
    quran.starts = null
    quran.status = 'ready'
    return
  }
  const one = rec.surahs.length === 1 ? data.get(rec.surahs[0]) : undefined
  const murattal = one?.timings?.at(-1)?.to
  if (murattal && t.duration && Math.abs(t.duration - murattal / 1000) / (murattal / 1000) < 0.01) {
    // this is quran.com's own recording: use its timings, word by word
    quran.exact = true
    quran.units = quran.units.map((u) => (u.kind === 'ayah' ? { kind: 'ayah', ayah: buildAyah(u.ayah.surah, one!.ayahs[u.ayah.n - 1], one!, u.ayah.n - 1, true) } : u))
    quran.starts = quran.units.map((u) => (u.kind === 'ayah' ? one!.timings![u.ayah.n - 1].from / 1000 : 0))
    quran.status = 'ready'
    return
  }
  quran.status = 'analysing'
  const p = await invoke<{ duration: number; pauses: [number, number][] }>('quran_pauses', { path: t.path })
  if (mine !== seq) return
  realign(p)
}

let lastPauses: { duration: number; pauses: [number, number][] } | null = null
function realign(p = lastPauses) {
  if (!p) return
  lastPauses = p
  const units: Unit[] = quran.units.map((u) => ({ weight: unitWeight(u, data), flexible: u.kind !== 'ayah' }))
  quran.starts = alignAyahs(units, p.pauses, p.duration, quran.anchors)
  quran.status = 'ready'
}

// ---- loading ----

const OVERRIDES = 'quranOverrides' // path → a recitation, or 'none'
const ANCHORS = 'quranAnchors' // path → { unit index: start time }
let seq = 0

export async function recitationFor(t: Track): Promise<Recitation | null> {
  const o = (await kvGet<Record<string, Recitation | 'none'>>(OVERRIDES))?.[t.path.toLowerCase()]
  return o === 'none' ? null : (o ?? detectRecitation(t))
}

/** Sets (or clears, with undefined) what a track recites; 'none' means it isn't a recitation. */
export async function overrideRecitation(t: Track, value: Recitation | 'none' | undefined) {
  const all = (await kvGet<Record<string, Recitation | 'none'>>(OVERRIDES)) ?? {}
  if (value === undefined) delete all[t.path.toLowerCase()]
  else all[t.path.toLowerCase()] = value
  await kvSet(OVERRIDES, all)
  if (queue.current?.path === t.path) load(t)
}

async function load(t: Track | null) {
  const mine = ++seq
  Object.assign(quran, { rec: null, path: '', units: [], starts: null, exact: false, status: '', error: '', fixing: false, anchors: {}, stream: false })
  lastPauses = null
  if (!t || t.kind !== 'audio') return
  const rec = await recitationFor(t)
  if (!rec || mine !== seq) return
  Object.assign(quran, { rec, path: t.path, status: 'loading', stream: /^[a-z]+:\/\//i.test(t.path) })
  try {
    const lang = settings.quranTranslations.includes('kemenag') ? 'id' : 'en'
    const wantClear = settings.quranTranslations.includes('clear')
    const next = new Map<number, SurahData>()
    for (const n of rec.surahs) next.set(n, await loadSurah(n, lang, rec.riwayah === 'shubah', wantClear))
    if (mine !== seq) return
    data = next
    const units: QUnit[] = [{ kind: 'istiadha' }]
    for (const n of rec.surahs) {
      const d = next.get(n)!
      if (hasBasmala(n)) units.push({ kind: 'basmala', surah: n })
      d.ayahs.forEach((a, i) => units.push({ kind: 'ayah', ayah: buildAyah(n, a, d, i, false) }))
    }
    quran.units = units
    quran.anchors = (await kvGet<Record<string, Record<number, number>>>(ANCHORS))?.[t.path.toLowerCase()] ?? {}
    await sync(t, mine)
  } catch (e) {
    if (mine !== seq) return
    quran.status = 'error'
    quran.error = e instanceof Error ? e.message : String(e)
  }
}

/** "This ayah starts now": pins the unit to the current time and re-places the rest around it. */
export async function markStart(unit: number, time: number) {
  quran.anchors = { ...quran.anchors, [unit]: Math.round(time * 100) / 100 }
  quran.fixing = false
  const all = (await kvGet<Record<string, Record<number, number>>>(ANCHORS)) ?? {}
  all[quran.path.toLowerCase()] = $state.snapshot(quran.anchors)
  await kvSet(ANCHORS, all)
  if (quran.rec?.partial || quran.stream) quran.starts = null
  else realign()
}

/** Forgets the taps for this track and places the ayahs from the pauses alone. */
export async function clearMarks() {
  quran.anchors = {}
  const all = (await kvGet<Record<string, Record<number, number>>>(ANCHORS)) ?? {}
  delete all[quran.path.toLowerCase()]
  await kvSet(ANCHORS, all)
  realign()
}

/** The unit being recited at `time`: from the sync, or for a partial recitation the last tapped ayah. */
export function currentUnit(time: number): number {
  const s = quran.starts
  if (s) {
    let lo = 0, hi = s.length - 1, ans = 0
    while (lo <= hi) {
      const mid = (lo + hi) >> 1
      if (s[mid] <= time) (ans = mid), (lo = mid + 1)
      else hi = mid - 1
    }
    return ans
  }
  let best = -1, at = -Infinity
  for (const [u, t] of Object.entries(quran.anchors)) if (t <= time && t > at) (best = +u), (at = t)
  return best
}

$effect.root(() => {
  $effect(() => {
    const t = queue.current
    // the translations decide which word-meaning language and editions to load
    void settings.quranTranslations.join()
    untrack(() => load(t))
  })
})
