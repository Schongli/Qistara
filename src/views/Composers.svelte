<script lang="ts">
  import Icon from '../components/Icon.svelte'
  import TrackTable from '../components/TrackTable.svelte'
  import { lib } from '../lib/library.svelte'
  import { queue } from '../lib/queue.svelte'
  import { nav } from '../lib/nav.svelte'
  import { groupMenu, openMenu } from '../lib/menu.svelte'
  import { collator, plural } from '../lib/format'

  let { composer }: { composer?: string } = $props()
  // the picked composer is part of the route, so Back returns to it
  const name = $derived(composer ?? lib.composers[0]?.name)
  const c = $derived(lib.composers.find((x) => x.name === name))
  // by album, then disc and track, so works stay in order
  const tracks = $derived(
    [...(c?.tracks ?? [])].sort((a, b) => collator.compare(a.album, b.album) || (a.disc ?? 1) - (b.disc ?? 1) || (a.track ?? 0) - (b.track ?? 0)),
  )
</script>

<div class="split">
  <div class="list" data-scroll role="listbox" aria-label="Composers">
    {#each lib.composers as x (x.name)}
      <button class="item press" class:sel={x.name === name} role="option" aria-selected={x.name === name} onclick={() => nav.replace({ name: 'composers', composer: x.name })} oncontextmenu={(e) => openMenu(e, groupMenu(x.tracks, () => nav.replace({ name: 'composers', composer: x.name }), x.name))}>
        <span class="ellipsis">{x.name}</span><span class="t-sub">{x.tracks.length}</span>
      </button>
    {:else}
      <p class="t-sub empty">No composer tags yet. Add them with Get Info (right-click a song).</p>
    {/each}
  </div>
  <div class="detail" data-scroll>
    {#if c}
      <h1 class="t-title">{c.name}</h1>
      <p class="t-sub">{plural(tracks.length, 'song')}</p>
      <div class="actions">
        <button class="btn primary press" onclick={() => queue.play(tracks)}><Icon name="Play" size={14} weight="fill" />Play</button>
        <button class="btn press" onclick={() => queue.play(tracks, Math.floor(Math.random() * tracks.length), true)}><Icon name="Shuffle" size={14} weight="bold" />Shuffle</button>
      </div>
      <TrackTable {tracks} />
    {/if}
  </div>
</div>

<style>
  .split { position: absolute; inset: 0; display: grid; grid-template-columns: 250px 1fr; }
  /* a phone-width window: the list on top, what's chosen below */
  @media (max-width: 600px) { .split { grid-template-columns: 1fr; grid-template-rows: minmax(0, 38%) 1fr; } .list { border-right: 0; border-bottom: 1px solid var(--sep); } .detail { padding: 16px; } }
  .list { overflow-y: auto; padding: calc(var(--bar-h) + 12px) 8px 16px 14px; border-right: 1px solid var(--sep); }
  .item { display: flex; justify-content: space-between; gap: 10px; width: 100%; height: 34px; align-items: center; padding: 0 10px; border-radius: var(--r); text-align: left; content-visibility: auto; contain-intrinsic-size: auto 34px; }
  .item:hover { background: var(--fill); }
  .item.sel { background: var(--fill-2); font-weight: 550; }
  .empty { padding: 8px; line-height: 1.5; }
  .detail { overflow-y: auto; padding: calc(var(--bar-h) + 24px) 28px 40px; }
  h1 { margin: 0; }
  p { margin: 4px 0 14px; }
  .actions { display: flex; gap: 10px; margin-bottom: 22px; }
</style>
