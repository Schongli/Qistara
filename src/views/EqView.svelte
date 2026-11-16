<script lang="ts">
  import { SYSTEM } from '../lib/os'
  import { open } from '@tauri-apps/plugin-dialog'
  import { invoke } from '@tauri-apps/api/core'
  import Icon from '../components/Icon.svelte'
  import Toggle from '../components/Toggle.svelte'
  import { eq, savePreset, userPresets } from '../lib/eq.svelte'
  import { freqs, hasGain, MAX_BANDS, parseApo, PRESETS, preampOf, response, total, TYPES, type Band } from '../lib/eq'
  import { settings } from '../lib/settings.svelte'

  const RANGE = 18 // ±dB shown
  // a band takes the colour of its region (sub-bass coral … air violet); the combined curve is the brass accent
  const bandColor = (f: number) => (f < 80 ? '#fb7185' : f < 400 ? '#fbbf24' : f < 2500 ? '#34d399' : f < 6000 ? '#38bdf8' : '#a78bfa')
  let w = $state(800)
  const h = 300
  let sel = $state(0)
  let err = $state('')

  const x = (f: number) => (Math.log10(f / 20) / 3) * w
  const fAt = (px: number) => Math.min(20000, Math.max(20, 20 * Math.pow(1000, px / w)))
  const y = (g: number) => h / 2 - (g / RANGE) * (h / 2)
  const gAt = (py: number) => Math.min(RANGE, Math.max(-RANGE, ((h / 2 - py) / (h / 2)) * RANGE))

  const F = freqs(200)
  const path = $derived(F.map((f, i) => `${i ? 'L' : 'M'}${x(f).toFixed(1)},${y(total(eq, f) + preampOf(eq)).toFixed(1)}`).join(''))
  const bandPath = (b: Band) => F.map((f, i) => `${i ? 'L' : 'M'}${x(f).toFixed(1)},${y(response(b, f)).toFixed(1)}`).join('')
  const nodeY = (b: Band) => y(hasGain(b.type) ? b.g : response(b, b.f))
  const pre = $derived(preampOf(eq))
  const band = $derived(eq.bands[sel])

  // direct manipulation: the node follows the pointer 1:1 from where it was grabbed
  let svg: SVGSVGElement
  let drag: { i: number; dx: number; dy: number } | null = null
  function down(e: PointerEvent, i: number) {
    sel = i
    const r = svg.getBoundingClientRect()
    const b = eq.bands[i]
    drag = { i, dx: e.clientX - r.left - x(b.f), dy: e.clientY - r.top - nodeY(b) }
    ;(e.currentTarget as Element).setPointerCapture(e.pointerId)
    eq.enabled = true
  }
  function moveNode(e: PointerEvent) {
    if (!drag) return
    const r = svg.getBoundingClientRect()
    const b = eq.bands[drag.i]
    b.f = Math.round(fAt(e.clientX - r.left - drag.dx))
    if (hasGain(b.type)) b.g = +gAt(e.clientY - r.top - drag.dy).toFixed(1)
  }
  function wheel(e: WheelEvent, i: number) {
    e.preventDefault()
    const b = eq.bands[i]
    b.q = +Math.min(10, Math.max(0.1, b.q * Math.pow(1.1, -e.deltaY / 100))).toFixed(2)
  }
  function addBand() {
    eq.bands.push({ type: 'peak', f: 2000, g: 0, q: 1, on: true })
    sel = eq.bands.length - 1
  }
  function applyPreset(name: string) {
    const bands = PRESETS[name] ?? userPresets[name]
    if (!bands) return
    eq.bands = structuredClone(bands)
    eq.enabled = true
    sel = 0
  }
  async function importApo() {
    err = ''
    const f = await open({ multiple: false, filters: [{ name: 'Equalizer APO / AutoEQ', extensions: ['txt'] }] })
    if (typeof f !== 'string') return
    const parsed = parseApo(await invoke<string>('read_text', { path: f }).catch((e) => ((err = String(e)), '')))
    if (!parsed) return void (err ||= 'No “Filter: ON PK Fc … Hz Gain … dB Q …” lines in that file.')
    Object.assign(eq, parsed)
    sel = 0
  }
  function save() {
    const name = prompt('Preset name')
    if (name?.trim()) savePreset(name.trim())
  }
  const gridF = [20, 50, 100, 200, 500, 1000, 2000, 5000, 10000, 20000]
  const label = (f: number) => (f >= 1000 ? `${f / 1000}k` : String(f))
</script>

<h1 class="t-title">Equalizer</h1>
<div class="top">
  <div class="group master"><Toggle bind:checked={eq.enabled} label="Parametric EQ" hint={settings.bitPerfect ? 'Exclusive mode and the native sample rate stay on, but EQ changes the samples, so playback is no longer bit-perfect.' : `Shapes the sound before it reaches the ${SYSTEM} mixer.`} /></div>
  <select onchange={(e) => (applyPreset(e.currentTarget.value), (e.currentTarget.value = ''))} aria-label="Presets">
    <option value="">Presets…</option>
    {#each Object.keys(PRESETS) as p (p)}<option value={p}>{p}</option>{/each}
    {#if Object.keys(userPresets).length}<option disabled>──────────</option>{/if}
    {#each Object.keys(userPresets) as p (p)}<option value={p}>{p}</option>{/each}
  </select>
  <button class="btn press" onclick={save}>Save Preset…</button>
  <button class="btn press" onclick={importApo}>Import AutoEQ…</button>
</div>
{#if err}<p class="err">{err}</p>{/if}

<div class="graph" class:off={!eq.enabled} bind:clientWidth={w}>
  <svg bind:this={svg} width={w} height={h} onpointermove={moveNode} onpointerup={() => (drag = null)} role="img" aria-label="EQ response curve">
    {#each gridF as f (f)}
      <line x1={x(f)} x2={x(f)} y1="0" y2={h} class="grid" />
      <text x={x(f) + 4} y={h - 6} class="lbl">{label(f)}</text>
    {/each}
    {#each [-12, -6, 0, 6, 12] as g (g)}
      <line x1="0" x2={w} y1={y(g)} y2={y(g)} class="grid" class:zero={g === 0} />
      <text x="4" y={y(g) - 4} class="lbl">{g > 0 ? '+' : ''}{g} dB</text>
    {/each}
    {#each eq.bands as b, i (i)}
      {#if b.on}<path d={bandPath(b)} class="band" style:stroke={bandColor(b.f)} class:dim={i !== sel} />{/if}
    {/each}
    <path d={path} class="curve" />
    {#each eq.bands as b, i (i)}
      <g
        class="node"
        class:sel={i === sel}
        class:disabled={!b.on}
        role="slider"
        tabindex="0"
        aria-label="Band {i + 1}: {b.f} Hz, {b.g} dB, Q {b.q}"
        aria-valuenow={b.g}
        onpointerdown={(e) => down(e, i)}
        onwheel={(e) => wheel(e, i)}
        onkeydown={(e) => {
          const step = e.shiftKey ? 3 : 0.5
          if (e.key === 'ArrowUp' && hasGain(b.type)) b.g = Math.min(RANGE, b.g + step)
          else if (e.key === 'ArrowDown' && hasGain(b.type)) b.g = Math.max(-RANGE, b.g - step)
          else if (e.key === 'ArrowRight') b.f = Math.min(20000, Math.round(b.f * 1.06))
          else if (e.key === 'ArrowLeft') b.f = Math.max(20, Math.round(b.f / 1.06))
          else return
          e.preventDefault()
        }}
      >
        <circle cx={x(b.f)} cy={nodeY(b)} r="14" class="hit" />
        <circle cx={x(b.f)} cy={nodeY(b)} r="8" style:fill={bandColor(b.f)} />
        <text x={x(b.f)} y={nodeY(b) + 3.5} class="n">{i + 1}</text>
      </g>
    {/each}
  </svg>
</div>
<p class="t-sub hint">Drag a point to change frequency and gain · scroll on it for width (Q) · arrow keys nudge the selected band.</p>

<div class="bands">
  {#each eq.bands as b, i (i)}
    <button class="chip" class:sel={i === sel} style:--c={bandColor(b.f)} onclick={() => (sel = i)}><i></i>{i + 1}</button>
  {/each}
  {#if eq.bands.length < MAX_BANDS}<button class="chip add" onclick={addBand} aria-label="Add band"><Icon name="Plus" size={12} weight="bold" /></button>{/if}
</div>

{#if band}
  <div class="detail">
    <label>Type
      <select bind:value={band.type}>{#each TYPES as [t, l] (t)}<option value={t}>{l}</option>{/each}</select>
    </label>
    <label>Frequency <input type="number" min="20" max="20000" bind:value={band.f} /> Hz</label>
    <label class:muted={!hasGain(band.type)}>Gain <input type="number" min={-RANGE} max={RANGE} step="0.1" bind:value={band.g} disabled={!hasGain(band.type)} /> dB</label>
    <label>Q <input type="number" min="0.1" max="10" step="0.01" bind:value={band.q} /></label>
    <label class="on"><input type="checkbox" bind:checked={band.on} /> On</label>
    <button class="icon-btn" aria-label="Remove band" onclick={() => { eq.bands.splice(sel, 1); sel = Math.max(0, sel - 1) }} disabled={eq.bands.length <= 1}><Icon name="Trash" size={15} /></button>
  </div>
{/if}

<div class="pre">
  <label class="on"><input type="checkbox" bind:checked={eq.autoPreamp} /> Automatic preamp</label>
  {#if eq.autoPreamp}
    <span class="t-sub">{pre.toFixed(1)} dB — lowered by the largest boost so the EQ can't clip.</span>
  {:else}
    <label>Preamp <input type="number" min="-24" max="6" step="0.1" bind:value={eq.preamp} /> dB</label>
  {/if}
</div>

<style>
  h1 { margin: 0 0 16px; }
  .top { display: flex; flex-wrap: wrap; align-items: center; gap: 10px; margin-bottom: 14px; }
  .master { flex: 1 1 420px; border-radius: var(--r-lg); background: var(--fill); }
  select, input[type='number'] { height: 28px; padding: 0 8px; border-radius: var(--r-sm); border: 1px solid var(--sep); background: var(--bg-2); color: var(--text); }
  input[type='number'] { width: 84px; }
  .graph { border-radius: var(--r-lg); background: var(--fill); overflow: hidden; touch-action: none; transition: opacity 200ms; }
  .graph.off { opacity: 0.55; }
  svg { display: block; }
  .grid { stroke: var(--sep); stroke-width: 1; }
  .grid.zero { stroke: var(--text-3); }
  .lbl { fill: var(--text-3); font-size: 10px; }
  .band { fill: none; stroke-width: 1.5; opacity: 0.45; }
  .band.dim { opacity: 0.18; }
  .curve { fill: none; stroke: var(--accent); stroke-width: 2.5; stroke-linejoin: round; }
  .node { cursor: grab; }
  .node:active { cursor: grabbing; }
  .node .hit { fill: transparent; }
  .node circle:not(.hit) { stroke: var(--bg); stroke-width: 2; transition: r 120ms; }
  .node.sel circle:not(.hit) { r: 10; stroke: var(--text); }
  .node.disabled { opacity: 0.35; }
  .n { fill: #000; font-size: 10px; font-weight: 700; text-anchor: middle; pointer-events: none; }
  .node:focus-visible { outline: none; }
  .node:focus-visible circle:not(.hit) { stroke: var(--accent); stroke-width: 3; }
  .hint { margin: 8px 2px 14px; }
  .bands { display: flex; gap: 6px; margin-bottom: 12px; }
  .chip { display: flex; align-items: center; gap: 6px; height: 26px; padding: 0 10px; border-radius: 13px; background: var(--fill); font-weight: 600; font-variant-numeric: tabular-nums; }
  .chip i { width: 9px; height: 9px; border-radius: 50%; background: var(--c); }
  .chip.sel { background: var(--fill-2); box-shadow: inset 0 0 0 1.5px var(--c); }
  .detail, .pre { display: flex; flex-wrap: wrap; align-items: center; gap: 16px; padding: 12px 14px; border-radius: var(--r-lg); background: var(--fill); margin-bottom: 10px; }
  label { display: flex; align-items: center; gap: 8px; color: var(--text-2); }
  label.muted { opacity: 0.45; }
  .on { color: var(--text); }
  .err { color: #e5484d; }
</style>
