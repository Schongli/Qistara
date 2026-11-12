// Translates the shown lyrics with Google Translate's free endpoint. Only the lyric lines are sent;
// results are cached per lyrics and language, so a song is translated once.
import { invoke } from '@tauri-apps/api/core'
import { untrack } from 'svelte'
import { lyr } from './lyrics.svelte'
import { settings } from './settings.svelte'
import { kvGet, kvSet } from './persist'
import { align, batches, hash, mapBack, parseGtx, sameLanguage, targetFor, uniqueLines } from './translate'
import type { Lyrics } from './lyrics'

/**
 * lines: one translation per lyric line (null = nothing to show). same: the lyrics are already in the target language.
 * source: the language Google detected (Learn mode looks words up in it).
 */
export const tr = $state({ lines: null as (string | null)[] | null, same: false, source: '', busy: false, error: '' })

export const targetLanguage = () => settings.lyricsLanguage || targetFor(navigator.language)

type Cached = { source: string; dict: Record<string, string> }

// the request goes out from Rust (translate.rs): Google refuses the HTTP plugin's HTTP/2 requests with 429
async function google(texts: string[], tl: string) {
  const raw = await invoke<string>('google_translate', { text: texts.join('\n'), from: 'auto', to: tl, dictionary: false })
  return parseGtx(JSON.parse(raw))
}

async function translate(texts: string[], tl: string): Promise<Cached> {
  const dict: Record<string, string> = {}
  let source = ''
  for (const b of batches(texts)) {
    const res = await google(b, tl)
    source ||= res.source
    let out = align(b, res.text)
    if (!out) {
      // the reply merged or split lines: ask line by line so each translation stays with its line
      out = []
      for (const line of b) out.push((await google([line], tl)).text.trim())
    }
    b.forEach((line, i) => (dict[line] = out[i]))
  }
  return { source, dict }
}

let seq = 0
async function run(data: Lyrics | null, on: boolean, tl: string) {
  const mine = ++seq
  Object.assign(tr, { lines: null, same: false, source: '', busy: false, error: '' })
  if (!on || !data) return
  const lines = data.lines.map((l) => l.text)
  const texts = uniqueLines(lines)
  if (!texts.length) return
  tr.busy = true
  try {
    const key = `tr:${tl}:${hash(texts.join('\n'))}`
    let c = await kvGet<Cached>(key)
    if (!c) {
      c = await translate(texts, tl)
      kvSet(key, c)
    }
    if (mine !== seq) return // the song, lyrics or language changed meanwhile
    tr.source = c.source
    tr.same = sameLanguage(c.source, tl)
    tr.lines = tr.same ? null : mapBack(lines, c.dict)
  } catch (e) {
    console.warn('lyrics translation failed:', e) // the button tooltip shows it too, but only on hover
    if (mine === seq) tr.error = e instanceof Error ? e.message : String(e)
  } finally {
    if (mine === seq) tr.busy = false
  }
}

$effect.root(() => {
  $effect(() => {
    const data = lyr.data
    const on = settings.translateLyrics || settings.learnLyrics // Learn mode always shows the translation
    const tl = targetLanguage()
    untrack(() => run(data, on, tl))
  })
})
