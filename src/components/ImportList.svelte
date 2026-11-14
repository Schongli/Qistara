<script lang="ts">
  // Pick playlists from a service and import them as playlists of your own files (Spotify, Apple Music, Tidal).
  import Icon from './Icon.svelte'
  import type { ImportResult } from '../lib/imports.svelte'
  import { nav } from '../lib/nav.svelte'
  import { plural } from '../lib/format'

  type Row = { id: string; name: string; tracks: number; image: string | null; all?: boolean }
  let { rows, results, importing, loading, service, onimport, onreload }: {
    rows: Row[]
    results: Record<string, ImportResult>
    importing: string
    loading: boolean
    service: string
    onimport: (id: string, name: string) => Promise<void>
    onreload: () => void
  } = $props()

  let picked = $state<string[]>([])
  const toggle = (id: string) => (picked = picked.includes(id) ? picked.filter((x) => x !== id) : [...picked, id])
  async function importPicked() {
    for (const r of rows.filter((r) => picked.includes(r.id))) await onimport(r.id, r.name)
    picked = []
  }
</script>

<section>
  <div class="title-row">
    <h2 class="t-h2">Import Playlists</h2>
    <button class="icon-btn press" aria-label="Reload playlists" title="Reload playlists" disabled={loading} onclick={onreload}><Icon name="ArrowsClockwise" size={15} /></button>
  </div>
  <p class="t-sub">Makes a Qistara playlist of your own files for each {service} playlist, so it plays in full quality. Songs you don't have are listed, never downloaded. Importing again updates it.</p>
  <div class="list">
    {#each rows as r (r.id)}
      {@const res = results[r.id]}
      <div class="row">
        <input type="checkbox" checked={picked.includes(r.id)} onchange={() => toggle(r.id)} aria-label="Import {r.name}" />
        <span class="thumb">
          {#if r.image}<img src={r.image} alt="" />{:else if r.all}<Icon name="Heart" size={16} weight="fill" />{:else}<Icon name="Playlist" size={16} />{/if}
        </span>
        <span class="txt">
          <span class="ellipsis">{r.name}</span>
          {#if r.tracks}<span class="t-sub">{plural(r.tracks, 'song')}</span>{/if}
        </span>
        <span class="res t-sub">
          {#if importing === r.id}
            Importing…
          {:else if res?.error}
            <span class="err">{res.error}</span>
          {:else if res}
            <button class="link" onclick={() => res.playlist && nav.root({ name: 'playlist', id: res.playlist })}>{res.found} of {res.total} found</button>
          {/if}
        </span>
      </div>
      {#if res?.missing.length}
        <details class="missing">
          <summary class="t-sub">{plural(res.missing.length, 'song')} not in your library</summary>
          <ul>{#each res.missing as m, i (i)}<li class="t-sub">{m.title} — {m.artists.join(', ')}</li>{/each}</ul>
        </details>
      {/if}
    {:else}
      <p class="t-sub empty">{loading ? 'Loading your playlists…' : 'No playlists found.'}</p>
    {/each}
  </div>
  <div class="actions">
    <button class="btn primary press" disabled={!picked.length || !!importing} onclick={importPicked}>
      {picked.length ? `Import ${plural(picked.length, 'Playlist')}` : 'Import Playlists'}
    </button>
  </div>
</section>

<style>
  section { max-width: 900px; margin: 22px 0 30px; }
  .title-row { display: flex; align-items: baseline; gap: 12px; }
  .title-row .icon-btn { align-self: center; }
  h2 { margin: 0 0 10px; }
  p { margin: 0 0 12px; line-height: 1.5; }
  .link { display: inline-flex; align-items: center; gap: 4px; color: var(--accent); }
  .list { border-top: 1px solid var(--sep); }
  .row { display: flex; align-items: center; gap: 12px; padding: 6px 8px; border-bottom: 1px solid var(--sep); }
  .row:hover { background: var(--fill); }
  .row input { width: 16px; height: 16px; accent-color: var(--accent); }
  .thumb { flex: none; width: 36px; height: 36px; border-radius: var(--r-xs); overflow: hidden; display: grid; place-items: center; background: var(--fill-2); color: var(--accent); }
  .thumb img { width: 100%; height: 100%; object-fit: cover; }
  .txt { flex: 1; min-width: 0; display: grid; line-height: 18px; }
  .res { flex: none; }
  .missing { padding: 4px 8px 8px 72px; border-bottom: 1px solid var(--sep); }
  .missing summary { cursor: default; }
  .missing ul { margin: 6px 0 0; padding-left: 16px; max-height: 220px; overflow-y: auto; }
  .empty { padding: 14px 4px; }
  .actions { display: flex; gap: 10px; margin-top: 12px; }
  .btn:disabled { opacity: 0.5; }
  .err { color: #e5484d; }
</style>
