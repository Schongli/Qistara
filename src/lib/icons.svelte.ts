// Icon packs: Phosphor (the default, and every icon's fallback) is always here; the others load the first time
// they're chosen or previewed, so they don't weigh on startup.
import { PHOSPHOR, type Weights } from './icon-data'
import { settings } from './settings.svelte'

const loaders = import.meta.glob<{ PACK: Record<string, Weights> }>('./icon-packs/*.ts')
export const packs = $state<Record<string, Record<string, Weights>>>({ phosphor: PHOSPHOR })

export function usePack(name: string) {
  const load = loaders[`./icon-packs/${name}.ts`]
  if (!packs[name] && load) load().then((m) => (packs[name] = m.PACK), (e) => console.warn(`icon pack ${name}:`, e))
}

$effect.root(() => {
  $effect(() => usePack(settings.iconPack))
})
