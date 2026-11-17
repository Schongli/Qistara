<script lang="ts">
  import { SYSTEM } from '../lib/os'
  import { open } from '@tauri-apps/plugin-dialog'
  import { invoke } from '@tauri-apps/api/core'
  import Icon from '../components/Icon.svelte'
  import Toggle from '../components/Toggle.svelte'
  import { eq, eqRate, savePreset, userPresets } from '../lib/eq.svelte'
  import { clampBand, freqs, hasGain, LIMITS, MAX_BANDS, parseApo, PRESETS, preampOf, response, total, TYPES, type Band } from '../lib/eq'
  import { settings } from '../lib/settings.svelte'

  const RANGE = 18 // ±dB shown; a band can go to ±24 and is then pinned to the edge
  // a band takes the colour of its region (sub-bass coral … air violet); the combined curve is the brass accent
  const bandColor = (f: number) => (f < 80 ? '#fb7185' : f < 400 ? '#fbbf24' : f < 2500 ? '#34d399' : f < 6000 ? '#38bdf8' : '#a78bfa')
  let w = $state(800)
  const h = 300
  let sel = $state(0)
  let err = $state('')
  // the curve is drawn at the rate that plays: shelves near Nyquist differ between 44.1 and 96 kHz
  const fs = $derived(eqRate())

  const x = (f: number) => (Math.log10(f / 20) / 3) * w
  const fAt = (px: number) => Math.min(20000, Math.max(20, 20 * Math.pow(1000, px / w)))
  const y = (g: number) => h / 2 - (g / RANGE) * (h / 2)
  const gAt = (py: number) => ((h / 2 - py) / (h / 2)) * RANGE
  const pin = (py: number) => Math.min(h - 10, Math.max(10, py))

  const F = freqs(200)
  const path = $derived(F.map((f, i) => `${i ? 'L' : 'M'}${x(f).toFixed(1)},${y(total(eq, f, fs) + preampOf(eq, fs)).toFixed(1)}`).join(''))
  const bandPath = (b: Band) => F.map((f, i) => `${i ? 'L' : 'M'}${x(f).toFixed(1)},${y(response(b, f, fs)).toFixed(1)}`).join('')
  const nodeY = (b: Band) => y(hasGain(b.type) ? b.g : response(b, b.f, fs))
  const pre = $derived(preampOf(eq, fs))
  const hz = (f: number) => (f >= 1000 ? `${+(f / 1000).toFixed(f >= 10000 ? 1 : 2)} kHz` : `${Math.round(f)} Hz`)
  const db = (g: number) => `${g > 0 ? '+' : ''}${g.toFixed(1)} dB`

  /** Every edit goes through clampBand: an emptied field or a wild number never reaches mpv. */
  function edit(i: number, patch: Partial<Band>) {
    eq.bands[i] = clampBand({ ...eq.bands[i], ...patch }, fs, eq.bands[i])
  }
  const numIn = (e: Event) => (e.currentTarget as HTMLInputElement).valueAsNumber

  // direct manipulation: the node follows the pointer 1:1 from where it was grabbed; Shift+drag changes Q
  let svg: SVGSVGElement
  let drag: { i: number; dx: number; dy: number; q: boolean; y0: number; q0: number } | null = null
  let hover = $state<{ px: number; py: number } | null>(null)
  function down(e: PointerEvent, i: number) {
    e.stopPropagation()
    sel = i
    const r = svg.getBoundingClientRect()
    const b = eq.bands[i]
    drag = { i, dx: e.clientX - r.left - x(b.f), dy: e.clientY - r.top - nodeY(b), q: e.shiftKey, y0: e.clientY, q0: b.q }
    ;(e.currentTarget as Element).setPointerCapture(e.pointerId)
    eq.enabled = true
  }
  function move(e: PointerEvent) {
    const r = svg.getBoundingClientRect()
    if (!drag) return void (hover = { px: e.clientX - r.left, py: e.clientY - r.top })
    const b = eq.bands[drag.i]
    // up widens the bell's peak (higher Q), down broadens it; one doubling per 60 px
    if (drag.q) return edit(drag.i, { q: +(drag.q0 * Math.pow(2, (drag.y0 - e.clientY) / 60)).toFixed(2) })
    edit(drag.i, { f: Math.round(fAt(e.clientX - r.left - drag.dx)), ...(hasGain(b.type) && { g: +gAt(e.clientY - r.top - drag.dy).toFixed(1) }) })
  }
  const endDrag = () => (drag = null)
  function wheel(e: WheelEvent, i: number) {
    e.preventDefault()
    edit(i, { q: +(eq.bands[i].q * Math.pow(1.1, -e.deltaY / 100)).toFixed(2) })
  }
  function addBand(f = 2000, g = 0) {
    if (eq.bands.length >= MAX_BANDS) return
    eq.bands.push(clampBand({ type: 'peak', f, g, q: 1, on: true }, fs))
    sel = eq.bands.length - 1
    eq.enabled = true
  }
  function removeBand(i: number) {
    if (eq.bands.length <= 1) return
    eq.bands.splice(i, 1)
    sel = Math.min(sel, eq.bands.length - 1)
  }
  /** Double-click empty space: a new bell right there. */
  function dbl(e: MouseEvent) {
    if ((e.target as Element).closest('.node')) return
    const r = svg.getBoundingClientRect()
    addBand(Math.round(fAt(e.clientX - r.left)), +Math.max(-LIMITS.gain, Math.min(LIMITS.gain, gAt(e.clientY - r.top))).toFixed(1))
  }
  function key(e: KeyboardEvent, i: number) {
    const b = eq.bands[i]
    const step = e.shiftKey ? 3 : 0.5
    if (e.key === 'ArrowUp' && hasGain(b.type)) edit(i, { g: b.g + step })
    else if (e.key === 'ArrowDown' && hasGain(b.type)) edit(i, { g: b.g - step })
    else if (e.key === 'ArrowRight') edit(i, { f: Math.round(b.f * 1.06) })
    else if (e.key === 'ArrowLeft') edit(i, { f: Math.round(b.f / 1.06) })
    else if (e.key === 'Delete' || e.key === 'Backspace') removeBand(i)
    else return
    e.preventDefault()
  }

  function applyPreset(name: string) {
    const bands = PRESETS[name] ?? userPresets[name]
    if (!bands) return
    eq.bands = structuredClone($state.snapshot(bands)).map((b: Band) => clampBand(b, fs))
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
  const readout = $derived(hover && !drag ? { f: fAt(hover.px), g: total(eq, fAt(hover.px), fs) + pre } : null)
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
  <svg bind:this={svg} width={w} height={h} onpointermove={move} onpointerup={endDrag} onpointercancel={endDrag} onlostpointercapture={endDrag}
    onpointerleave={() => (hover = null)} ondblclick={dbl} role="img" aria-label="EQ response curve, {fs / 1000} kHz">
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
    {#if readout && hover}
      <line x1={hover.px} x2={hover.px} y1="0" y2={h} class="cursor" />
      <text x={Math.min(hover.px + 8, w - 110)} y="16" class="read">{hz(readout.f)} · {db(readout.g)}</text>
    {/if}
    {#each eq.bands as b, i (i)}
      {@const ny = nodeY(b)}
      <g
        class="node"
        class:sel={i === sel}
        class:disabled={!b.on}
        class:pinned={pin(ny) !== ny}
        role="slider"
        tabindex="0"
        aria-label="Band {i + 1}, {TYPES.find(([t]) => t === b.type)?.[1]}"
        aria-valuemin={-LIMITS.gain}
        aria-valuemax={LIMITS.gain}
        aria-valuenow={b.g}
        aria-valuetext="{hz(b.f)}, {hasGain(b.type) ? db(b.g) : 'no gain'}, Q {b.q}"
        onpointerdown={(e) => down(e, i)}
        onwheel={(e) => wheel(e, i)}
        onkeydown={(e) => key(e, i)}
        onfocus={() => (sel = i)}
      >
        <circle cx={x(b.f)} cy={pin(ny)} r="14" class="hit" />
        <circle cx={x(b.f)} cy={pin(ny)} r="8" style:fill={bandColor(b.f)} />
        <text x={x(b.f)} y={pin(ny) + 3.5} class="n">{i + 1}</text>
      </g>
    {/each}
  </svg>
</div>
<p class="t-sub hint">Drag a point for frequency and gain · Shift+drag or scroll on it for width (Q) · double-click to add a band · Delete removes the selected one · curve at {fs / 1000} kHz.</p>

<div class="table" role="table" aria-label="Bands">
  <div class="tr th" role="row">
    <span role="columnheader">Band</span><span role="columnheader">Type</span><span role="columnheader">Frequency</span>
    <span role="columnheader">Gain</span><span role="columnheader">Q</span><span role="columnheader">On</span><span></span>
  </div>
  {#each eq.bands as b, i (i)}
    <div class="tr" class:sel={i === sel} role="row" tabindex="-1" onfocusin={() => (sel = i)} onclick={() => (sel = i)} onkeydown={() => {}}>
      <span class="idx" role="cell"><i style:background={bandColor(b.f)}></i>{i + 1}</span>
      <span role="cell"><select value={b.type} onchange={(e) => edit(i, { type: e.currentTarget.value as Band['type'] })} aria-label="Band {i + 1} type">{#each TYPES as [t, l] (t)}<option value={t}>{l}</option>{/each}</select></span>
      <span role="cell"><input type="number" min="20" max="20000" step="1" value={b.f} onchange={(e) => edit(i, { f: numIn(e) })} aria-label="Band {i + 1} frequency in Hz" /> <small>Hz</small></span>
      <span role="cell" class:muted={!hasGain(b.type)}><input type="number" min={-LIMITS.gain} max={LIMITS.gain} step="0.1" value={b.g} disabled={!hasGain(b.type)} onchange={(e) => edit(i, { g: numIn(e) })} aria-label="Band {i + 1} gain in dB" /> <small>dB</small></span>
      <span role="cell"><input type="number" min={LIMITS.q[0]} max={LIMITS.q[1]} step="0.01" value={b.q} onchange={(e) => edit(i, { q: numIn(e) })} aria-label="Band {i + 1} Q" /></span>
      <span role="cell"><input type="checkbox" checked={b.on} onchange={(e) => edit(i, { on: e.currentTarget.checked })} aria-label="Band {i + 1} on" /></span>
      <span role="cell"><button class="icon-btn" aria-label="Remove band {i + 1}" onclick={() => removeBand(i)} disabled={eq.bands.length <= 1}><Icon name="Trash" size={14} /></button></span>
    </div>
  {/each}
  {#if eq.bands.length < MAX_BANDS}
    <button class="add press" onclick={() => addBand()}><Icon name="Plus" size={12} weight="bold" />Add Band</button>
  {/if}
</div>

<div class="pre">
  <label class="on"><input type="checkbox" bind:checked={eq.autoPreamp} /> Automatic preamp</label>
  {#if eq.autoPreamp}
    <span class="t-sub">{pre.toFixed(1)} dB: lowered by the largest boost so the EQ can't clip.</span>
  {:else}
    <label>Preamp <input type="number" min="-24" max="6" step="0.1" value={eq.preamp} onchange={(e) => (eq.preamp = Number.isFinite(numIn(e)) ? Math.min(6, Math.max(-24, numIn(e))) : eq.preamp)} /> dB</label>
  {/if}
</div>

<style>
  h1 { margin: 0 0 16px; }
  .top { display: flex; flex-wrap: wrap; align-items: center; gap: 10px; margin-bottom: 14px; }
  .master { flex: 1 1 420px; border-radius: var(--r-lg); background: var(--fill); }
  select, input[type='number'] { height: 28px; padding: 0 8px; border-radius: var(--r-sm); border: 1px solid var(--sep); background: var(--bg-2); color: var(--text); font-variant-numeric: tabular-nums; }
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
  .cursor { stroke: var(--text-3); stroke-dasharray: 3 3; pointer-events: none; }
  .read { fill: var(--text); font-size: 11px; font-weight: 600; font-variant-numeric: tabular-nums; pointer-events: none; }
  .node { cursor: grab; }
  .node:active { cursor: grabbing; }
  .node .hit { fill: transparent; }
  .node circle:not(.hit) { stroke: var(--bg); stroke-width: 2; transition: r 120ms; }
  .node.sel circle:not(.hit) { r: 10; stroke: var(--text); }
  .node.pinned circle:not(.hit) { stroke-dasharray: 3 2; }
  .node.disabled { opacity: 0.35; }
  .n { fill: #000; font-size: 10px; font-weight: 700; text-anchor: middle; pointer-events: none; }
  .node:focus-visible { outline: none; }
  .node:focus-visible circle:not(.hit) { stroke: var(--accent); stroke-width: 3; }
  .hint { margin: 8px 2px 14px; }
  .table { display: grid; gap: 2px; padding: 6px; border-radius: var(--r-lg); background: var(--fill); margin-bottom: 10px; overflow-x: auto; }
  .tr { display: grid; grid-template-columns: 64px 128px 130px 124px 96px 44px 36px; align-items: center; gap: 8px; min-height: 38px; padding: 0 8px; border-radius: var(--r); min-width: 620px; }
  .tr.sel { background: var(--fill-2); }
  .tr.th { min-height: 26px; font-size: 11px; font-weight: 650; letter-spacing: 0.04em; text-transform: uppercase; color: var(--text-2); }
  .tr > span { display: flex; align-items: center; gap: 6px; min-width: 0; }
  .tr small { color: var(--text-3); }
  .tr .muted { opacity: 0.45; }
  .idx { font-weight: 650; font-variant-numeric: tabular-nums; }
  .idx i { width: 9px; height: 9px; border-radius: 50%; }
  .tr select { width: 100%; }
  .add { display: flex; align-items: center; gap: 6px; justify-self: start; height: 30px; margin: 4px 8px; padding: 0 10px; border-radius: var(--r); color: var(--text-2); font-weight: 600; }
  .add:hover { background: var(--fill-2); color: var(--text); }
  .pre { display: flex; flex-wrap: wrap; align-items: center; gap: 16px; padding: 12px 14px; border-radius: var(--r-lg); background: var(--fill); margin-bottom: 10px; }
  label { display: flex; align-items: center; gap: 8px; color: var(--text-2); }
  .on { color: var(--text); }
  .err { color: var(--signal-veil); }
  @media (prefers-reduced-motion: reduce) { .graph, .node circle:not(.hit) { transition: none; } }
</style>
