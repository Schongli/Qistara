// Qur'an mode's own words in English and Indonesian (the rest of the app is English for now). The language
// follows the system unless Settings › Lyrics › Qur'an says otherwise.

export type QLang = 'en' | 'id'

export const QURAN_TEXT = {
  en: {
    latin: 'Latin', translit: 'Transliteration', fix: 'Fix sync', fixHint: 'Tap the ayah being recited to sync it', forget: 'Forget the ayahs you tapped',
    tapNow: 'Tap the ayah being recited right now. Esc cancels.', loading: 'Loading the surah…', analysing: 'Finding the ayahs in this recording (once per surah)…',
    stream: "A stream can't be scanned for the pauses between ayahs", selection: "This is a selection of ayahs, so it can't be synced on its own",
    useFix: 'use Fix sync on the ayah being recited.', shubah: "Shu'bah text from the King Fahd Complex; word meanings follow Hafs where the words match.",
    pickTitle: 'What does this recording recite?', find: 'Surah name or number', riwayah: 'Riwayah', partial: 'Only some ayahs (sync by tapping)',
    auto: 'Automatic', autoHint: 'Let Qistara work it out from the name', none: 'Not a Recitation', cancel: 'Cancel', save: 'Save',
    juzAmma: "Juz 'Amma", juzTabarak: 'Juz Tabarak',
  },
  id: {
    latin: 'Latin', translit: 'Transliterasi', fix: 'Perbaiki sinkron', fixHint: 'Ketuk ayat yang sedang dibaca untuk menyelaraskannya', forget: 'Lupakan ayat yang sudah diketuk',
    tapNow: 'Ketuk ayat yang sedang dibaca sekarang. Esc untuk batal.', loading: 'Memuat surah…', analysing: 'Mencari letak ayat dalam rekaman ini (sekali per surah)…',
    stream: 'Siaran tidak bisa dipindai untuk mencari jeda antarayat', selection: 'Ini hanya sebagian ayat, jadi tidak bisa diselaraskan otomatis',
    useFix: 'pakai Perbaiki sinkron pada ayat yang sedang dibaca.', shubah: "Teks riwayat Syu'bah dari Kompleks Raja Fahd; arti kata mengikuti Hafs bila katanya sama.",
    pickTitle: 'Rekaman ini membaca surah apa?', find: 'Nama atau nomor surah', riwayah: 'Riwayat', partial: 'Hanya sebagian ayat (selaraskan dengan mengetuk)',
    auto: 'Otomatis', autoHint: 'Biarkan Qistara menebak dari judulnya', none: 'Bukan bacaan Al-Qur’an', cancel: 'Batal', save: 'Simpan',
    juzAmma: "Juz 'Amma", juzTabarak: 'Juz Tabarak',
  },
} satisfies Record<QLang, Record<string, string>>

/** The language Qur'an mode speaks: the setting, or the system's when it's Automatic. */
export const quranLang = (setting: string, system = typeof navigator === 'undefined' ? 'en' : navigator.language): QLang =>
  setting === 'id' || setting === 'en' ? setting : system.toLowerCase().startsWith('id') ? 'id' : 'en'
