<script lang="ts">
  // Three bars that move while playing, a still icon when paused (or with reduced motion).
  let { paused }: { paused: boolean } = $props()
</script>

<span class="eq" class:paused aria-label={paused ? 'Paused' : 'Now playing'}><i></i><i></i><i></i></span>

<style>
  .eq { display: inline-flex; align-items: flex-end; gap: 2px; width: 12px; height: 11px; }
  i { flex: 1; background: var(--accent); border-radius: 1px; height: 40%; animation: b 0.9s ease-in-out infinite alternate; }
  i:nth-child(2) { animation-delay: -0.45s; height: 90%; }
  i:nth-child(3) { animation-delay: -0.2s; height: 60%; }
  .paused i { animation-play-state: paused; }
  @keyframes b { from { height: 25%; } to { height: 100%; } }
  @media (prefers-reduced-motion: reduce) { i { animation: none; } }
</style>
