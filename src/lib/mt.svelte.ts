// Translate on this PC: the language packs (translate_local.rs) and the engine that runs them (mt.worker.ts).
// Packs are downloaded once from Mozilla; translating sends nothing anywhere.
import { invoke } from '@tauri-apps/api/core'
import { listen } from '@tauri-apps/api/event'
import glueUrl from '@browsermt/bergamot-translator/worker/bergamot-translator-worker.js?url'
import wasmUrl from '@browsermt/bergamot-translator/worker/bergamot-translator-worker.wasm?url'
import { route, type Pair } from './mt'

export interface Pack extends Pair { bytes: number }
/** Thrown by translateLocal when a pack is missing (translate_local.rs uses the same words). */
export const MISSING = 'translation-pack-missing'

export const mt = $state({
  installed: [] as Pack[],
  /** what Mozilla offers, read once it's needed */
  catalog: null as Pack[] | null,
  /** "ja-en" while that pack downloads */
  installing: '',
  progress: 0,
  error: '',
})

export const refreshPacks = () => invoke<Pack[]>('mt_status').then((p) => (mt.installed = p), () => {})
void refreshPacks()

export async function loadCatalog(): Promise<Pack[]> {
  mt.catalog ??= await invoke<Pack[]>('mt_catalog')
  return mt.catalog
}

/** The packs a translation still needs, or null when Mozilla has no way between the two languages. */
export async function needed(from: string, to: string): Promise<Pack[] | null> {
  const path = route(from, to, await loadCatalog())
  if (!path) return null
  return path.filter((p) => !mt.installed.some((i) => i.from === p.from && i.to === p.to)).map((p) => mt.catalog!.find((c) => c.from === p.from && c.to === p.to)!)
}

export async function installPack(p: Pair) {
  mt.installing = `${p.from}-${p.to}`
  mt.progress = 0
  mt.error = ''
  const off = await listen<{ file: number; files: number; done: number; total: number }>('mt-progress', ({ payload: x }) => {
    mt.progress = Math.min(1, (x.file + (x.total ? x.done / x.total : 0)) / Math.max(1, x.files))
  })
  try {
    await invoke('mt_install', { from: p.from, to: p.to })
    await refreshPacks()
  } catch (e) {
    mt.error = String(e)
    throw e
  } finally {
    off()
    mt.installing = ''
  }
}

/** Installs the packs one after another (a song through English needs two). */
export async function installAll(packs: Pair[]) {
  for (const p of packs) await installPack(p)
}

export async function removePack(p: Pair) {
  stop() // the engine may hold its model
  await invoke('mt_remove', { from: p.from, to: p.to })
  await refreshPacks()
}

// ---- the engine ----

let worker: Worker | null = null
let ready: Promise<unknown> | null = null
let seq = 0
const pending = new Map<number, { res: (v: unknown) => void; rej: (e: Error) => void }>()

function call<T>(name: string, args: unknown[], transfer: Transferable[] = []): Promise<T> {
  const id = ++seq
  return new Promise<T>((res, rej) => {
    pending.set(id, { res: res as (v: unknown) => void, rej })
    worker!.postMessage({ id, name, args }, transfer)
  })
}

function engine(): Promise<unknown> {
  if (!worker) {
    worker = new Worker(new URL('./mt.worker.ts', import.meta.url), { type: 'classic' })
    worker.onmessage = ({ data: { id, result, error } }) => {
      const p = pending.get(id)
      pending.delete(id)
      if (error) p?.rej(new Error(error))
      else p?.res(result)
    }
    ready = call('init', [new URL(glueUrl, location.href).href, new URL(wasmUrl, location.href).href])
  }
  return ready!
}

/** Frees the engine and its models (they take memory while loaded); the next translation starts it again. */
function stop() {
  worker?.terminate()
  worker = ready = null
  for (const p of pending.values()) p.rej(new Error('stopped'))
  pending.clear()
}

async function ensure(p: Pair) {
  const pair = `${p.from}-${p.to}`
  if (await call<boolean>('has', [pair])) return
  const get = (kind: string) => invoke<ArrayBuffer>('mt_file', { from: p.from, to: p.to, kind })
  const [model, lex] = await Promise.all([get('model'), get('lex')])
  const vocabs = await get('vocab').then((v) => [v], () => Promise.all([get('srcvocab'), get('trgvocab')]))
  await call('load', [pair, { model, lex, vocabs }], [model, lex, ...vocabs])
}

/** Lines translated on this PC, one per line. Throws MISSING when a pack for this pair isn't installed. */
export async function translateLocal(lines: string[], from: string, to: string): Promise<string[]> {
  const path = route(from, to, mt.installed)
  if (!path) throw new Error(MISSING)
  if (!path.length) return lines
  await engine()
  for (const p of path) await ensure(p)
  return call<string[]>('translate', [path.map((p) => `${p.from}-${p.to}`), lines])
}

/** Whether a translation between the two could run here now. */
export const canTranslateLocally = (from: string, to: string) => !!route(from, to, mt.installed)
