<script lang="ts">
  import Icon from '../components/Icon.svelte'
  import { exportPlaylist } from '../lib/playlistio'
  import Artwork from '../components/Artwork.svelte'
  import TrackTable from '../components/TrackTable.svelte'
  import { deletePlaylist, lib, setPlaylistItems, updatePlaylist, type Track } from '../lib/library.svelte'
  import { evaluate, describe, type Rules } from '../lib/smart'
  import { nav, ui } from '../lib/nav.svelte'
  import { queue } from '../lib/queue.svelte'
  import { move } from '../lib/queue'
  import { plural, totalTime } from '../lib/format'

  let { id = null, title = '', builtin = null, subtitle = '' }: { id?: number | null; title?: string; builtin?: Rules | null; subtitle?: string } = $props()

  const p = $derived(id === null ? null : lib.playlists.find((x) => x.id === id))
  const rules = $derived<Rules | null>(builtin ?? (p?.rules ? JSON.parse(p.rules) : null))
  const tracks = $derived<Track[]>(
    rules ? evaluate(lib.tracks, rules) : (p?.items.map((i) => lib.byId.get(i)).filter((t): t is Track => !!t) ?? []),
  )
  // a 2x2 mosaic of different albums, like Apple Music playlist art
  const covers = $derived([...new Set(tracks.map((t) => t.art).filter(Boolean))].slice(0, 4) as string[])
  const duration = $derived(tracks.reduce((s, t) => s + t.duration, 0))

  let renaming = $state(false)
  let draft = $state('')
  function rename() {
    if (p && draft.trim()) updatePlaylist(p.id, draft.trim(), p.rules)
    renaming = false
  }
  async function remove() {
    if (!p) return
    await deletePlaylist(p.id)
    nav.root({ name: 'albums' })
  }
</script>

<header class="head">
  <div class="art">
    {#if covers.length >= 4}
      <div class="mosaic">{#each covers as c (c)}<Artwork art={c} radius="0" shadow={false} />{/each}</div>
    {:else}
      <Artwork art={covers[0] ?? null} />
    {/if}
  </div>
  <div class="info">
    {#if rules}<div class="kind"><Icon name="Sparkle" size={12} weight="fill" /> Smart Playlist</div>{:else}<div class="kind">Playlist</div>{/if}
    {#if renaming}
      <input class="t-title rename" bind:value={draft} onblur={rename} onkeydown={(e) => (e.key === 'Enter' ? rename() : e.key === 'Escape' && (renaming = false))} aria-label="Playlist name" />
    {:else}
      <h1 class="t-title" ondblclick={() => p && ((draft = p.name), (renaming = true))}>{p?.name ?? title}</h1>
    {/if}
    <p class="t-sub">{subtitle || (rules ? describe(rules) : '')}</p>
    <p class="t-sub">{plural(tracks.length, 'song')}{tracks.length ? `, ${totalTime(duration)}` : ''}</p>
    <div class="actions">
      <button class="btn primary press" onclick={() => queue.play(tracks)} disabled={!tracks.length}><Icon name="Play" size={14} weight="fill" />Play</button>
      <button class="btn press" onclick={() => queue.play(tracks, Math.floor(Math.random() * tracks.length), true)} disabled={!tracks.length}><Icon name="Shuffle" size={14} weight="bold" />Shuffle</button>
      {#if p && rules}
        <button class="btn press" onclick={() => (ui.editor = { id: p.id, name: p.name, rules })}><Icon name="Gear" size={14} />Edit Rules</button>
      {/if}
      <button class="icon-btn press" aria-label="Export playlist as M3U8" title="Export as M3U8" onclick={() => exportPlaylist(p?.name ?? title, tracks)} disabled={!tracks.length}><Icon name="Export" size={16} /></button>
      {#if p}
        <button class="icon-btn press" aria-label="Delete playlist" title="Delete playlist" onclick={remove}><Icon name="Trash" size={16} /></button>
      {/if}
    </div>
  </div>
</header>

<TrackTable
  {tracks}
  onreorder={p && !rules ? (from, to) => setPlaylistItems(p.id, move(p.items, from, to)) : undefined}
  onremove={p && !rules ? (i) => setPlaylistItems(p.id, p.items.filter((_, j) => j !== i)) : undefined}
/>

<style>
  .head { display: flex; gap: 28px; align-items: flex-end; margin-bottom: 26px; }
  .art { width: 236px; flex: none; }
  .mosaic { display: grid; grid-template-columns: 1fr 1fr; border-radius: var(--r-art); overflow: hidden; box-shadow: var(--shadow-art); }
  .info { min-width: 0; padding-bottom: 4px; }
  .kind { display: flex; align-items: center; gap: 5px; font-size: 11px; font-weight: 650; text-transform: uppercase; letter-spacing: 0.04em; color: var(--accent); margin-bottom: 4px; }
  h1 { margin: 0; }
  .rename { border: 0; outline: none; background: var(--fill); border-radius: var(--r-sm); padding: 0 6px; width: 100%; }
  p { margin: 4px 0 0; }
  .actions { display: flex; gap: 10px; align-items: center; margin-top: 16px; }
  .btn:disabled { opacity: 0.5; }
</style>
