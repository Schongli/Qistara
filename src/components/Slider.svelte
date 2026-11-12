<script lang="ts">
  // Pointer-driven slider: tracks the pointer 1:1 from pointer-down, keeps tracking outside
  // its bounds (pointer capture) and only commits on release unless `live` is set.
  let { value, max = 1, label, live = false, onchange, class: cls = '' }: {
    value: number
    max?: number
    label: string
    live?: boolean
    onchange: (v: number) => void
    class?: string
  } = $props()

  let el: HTMLDivElement
  let drag = $state<number | null>(null)
  const shown = $derived(drag ?? value)
  const pct = $derived(max > 0 ? Math.min(100, Math.max(0, (shown / max) * 100)) : 0)

  const at = (e: PointerEvent) => {
    const r = el.getBoundingClientRect()
    return Math.min(1, Math.max(0, (e.clientX - r.left) / r.width)) * max
  }
  function down(e: PointerEvent) {
    if (e.button !== 0) return
    el.setPointerCapture(e.pointerId)
    drag = at(e)
    if (live) onchange(drag)
  }
  function moveTo(e: PointerEvent) {
    if (drag === null) return
    drag = at(e)
    if (live) onchange(drag)
  }
  function up() {
    if (drag === null) return
    onchange(drag)
    drag = null
  }
  function key(e: KeyboardEvent) {
    const step = max / (e.shiftKey ? 10 : 50)
    if (e.key === 'ArrowRight' || e.key === 'ArrowUp') onchange(Math.min(max, value + step))
    else if (e.key === 'ArrowLeft' || e.key === 'ArrowDown') onchange(Math.max(0, value - step))
    else return
    e.preventDefault()
    e.stopPropagation()
  }
</script>

<div
  bind:this={el}
  class="slider {cls}"
  class:dragging={drag !== null}
  role="slider"
  tabindex="0"
  aria-label={label}
  aria-valuemin={0}
  aria-valuemax={max}
  aria-valuenow={Math.round(shown)}
  onpointerdown={down}
  onpointermove={moveTo}
  onpointerup={up}
  onpointercancel={() => (drag = null)}
  onkeydown={key}
>
  <div class="track"><div class="fill" style:width="{pct}%"></div></div>
  <div class="knob" style:left="{pct}%"></div>
</div>

<style>
  .slider { position: relative; height: 16px; display: flex; align-items: center; touch-action: none; }
  .track { position: relative; width: 100%; height: 4px; border-radius: 2px; background: var(--fill-2); overflow: hidden; transition: height 140ms var(--ease-out); }
  .fill { height: 100%; background: var(--text-2); }
  .slider:hover .track, .dragging .track { height: 6px; }
  .knob {
    position: absolute; top: 50%; width: 12px; height: 12px; margin: -6px 0 0 -6px; border-radius: 50%;
    background: #fff; box-shadow: 0 0 0 0.5px rgba(0, 0, 0, 0.2), 0 1px 3px rgba(0, 0, 0, 0.3);
    opacity: 0; transform: scale(0.6); transition: opacity 140ms, transform 140ms var(--ease-out);
  }
  .slider:hover .knob, .dragging .knob, .slider:focus-visible .knob { opacity: 1; transform: scale(1); }
  .slider:focus-visible { outline-offset: 4px; }
</style>
