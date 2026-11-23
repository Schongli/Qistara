// Shares the playing track's lyrics with notch (a separate top-of-screen app) through a local file
// (`notch_lyrics` in lib.rs): synced lines, their translation, word timing, or the ayahs of a recitation.
// Nothing goes over the network. notch reads it only while this track is what Windows' media controls show.
import { invoke } from '@tauri-apps/api/core'
import { untrack } from 'svelte'
import { lyr } from './lyrics.svelte'
import { tr } from './translate.svelte'
import { BASMALA, ISTIADHA, quran, type TranslationId } from './quran.svelte'
import { queue } from './queue.svelte'
import { settings } from './settings.svelte'
import { lyricLines, quranLines, type NotchLyrics, type RecitedUnit } from './notch'

function share() {
  const t = queue.current
  // Mid-search there is nothing to say yet; notch keeps what it has until the answer.
  if (!t || lyr.loading || quran.status === 'loading' || quran.status === 'analysing') return
  const reciting = quran.status === 'ready' && quran.starts
  const translation = settings.quranTranslations[0] as TranslationId | undefined
  const units: RecitedUnit[] = quran.units.map((u) =>
    u.kind === 'istiadha' ? { text: ISTIADHA }
    : u.kind === 'basmala' ? { text: BASMALA }
    : {
        text: u.ayah.ar,
        tr: translation ? u.ayah.tr[translation] : undefined,
        // word times exist only for quran.com's own recording
        words: u.ayah.words?.every((w) => w.seg) ? u.ayah.words.map((w) => ({ t: w.seg![0], end: w.seg![1], text: w.ar })) : undefined,
      },
  )
  const data: NotchLyrics = {
    v: 1,
    title: t.title,
    artist: t.artist,
    duration: t.duration,
    rtl: !!reciting,
    lines: reciting ? quranLines(units, quran.starts, t.duration) : lyricLines(lyr.data, tr.lines),
  }
  invoke('notch_lyrics', { json: JSON.stringify(data) }).catch((e) => console.warn('notch lyrics:', e))
}

$effect.root(() => {
  $effect(() => {
    void [queue.current, lyr.data, lyr.loading, tr.lines, quran.status, quran.starts, quran.units, settings.quranTranslations[0]]
    untrack(share)
  })
})
