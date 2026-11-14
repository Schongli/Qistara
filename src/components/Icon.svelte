<script lang="ts">
  // An icon from the pack chosen in Settings › Appearance. Names and weights are Phosphor's, which the app was
  // drawn with; scripts/icon-map.json says what each one is in the other packs.
  import type { SVGAttributes } from 'svelte/elements'
  import { ICONS } from '../lib/icon-data'
  import { settings } from '../lib/settings.svelte'

  // pack: a fixed pack instead of the chosen one (the previews in Settings)
  let { name, size = '1em', weight = 'regular', pack, ...rest }: { name: string; size?: number | string; weight?: string; pack?: string } & SVGAttributes<SVGSVGElement> = $props()

  const glyph = $derived.by(() => {
    const w = ICONS[pack ?? settings.iconPack]?.[name] ?? ICONS.phosphor[name]
    return w && ((weight === 'fill' && w.fill) || (weight === 'bold' && w.bold) || w.regular)
  })
</script>

{#if glyph}
  <svg width={size} height={size} viewBox="0 0 {glyph[0]} {glyph[1]}" class:solid={glyph[3]} aria-hidden="true" {...rest}>{@html glyph[2]}</svg>
{/if}

<style>
  svg { flex: none; display: inline-block; vertical-align: middle; }
  /* packs without filled icons (Lucide): the "on" state fills the outline */
  .solid :global(path) { fill: currentColor; }
</style>
