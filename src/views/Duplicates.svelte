<script lang="ts">
  import { revealItemInDir } from '@tauri-apps/plugin-opener'
  import FolderOpen from 'phosphor-svelte/lib/FolderOpen'
  import Play from 'phosphor-svelte/lib/Play'
  import { lib } from '../lib/library.svelte'
  import { findDuplicates } from '../lib/dupes'
  import { queue } from '../lib/queue.svelte'
  import { plural, quality } from '../lib/format'

  const groups = $derived(findDuplicates(lib.tracks))
  const extra = $derived(groups.reduce((n, g) => n + g.length - 1, 0))
</script>

<h1 class="t-title">Duplicates</h1>
<p class="t-sub">
  {#if groups.length}
    {groups.length === 1 ? '1 song exists' : `${groups.length} songs exist`} in more than one file ({extra} extra {extra === 1 ? 'copy' : 'copies'}). The
    best-quality copy is listed first. Sonora never deletes files; use Show in File Explorer to tidy up.
  {:else}
    No duplicate songs found. Songs match when artist, title and length agree (remaster and version tags are ignored).
  {/if}
</p>

{#each groups as g (g[0].id)}
  <section class="group">
    <div class="head"><b class="ellipsis">{g[0].title}</b><span class="t-sub ellipsis">{g[0].albumArtist || g[0].artist}</span></div>
    {#each g as t, i (t.id)}
      {@const q = quality(t)}
      <div class="row">
        <span class="tag" class:best={i === 0}>{i === 0 ? 'Best' : 'Copy'}</span>
        <span class="q">{q.detail || t.codec}</span>
        <span class="path t-sub ellipsis" title={t.path}>{t.path}</span>
        <button class="icon-btn" aria-label="Play this copy" onclick={() => queue.play([t])}><Play size={14} weight="fill" /></button>
        <button class="icon-btn" aria-label="Show in File Explorer" title="Show in File Explorer" onclick={() => revealItemInDir(t.path)}><FolderOpen size={15} /></button>
      </div>
    {/each}
  </section>
{/each}

<style>
  h1 { margin: 0 0 6px; }
  p { margin: 0 0 20px; max-width: 760px; line-height: 1.5; }
  .group { max-width: 980px; margin-bottom: 14px; padding: 10px 12px; border-radius: 10px; background: var(--fill); }
  .head { display: flex; gap: 10px; align-items: baseline; margin-bottom: 6px; min-width: 0; }
  .row { display: grid; grid-template-columns: 44px 170px 1fr auto auto; gap: 12px; align-items: center; height: 32px; }
  .tag { font-size: 10px; font-weight: 700; text-transform: uppercase; letter-spacing: 0.04em; color: var(--text-3); }
  .tag.best { color: #30d158; }
  .q { font-variant-numeric: tabular-nums; }
  .path { min-width: 0; user-select: text; }
</style>
