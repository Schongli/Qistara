// The Qur'an: the 114 surahs, and recognising recitations among library tracks (title, album, filename).

/** [Arabic name, transliterated name, ayah count] for surahs 1–114 (from quran.com). */
const TABLE: [string, string, number][] = [
  ["الفاتحة", "Al-Fatihah", 7],
  ["البقرة", "Al-Baqarah", 286],
  ["آل عمران", "Ali 'Imran", 200],
  ["النساء", "An-Nisa", 176],
  ["المائدة", "Al-Ma'idah", 120],
  ["الأنعام", "Al-An'am", 165],
  ["الأعراف", "Al-A'raf", 206],
  ["الأنفال", "Al-Anfal", 75],
  ["التوبة", "At-Tawbah", 129],
  ["يونس", "Yunus", 109],
  ["هود", "Hud", 123],
  ["يوسف", "Yusuf", 111],
  ["الرعد", "Ar-Ra'd", 43],
  ["ابراهيم", "Ibrahim", 52],
  ["الحجر", "Al-Hijr", 99],
  ["النحل", "An-Nahl", 128],
  ["الإسراء", "Al-Isra", 111],
  ["الكهف", "Al-Kahf", 110],
  ["مريم", "Maryam", 98],
  ["طه", "Taha", 135],
  ["الأنبياء", "Al-Anbya", 112],
  ["الحج", "Al-Hajj", 78],
  ["المؤمنون", "Al-Mu'minun", 118],
  ["النور", "An-Nur", 64],
  ["الفرقان", "Al-Furqan", 77],
  ["الشعراء", "Ash-Shu'ara", 227],
  ["النمل", "An-Naml", 93],
  ["القصص", "Al-Qasas", 88],
  ["العنكبوت", "Al-'Ankabut", 69],
  ["الروم", "Ar-Rum", 60],
  ["لقمان", "Luqman", 34],
  ["السجدة", "As-Sajdah", 30],
  ["الأحزاب", "Al-Ahzab", 73],
  ["سبإ", "Saba", 54],
  ["فاطر", "Fatir", 45],
  ["يس", "Ya-Sin", 83],
  ["الصافات", "As-Saffat", 182],
  ["ص", "Sad", 88],
  ["الزمر", "Az-Zumar", 75],
  ["غافر", "Ghafir", 85],
  ["فصلت", "Fussilat", 54],
  ["الشورى", "Ash-Shuraa", 53],
  ["الزخرف", "Az-Zukhruf", 89],
  ["الدخان", "Ad-Dukhan", 59],
  ["الجاثية", "Al-Jathiyah", 37],
  ["الأحقاف", "Al-Ahqaf", 35],
  ["محمد", "Muhammad", 38],
  ["الفتح", "Al-Fath", 29],
  ["الحجرات", "Al-Hujurat", 18],
  ["ق", "Qaf", 45],
  ["الذاريات", "Adh-Dhariyat", 60],
  ["الطور", "At-Tur", 49],
  ["النجم", "An-Najm", 62],
  ["القمر", "Al-Qamar", 55],
  ["الرحمن", "Ar-Rahman", 78],
  ["الواقعة", "Al-Waqi'ah", 96],
  ["الحديد", "Al-Hadid", 29],
  ["المجادلة", "Al-Mujadila", 22],
  ["الحشر", "Al-Hashr", 24],
  ["الممتحنة", "Al-Mumtahanah", 13],
  ["الصف", "As-Saf", 14],
  ["الجمعة", "Al-Jumu'ah", 11],
  ["المنافقون", "Al-Munafiqun", 11],
  ["التغابن", "At-Taghabun", 18],
  ["الطلاق", "At-Talaq", 12],
  ["التحريم", "At-Tahrim", 12],
  ["الملك", "Al-Mulk", 30],
  ["القلم", "Al-Qalam", 52],
  ["الحاقة", "Al-Haqqah", 52],
  ["المعارج", "Al-Ma'arij", 44],
  ["نوح", "Nuh", 28],
  ["الجن", "Al-Jinn", 28],
  ["المزمل", "Al-Muzzammil", 20],
  ["المدثر", "Al-Muddaththir", 56],
  ["القيامة", "Al-Qiyamah", 40],
  ["الانسان", "Al-Insan", 31],
  ["المرسلات", "Al-Mursalat", 50],
  ["النبإ", "An-Naba", 40],
  ["النازعات", "An-Nazi'at", 46],
  ["عبس", "'Abasa", 42],
  ["التكوير", "At-Takwir", 29],
  ["الإنفطار", "Al-Infitar", 19],
  ["المطففين", "Al-Mutaffifin", 36],
  ["الإنشقاق", "Al-Inshiqaq", 25],
  ["البروج", "Al-Buruj", 22],
  ["الطارق", "At-Tariq", 17],
  ["الأعلى", "Al-A'la", 19],
  ["الغاشية", "Al-Ghashiyah", 26],
  ["الفجر", "Al-Fajr", 30],
  ["البلد", "Al-Balad", 20],
  ["الشمس", "Ash-Shams", 15],
  ["الليل", "Al-Layl", 21],
  ["الضحى", "Ad-Duhaa", 11],
  ["الشرح", "Ash-Sharh", 8],
  ["التين", "At-Tin", 8],
  ["العلق", "Al-'Alaq", 19],
  ["القدر", "Al-Qadr", 5],
  ["البينة", "Al-Bayyinah", 8],
  ["الزلزلة", "Az-Zalzalah", 8],
  ["العاديات", "Al-'Adiyat", 11],
  ["القارعة", "Al-Qari'ah", 11],
  ["التكاثر", "At-Takathur", 8],
  ["العصر", "Al-'Asr", 3],
  ["الهمزة", "Al-Humazah", 9],
  ["الفيل", "Al-Fil", 5],
  ["قريش", "Quraysh", 4],
  ["الماعون", "Al-Ma'un", 7],
  ["الكوثر", "Al-Kawthar", 3],
  ["الكافرون", "Al-Kafirun", 6],
  ["النصر", "An-Nasr", 3],
  ["المسد", "Al-Masad", 5],
  ["الإخلاص", "Al-Ikhlas", 4],
  ["الفلق", "Al-Falaq", 5],
  ["الناس", "An-Nas", 6],
]

/** The names as Indonesian Qur'ans print them (Kemenag's transliteration): "Asy-Syams", "Al-Kausar". */
const ID_NAMES: string[] = [
  "Al-Fatihah",
  "Al-Baqarah",
  "Ali 'Imran",
  "An-Nisa'",
  "Al-Ma'idah",
  "Al-An'am",
  "Al-A'raf",
  "Al-Anfal",
  "At-Taubah",
  "Yunus",
  "Hud",
  "Yusuf",
  "Ar-Ra'd",
  "Ibrahim",
  "Al-Hijr",
  "An-Nahl",
  "Al-Isra'",
  "Al-Kahf",
  "Maryam",
  "Taha",
  "Al-Anbiya'",
  "Al-Hajj",
  "Al-Mu'minun",
  "An-Nur",
  "Al-Furqan",
  "Asy-Syu'ara'",
  "An-Naml",
  "Al-Qasas",
  "Al-'Ankabut",
  "Ar-Rum",
  "Luqman",
  "As-Sajdah",
  "Al-Ahzab",
  "Saba'",
  "Fatir",
  "Yasin",
  "As-Saffat",
  "Sad",
  "Az-Zumar",
  "Gafir",
  "Fussilat",
  "Asy-Syura",
  "Az-Zukhruf",
  "Ad-Dukhan",
  "Al-Jasiyah",
  "Al-Ahqaf",
  "Muhammad",
  "Al-Fath",
  "Al-Hujurat",
  "Qaf",
  "Az-Zariyat",
  "At-Tur",
  "An-Najm",
  "Al-Qamar",
  "Ar-Rahman",
  "Al-Waqi'ah",
  "Al-Hadid",
  "Al-Mujadilah",
  "Al-Hasyr",
  "Al-Mumtahanah",
  "As-Saff",
  "Al-Jumu'ah",
  "Al-Munafiqun",
  "At-Tagabun",
  "At-Talaq",
  "At-Tahrim",
  "Al-Mulk",
  "Al-Qalam",
  "Al-Haqqah",
  "Al-Ma'arij",
  "Nuh",
  "Al-Jinn",
  "Al-Muzzammil",
  "Al-Muddassir",
  "Al-Qiyamah",
  "Al-Insan",
  "Al-Mursalat",
  "An-Naba'",
  "An-Nazi'at",
  "'Abasa",
  "At-Takwir",
  "Al-Infitar",
  "Al-Mutaffifin",
  "Al-Insyiqaq",
  "Al-Buruj",
  "At-Tariq",
  "Al-A'la",
  "Al-Gasyiyah",
  "Al-Fajr",
  "Al-Balad",
  "Asy-Syams",
  "Al-Lail",
  "Ad-Duha",
  "Asy-Syarh",
  "At-Tin",
  "Al-'Alaq",
  "Al-Qadr",
  "Al-Bayyinah",
  "Az-Zalzalah",
  "Al-'Adiyat",
  "Al-Qari'ah",
  "At-Takasur",
  "Al-'Asr",
  "Al-Humazah",
  "Al-Fil",
  "Quraisy",
  "Al-Ma'un",
  "Al-Kausar",
  "Al-Kafirun",
  "An-Nasr",
  "Al-Lahab",
  "Al-Ikhlas",
  "Al-Falaq",
  "An-Nas",
]

/** en: the transliteration quran.com uses; id: the Kemenag one. */
export interface Surah { n: number; ar: string; en: string; id: string; ayahs: number }
export const SURAHS: Surah[] = TABLE.map(([ar, en, ayahs], i) => ({ n: i + 1, ar, en, id: ID_NAMES[i], ayahs }))

/** At-Tawbah has no basmala, and Al-Fatihah's basmala is its first ayah. */
export const hasBasmala = (n: number) => n !== 1 && n !== 9

/** Strips harakat, Qur'anic marks and tatweel, and unifies alef, ya, ta marbuta and hamza seats. */
export const normalizeArabic = (s: string) =>
  s
    .replace(/[\u0610-\u061A\u064B-\u065F\u0670\u06D6-\u06ED\u08D3-\u08FF\u0640]/g, '')
    .replace(/[أإآٱ]/g, 'ا')
    .replace(/ى/g, 'ي')
    .replace(/ة/g, 'ه')
    .replace(/ؤ/g, 'و')
    .replace(/ئ/g, 'ي')
    .replace(/\s+/g, ' ')
    .trim()

/** The Shu'bah edition writes the alef (or alef maksura) carrying an open tanwin as its own word, "هُدࣰ ى":
 *  joined back so its words line up with the Hafs ones. */
export const joinTanwin = (s: string) => s.replace(/([ࣰ-ࣲ])\s+(?=[اى])/g, '$1')

const ARTICLE = new Set(['al', 'an', 'ad', 'at', 'as', 'ash', 'az', 'ar', 'adh', 'ath', 'el'])
/** Latin words without articles, apostrophes, doubled letters or a final h, with the common spelling variants
 *  folded together, English and Indonesian alike: "Al-Ma'idah" → ["maida"], "Al-Burooj" → ["buruj"],
 *  "Quraysh" and "Quraisy" → ["quraish"], "Ash-Shams" and "Asy-Syams" → ["shams"], "Al-Kawthar", "Al-Kautsar"
 *  and "Al-Kausar" → ["kausar"], "Adh-Dhariyat" and "Adz-Dzariyat" → ["zariat"]. */
export const latinWords = (s: string) =>
  s
    .normalize('NFKD')
    .replace(/\p{M}/gu, '')
    .toLowerCase()
    .replace(/['’ʿʾ`]/g, '')
    // Indonesian (Kemenag) and English spellings of the same Arabic letters: ش sy/sh, ذ dz/dh/z, ث ts/th/s, غ g/gh
    .replace(/sy/g, 'sh')
    .replace(/dz|dh/g, 'z')
    .replace(/ts|th/g, 's')
    .replace(/gh/g, 'g')
    .replace(/aw/g, 'au')
    .replace(/ee/g, 'i')
    .replace(/oo|ou/g, 'u')
    .replace(/y/g, 'i')
    .replace(/[^a-z]+/g, ' ')
    .split(' ')
    .filter((w) => w && !ARTICLE.has(w))
    .map((w) => w.replace(/(.)\1+/g, '$1').replace(/h$/, ''))

/** True when a and b differ by at most one inserted, removed or changed letter. */
function oneEdit(a: string, b: string): boolean {
  if (Math.abs(a.length - b.length) > 1) return false
  let i = 0
  while (i < a.length && i < b.length && a[i] === b[i]) i++
  const [x, y] = [a.slice(i + 1), b.slice(i + 1)]
  return x === y || a.slice(i) === y || a.slice(i + 1) === b.slice(i)
}

const AR_ALIASES: [string, number][] = [['الانشراح', 94], ['اللهب', 111], ['الدهر', 76], ['بني اسراييل', 17], ['المومن', 40], ['براءه', 9]]
const LAT_ALIASES: [string, number][] = [['Lahab', 111], ['Inshirah', 94], ['Dahr', 76], ['Bani Israil', 17], ["Al-Mu'min", 40], ['Taha', 20]]
const arNames: [string, number][] = [...SURAHS.map((s): [string, number] => [normalizeArabic(s.ar), s.n]), ...AR_ALIASES]
const latNames = [...SURAHS.flatMap((s): [string, number][] => [[s.en, s.n], [s.id, s.n]]), ...LAT_ALIASES].map(([en, n]) => ({ n, words: latinWords(en) }))

// "a selection from Surah …": only some ayahs are recited, so it can't be synced automatically
const PARTIAL = /ما تيسر|مقتطف|ايات من|excerpt|selection|verses from/
// albums, artists and folders that mean Qur'an recitation (in latinWords / normalizeArabic spelling)
// murotal: Indonesian "murottal"; muzamil, bahanan, taqi: reciters well known in Indonesia
const QURAN_HINT = /quran|koran|mushaf|juz|tilawa|recitation|murata|murotal|mujawad|ngaji|afasi|mishari|sudais|shuraim|gamdi|minshawi|husari|basit|muaiqli|muzamil|bahanan|taqi malik|مصحف|القران|قران|جزء|تلاوه|مرتل|مجود|العفاسي|السديس|الحصري|المنشاوي|عبد الباسط/

/** Surah number named in Arabic: after "سورة" anywhere, or, in a Qur'an album, as the title's first word(s). */
function arabicSurah(text: string, bare: boolean): number | null {
  const s = normalizeArabic(text)
  const lead = s.length - s.replace(/^[\d\s\-_.]+/, '').length
  let best: [number, number] | null = null
  for (const [name, n] of arNames) {
    for (const key of bare ? [`سوره ${name}`, name] : [`سوره ${name}`]) {
      const found = s.indexOf(key)
      // without "سورة" the name must open the title, so "دعاء ليلة القدر" stays a dua
      const i = key === name && found !== lead ? -1 : found
      // a whole word: "سورة الحج" must not match inside "سورة الحجرات"
      if (i >= 0 && !/[\u0621-\u064A]/.test(s[i + key.length] ?? '') && (!best || name.length > best[1])) best = [n, name.length]
    }
  }
  return best?.[0] ?? null
}

/** Surah number named in Latin script: after "surah/surat/sura", or bare when `bare`. */
function latinSurah(text: string, bare: boolean): number | null {
  const words = latinWords(text)
  // "QS. Yasin" (Qur'an Surah) is how Indonesian titles name one
  const at = words.findIndex((w) => w === 'sura' || w === 'surat' || w === 'qs')
  if (at < 0 && !bare) return null
  const rest = at < 0 ? words : words.slice(at + 1)
  const line = ` ${rest.join(' ')} `
  let best: [number, number] | null = null
  for (const { n, words: name } of latNames) {
    const phrase = name.join(' ')
    // without the keyword the name must open the title ("Ad-Duha - ibiidris"), so "Lailatul Qadr Dua" stays a dua
    const whole = at < 0 ? line.startsWith(` ${phrase} `) : line.includes(` ${phrase} `)
    // "Ya-Sin" written "Yasin"; only for long names, so "nas" never matches inside "nasheed"
    const glued = name.join('').length >= 5 && (at < 0 ? rest[0] === name.join('') : rest.includes(name.join('')))
    // "Surah Imran" for "Ali 'Imran": the name's last word, after the keyword only
    const last = at >= 0 && name.length > 1 && name[name.length - 1].length >= 4 && rest[0] === name[name.length - 1]
    const len = phrase.length
    if ((whole || glued || last) && (!best || len > best[1])) best = [n, len]
  }
  if (best) return best[0]
  // one-letter spelling slips in a longer one-word name ("Kawthur", "Mao'un"), as the first word only
  const first = rest[0] ?? ''
  const near = latNames.filter(({ words: name }) => name.length === 1 && name[0].length >= 4 && first.length >= 4 && oneEdit(first, name[0]))
  // one surah, perhaps under both its English and Kemenag names
  return new Set(near.map((x) => x.n)).size === 1 ? near[0].n : null
}

/** Whole-juz tracks whose juz starts exactly at a surah: Juz 'Amma (30) and Juz Tabarak (29). */
function juz(text: string): number[] | null {
  const ar = normalizeArabic(text)
  const lat = ` ${latinWords(text).join(' ')} `
  const span = (a: number, b: number) => Array.from({ length: b - a + 1 }, (_, i) => a + i)
  if (/جزء عم(\s|$)/.test(ar) || / juz (ama|30) /.test(lat)) return span(78, 114)
  if (/جزء تبارك/.test(ar) || / juz (tabarak|29) /.test(lat)) return span(67, 77)
  return null
}

export type Riwayah = 'hafs' | 'shubah'
/** partial: only a selection of the surah is recited, so ayahs are synced by tapping, not automatically. */
export interface Recitation { surahs: number[]; riwayah: Riwayah; partial: boolean }
type TrackLike = { title: string; album: string; artist: string; albumArtist: string; path: string; track: number | null }

/** The surah(s) a track recites, or null for anything else. Titles win over track numbers. */
export function detectRecitation(t: TrackLike): Recitation | null {
  const file = t.path.split(/[\\/]/).pop()?.replace(/\.[^.]+$/, '') ?? ''
  const context = `${t.album} ${t.artist} ${t.albumArtist} ${t.path}`
  const hint = QURAN_HINT.test(`${normalizeArabic(context)} ${latinWords(context).join(' ')}`)
  const all = `${t.title} ${t.album} ${file}`
  const words = latinWords(all).join(' ')
  const riwayah: Riwayah = /شعبه/.test(normalizeArabic(all)) || /(^| )(shuba|shouba)( |$)/.test(words) ? 'shubah' : 'hafs'
  const partial = PARTIAL.test(`${normalizeArabic(all)} ${words}`)

  const whole = juz(t.title) ?? juz(file)
  if (whole) return { surahs: whole, riwayah, partial }
  for (const text of [t.title, file]) {
    const n = arabicSurah(text, hint) ?? latinSurah(text, hint)
    if (n) return { surahs: [n], riwayah, partial }
  }
  // a numbered mushaf album: track N is surah N
  if (/مصحف|mushaf/i.test(normalizeArabic(t.album)) && t.track && t.track >= 1 && t.track <= 114) return { surahs: [t.track], riwayah, partial }
  return null
}

// ---- ayah sync ----

export type Pause = [number, number]
/** One stretch of recitation: an ayah, or a flexible opening (isti'adha, basmala) that may be absent. */
export interface Unit { weight: number; flexible?: boolean }

/** How the aligner weighs things (tuned on Alafasy murattal with known timings, see quran.test.ts). */
export const TUNING = { length: 3, pause: 0.7, miss: 1.2 }
/** Prefer the longer pauses that end an ayah over the short ones at a waqf inside it. */
const pauseCost = (len: number) => TUNING.pause * Math.max(0, 1 - len / 1.0)
const lengthCost = (actual: number, expected: number, flexible?: boolean) =>
  actual <= 0 ? (flexible ? 0 : 50) : TUNING.length * Math.log(actual / expected) ** 2 * (flexible ? 0.3 : 1)

/**
 * The start time of each unit. One pause is chosen per boundary so each unit's length stays close to
 * its expected share (`weight`, e.g. the murattal verse length) of the recitation. Anchors (unit index
 * → time, from tapping "starts here") fix a boundary to the pause next to the tap.
 */
export function alignAyahs(units: Unit[], pauses: Pause[], duration: number, anchors: Record<number, number> = {}): number[] {
  const n = units.length
  if (!n) return []
  // the recitation itself, without the silence before and after it
  let from = 0
  let to = duration
  const inner: Pause[] = []
  for (const [s, e] of pauses) {
    if (s <= 0.05) from = Math.max(from, e)
    else if (e >= duration - 0.05) to = Math.min(to, s)
    else inner.push([s, e])
  }
  const span = Math.max(0.01, to - from)
  const total = units.reduce((s, u) => s + u.weight, 0) || 1
  const expectedLen = units.map((u) => (span * u.weight) / total)
  const expectedStart: number[] = []
  units.reduce((at, u, i) => ((expectedStart[i] = at), at + expectedLen[i]), from)

  // a tap with no pause nearby becomes a boundary of its own
  const cands: Pause[] = [...inner]
  for (const t of Object.values(anchors)) if (!inner.some(([, e]) => Math.abs(e - t) <= 1.5)) cands.push([t, t])
  cands.sort((a, b) => a[1] - b[1])
  if (!cands.length || n === 1) return expectedStart.map((x) => Math.round(x * 100) / 100)

  // boundary b is the start of unit b; it may only use pauses near where it is expected
  const band = Math.max(20, 0.1 * span)
  const windows: number[][] = [[]]
  for (let b = 1; b < n; b++) {
    const anchor = anchors[b]
    let w: number[]
    if (anchor !== undefined) {
      let best = 0
      cands.forEach(([, e], j) => Math.abs(e - anchor) < Math.abs(cands[best][1] - anchor) && (best = j))
      w = [best]
    } else {
      w = cands.flatMap(([, e], j) => (Math.abs(e - expectedStart[b]) <= band ? [j] : []))
      if (!w.length) {
        let best = 0
        cands.forEach(([, e], j) => Math.abs(e - expectedStart[b]) < Math.abs(cands[best][1] - expectedStart[b]) && (best = j))
        w = [best]
      }
      // while everything before it is optional, a unit may start right at the opening (-1)
      if (units.slice(0, b).every((x) => x.flexible)) w = [-1, ...w]
    }
    windows.push(w)
  }

  // dp[b][k]: cheapest way for unit b to start at windows[b][k] (-1 = the opening). A step of 2 or 3
  // skips boundaries that have no pause of their own (ayahs joined in one breath, wasl); those are
  // placed inside the speech in proportion to the template.
  const MISS = TUNING.miss
  const startOf = (j: number) => (j < 0 ? from : cands[j][1])
  // prefix sums, so the inner loop never re-adds expected lengths or re-checks flags and taps
  const sumLen = [0]
  const sumFixed = [0] // units that are not optional
  const sumTapped = [0]
  for (let i = 0; i < n; i++) {
    sumLen.push(sumLen[i] + expectedLen[i])
    sumFixed.push(sumFixed[i] + (units[i].flexible ? 0 : 1))
    sumTapped.push(sumTapped[i] + (anchors[i] !== undefined ? 1 : 0))
  }
  const share = (a: number, b: number) => sumLen[b] - sumLen[a]
  const optional = (a: number, b: number) => sumFixed[b] === sumFixed[a]
  windows[0] = [-1]
  const dp: Float64Array[] = [Float64Array.of(0)]
  const back: { k: number; step: number }[][] = [[]]
  for (let b = 1; b < n; b++) {
    const w = windows[b]
    dp[b] = new Float64Array(w.length).fill(Infinity)
    back[b] = []
    for (let jj = 0; jj < w.length; jj++) {
      const j = w[jj]
      const end = j < 0 ? from : cands[j][0]
      const pc = j < 0 ? 0 : pauseCost(cands[j][1] - cands[j][0])
      let best = Infinity
      let arg = { k: -1, step: 1 }
      for (let step = 1; step <= Math.min(3, b); step++) {
        const a = b - step
        // a tapped start is never skipped over
        if (step > 1 && sumTapped[b] - sumTapped[a + 1] > 0) break
        const expected = share(a, b)
        const opt = optional(a, b)
        const prev = windows[a]
        const prevCost = dp[a]
        for (let k = 0; k < prev.length; k++) {
          const i = prev[k]
          const zero = i === j && step === 1 && units[a].flexible
          if (!(i < j || zero)) continue
          const before = prevCost[k]
          if (before === Infinity) continue
          const c = before + (zero ? 0 : lengthCost(end - startOf(i), expected, opt) + pc) + MISS * (step - 1)
          if (c < best) (best = c), (arg = { k, step })
        }
      }
      dp[b][jj] = best
      back[b].push(arg)
    }
  }
  // the last unit(s) run to the end of the recitation
  let bestTotal = Infinity
  let last = { b: 0, k: 0 }
  for (let a = n - 1; a >= Math.max(0, n - 3); a--) {
    if (a < n - 1 && sumTapped[n] - sumTapped[a + 1] > 0) break
    windows[a].forEach((i, k) => {
      const c = dp[a][k] + lengthCost(to - startOf(i), share(a, n), optional(a, n)) + MISS * (n - 1 - a)
      if (c < bestTotal) (bestTotal = c), (last = { b: a, k })
    })
  }
  if (!Number.isFinite(bestTotal)) return expectedStart.map((x) => Math.round(x * 100) / 100)

  const starts = new Array<number>(n)
  // boundaries after `a` up to (not including) `b` share the stretch from a's start to `end`
  const fill = (a: number, b: number, end: number) => {
    const t0 = starts[a]
    for (let x = a + 1; x < b; x++) starts[x] = t0 + ((end - t0) * share(a, x)) / share(a, b)
  }
  let { b: cur, k } = last
  starts[cur] = startOf(windows[cur][k])
  fill(cur, n, to)
  while (cur > 0) {
    const { k: pk, step } = back[cur][k]
    const prev = cur - step
    starts[prev] = startOf(windows[prev][pk])
    const j = windows[cur][k]
    fill(prev, cur, j < 0 ? from : cands[j][0])
    ;[cur, k] = [prev, pk]
  }
  return starts.map((x) => Math.round(x * 100) / 100)
}
