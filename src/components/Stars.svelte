<script lang="ts">
  // Five-star rating (MusicBee/iTunes): hover previews, click sets, clicking the same star clears.
  import Star from 'phosphor-svelte/lib/Star'
  let { value, onchange }: { value: number; onchange: (v: number) => void } = $props()
  let hover = $state(0)
</script>

<span class="stars" role="radiogroup" aria-label="Rating" tabindex="-1" onpointerleave={() => (hover = 0)}>
  {#each [1, 2, 3, 4, 5] as n (n)}
    <button
      role="radio"
      aria-checked={value === n}
      aria-label="{n} star{n > 1 ? 's' : ''}"
      class:on={(hover || value) >= n}
      onpointerenter={() => (hover = n)}
      onclick={(e) => { e.stopPropagation(); onchange(value === n ? 0 : n) }}
    ><Star size={11} weight={(hover || value) >= n ? 'fill' : 'regular'} /></button>
  {/each}
</span>

<style>
  .stars { display: inline-flex; gap: 1px; color: var(--text-3); }
  button { display: grid; padding: 5px 2px; color: inherit; } /* larger hit area than the 11px glyph */
  button.on { color: var(--text-2); }
</style>
