<script lang="ts">
  // The Qistara pivot Q without its tile. The pivot dot turns signal-cyan while playback is verified bit-perfect.
  import { signal } from '../lib/signal.svelte'

  let { size = 22, live = true }: { size?: number; live?: boolean } = $props()
  const id = $props.id()
  const pure = $derived(live && signal.verdict.bitPerfect)
</script>

<svg width={size} height={size} viewBox="170 170 700 700" class:pure aria-hidden="true">
  <mask id="cut-{id}"><rect x="170" y="170" width="700" height="700" fill="#fff" /><path d="M631.9 631.9L728.1 728.1" stroke="#000" stroke-width="156" /></mask>
  <circle cx="482" cy="482" r="280" fill="none" stroke="var(--brand)" stroke-width="96" mask="url(#cut-{id})" />
  <path d="M650.3 582.4L862.4 794.5L862.4 808.7L808.7 862.4L794.5 862.4L582.4 650.3Z" fill="var(--brand)" />
  <circle cx="482" cy="482" r="77" class="dot" />
</svg>

<style>
  svg { flex: none; display: block; }
  .dot { fill: var(--brand-2); transition: fill 400ms var(--ease-out); }
  .pure .dot { fill: var(--signal-pure); }
</style>
