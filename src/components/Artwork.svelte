<script lang="ts">
  import MusicNotes from 'phosphor-svelte/lib/MusicNotes'
  import { artUrl } from '../lib/library.svelte'

  let { art, src = null, size = '100%', radius = 'var(--r-art)', shadow = true }: {
    art: string | null
    src?: string | null // a full URL (large cover) wins over the cached thumbnail
    size?: string
    radius?: string
    shadow?: boolean
  } = $props()

  const url = $derived(src ?? artUrl(art))
  let failed = $state(false)
  $effect(() => {
    url
    failed = false
  })
</script>

<div class="art" class:shadow style:width={size} style:border-radius={radius}>
  {#if url && !failed}
    <img src={url} alt="" loading="lazy" decoding="async" draggable="false" onerror={() => (failed = true)} />
  {:else}
    <div class="ph"><MusicNotes size="38%" weight="light" /></div>
  {/if}
</div>

<style>
  .art { aspect-ratio: 1; overflow: hidden; background: var(--fill-2); flex: none; position: relative; }
  .art.shadow { box-shadow: var(--shadow-art); }
  img { width: 100%; height: 100%; object-fit: cover; }
  .ph {
    width: 100%; height: 100%; display: grid; place-items: center; color: var(--text-3);
    background: linear-gradient(145deg, var(--fill), var(--fill-2));
  }
</style>
