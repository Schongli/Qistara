// Translates the shown lyrics with Google Translate's free endpoint. Only the lyric lines are sent;
// results are cached per lyrics and language, so a song is translated once.
import { invoke } from '@tauri-apps/api/core'
import { untrack } from 'svelte'
import { lyr } from './lyrics.svelte'
import { settings } from './settings.svelte'
import { kvGet, kvSet } from './persist'
import { align, alignSegments, batches, busyFor, hash, mapBack, parseGtx, sameLanguage, targetFor, uniqueLines } from './translate'
import type { Lyrics } from './lyrics'

/**
 * lines: one translation per lyric line (null = nothing to show). same: the lyrics are already in the target language.
 * source: the language Google detected (Learn mode looks words up in it).
 */
export const tr = $state({ lines: null as (string | null)[] | null, same: false, source: '', busy: false, error: '' })

export const targetLanguage = () => settings.lyricsLanguage || targetFor(navigator.language)

type Cached = { source: string; dict: Record<string, string> }

// Google's free endpoint turns bursts away (HTTP 429): one request at a time, at least a second apart. When it says
// it's busy, wait a little (2 s, then 4 s) and try again; after that leave it alone for as long as it asked.
const GAP = 1000
const BUSY = 'Google Translate is busy right now. Qistara tries again by itself in a moment.'
let last = 0
let chain: Promise<unknown> = Promise.resolve()
let coolUntil = 0
const sleep = (ms: number) => new Promise((r) => setTimeout(r, ms))
/** Milliseconds until Google may be asked again (0 = now). */
export const googleRest = () => Math.max(0, coolUntil - Date.now())

/**
 * Google's parsed reply. The request goes out from Rust (translate.rs): Google refuses the HTTP plugin's HTTP/2
 * requests with 429. Learn mode's word lookups use this too, so they share the pacing.
 */
export function gtx(text: string, from: string, to: string, dictionary: boolean): Promise<unknown> {
  const run = async () => {
    for (let attempt = 0; ; attempt++) {
      if (googleRest()) throw new Error(BUSY)
      await sleep(Math.max(0, last + GAP - Date.now()))
      last = Date.now()
      try {
        return JSON.parse(await invoke<string>('google_translate', { text, from, to, dictionary }))
      } catch (e) {
        const wait = busyFor(e)
        if (wait === null) throw e
        if (attempt === 2) {
          coolUntil = Date.now() + wait * 1000
          throw new Error(BUSY)
        }
        await sleep(2 ** (attempt + 1) * 1000)
      }
    }
  }
  const p = chain.then(run, run)
  chain = p.catch(() => {})
  return p
}

/** One request per batch. A reply that can't be lined up is asked again in halves, never line by line. */
async function translateBatch(b: string[], tl: string, c: Cached): Promise<void> {
  const j = await gtx(b.join('\n'), 'auto', tl, false)
  const res = parseGtx(j)
  c.source ||= res.source
  const out = alignSegments(b, j) ?? align(b, res.text)
  if (out) return b.forEach((line, i) => (c.dict[line] = out[i]))
  if (b.length === 1) return void (c.dict[b[0]] = res.text.trim())
  const half = Math.ceil(b.length / 2)
  await translateBatch(b.slice(0, half), tl, c)
  await translateBatch(b.slice(half), tl, c)
}

async function translate(texts: string[], tl: string): Promise<Cached> {
  const c: Cached = { source: '', dict: {} }
  for (const b of batches(texts)) await translateBatch(b, tl, c)
  return c
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
    // Google was busy: try this song again once it may be asked
    if (googleRest()) setTimeout(() => mine === seq && run(data, on, tl), googleRest() + 500)
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
