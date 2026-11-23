// The translation engine (Bergamot, Firefox Translations' WebAssembly build of Marian), in a classic worker so the
// Emscripten glue can be loaded with importScripts. The page (mt.svelte.ts) hands in the glue and wasm URLs, then
// models as raw buffers; this side only translates. Follows @browsermt/bergamot-translator's own
// worker/translator-worker.js, which can't be used as is (it loads its files by name next to itself).

/* eslint-disable @typescript-eslint/no-explicit-any */
type Glue = any
declare const self: { Module: Glue; importScripts(...urls: string[]): void; postMessage(m: unknown, transfer?: Transferable[]): void; onmessage: ((e: MessageEvent) => void) | null }

// int8 matrix routines the wasm imports from 'wasm_gemm': its own fallbacks, under the names it expects
const GEMM: Record<string, string> = {
  int8_prepare_a: 'int8PrepareAFallback',
  int8_prepare_b: 'int8PrepareBFallback',
  int8_prepare_b_from_transposed: 'int8PrepareBFromTransposedFallback',
  int8_prepare_b_from_quantized_transposed: 'int8PrepareBFromQuantizedTransposedFallback',
  int8_prepare_bias: 'int8PrepareBiasFallback',
  int8_multiply_and_add_bias: 'int8MultiplyAndAddBiasFallback',
  int8_select_columns_of_b: 'int8SelectColumnsOfBFallback',
}

// the package's defaults for these models, then what it always overrides
const CONFIG = [
  'beam-size: 1', 'normalize: 1.0', 'word-penalty: 0', 'cpu-threads: 0', 'gemm-precision: int8shiftAlphaAll', 'skip-cost: true',
  'alignment: soft', 'quiet: true', 'quiet-translation: true', 'max-length-break: 128', 'mini-batch-words: 1024',
  'workspace: 128', 'max-length-factor: 2.0',
].join('\n')

let mod: Glue
let service: Glue
const models = new Map<string, Glue>()

async function init(glue: string, wasm: string) {
  if (mod) return
  const bytes = await (await fetch(wasm)).arrayBuffer()
  mod = await new Promise((resolve, reject) => {
    const Module: Glue = {
      instantiateWasm(info: WebAssembly.Imports, accept: (i: WebAssembly.Instance) => void) {
        const gemm = Object.fromEntries(Object.entries(GEMM).map(([k, name]) => [k, (...a: unknown[]) => Module.asm[name](...a)]))
        WebAssembly.instantiate(bytes, { ...info, wasm_gemm: gemm }).then(({ instance }) => accept(instance), reject)
        return {}
      },
      onRuntimeInitialized: () => resolve(Module),
    }
    self.Module = Module
    self.importScripts(glue)
  })
  service = new mod.BlockingService({ cacheSize: 0 })
}

function aligned(buf: ArrayBuffer, alignment: number) {
  const bytes = new Int8Array(buf)
  const m = new mod.AlignedMemory(bytes.byteLength, alignment)
  m.getByteArrayView().set(bytes)
  return m
}

function load(pair: string, b: { model: ArrayBuffer; lex: ArrayBuffer; vocabs: ArrayBuffer[] }) {
  if (models.has(pair)) return
  const vocabs = new mod.AlignedMemoryList()
  b.vocabs.forEach((v) => vocabs.push_back(aligned(v, 64)))
  models.set(pair, new mod.TranslationModel(CONFIG, aligned(b.model, 256), aligned(b.lex, 64), vocabs, null))
}

/** One output per input text; `pairs` is one model, or two through English. */
function translate(pairs: string[], texts: string[]): string[] {
  const input = new mod.VectorString()
  const options = new mod.VectorResponseOptions()
  for (const t of texts) {
    input.push_back(t)
    options.push_back({ alignment: false, html: false, qualityScores: false })
  }
  const ms = pairs.map((p) => models.get(p))
  if (ms.some((m) => !m)) throw new Error('model not loaded')
  const res = ms.length > 1 ? service.translateViaPivoting(ms[0], ms[1], input, options) : service.translate(ms[0], input, options)
  const out = texts.map((_, i) => res.get(i).getTranslatedText())
  input.delete()
  options.delete()
  res.delete()
  return out
}

const API: Record<string, (...a: any[]) => unknown> = { init, load, has: (pair: string) => models.has(pair), translate }

self.onmessage = async ({ data: { id, name, args } }: MessageEvent) => {
  try {
    self.postMessage({ id, result: await API[name](...args) })
  } catch (e) {
    self.postMessage({ id, error: e instanceof Error ? e.message : String(e) })
  }
}
