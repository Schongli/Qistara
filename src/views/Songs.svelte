<script lang="ts" module>
  type Col = 'title' | 'artist' | 'album' | 'duration' | 'addedAt'
  // kept across visits, like the rest of the view's place
  let sort = $state<{ col: Col; desc: boolean }>({ col: 'title', desc: false })
</script>

<script lang="ts">
  import Icon from '../components/Icon.svelte'
  import VirtualList from '../components/VirtualList.svelte'
  import Artwork from '../components/Artwork.svelte'
  import Playing from '../components/Playing.svelte'
  import { lib, type Track } from '../lib/library.svelte'
  import { queue } from '../lib/queue.svelte'
  import { mpv } from '../lib/player.svelte'
  import { settings } from '../lib/settings.svelte'
  import { openMenu, trackMenu } from '../lib/menu.svelte'
  import { byName, collator, plural, quality, time } from '../lib/format'

  const cmp: Record<Col, (a: Track, b: Track) => number> = {
    title: (a, b) => collator.compare(a.title, b.title),
    artist: (a, b) => byName(a.artist, b.artist) || collator.compare(a.album, b.album) || (a.track ?? 0) - (b.track ?? 0),
    album: (a, b) => collator.compare(a.album, b.album) || (a.disc ?? 1) - (b.disc ?? 1) || (a.track ?? 0) - (b.track ?? 0),
    duration: (a, b) => a.duration - b.duration,
    addedAt: (a, b) => a.addedAt - b.addedAt,
  }
  const songs = $derived.by(() => {
    const s = [...lib.songs].sort(cmp[sort.col])
    return sort.desc ? s.reverse() : s
  })
  let selected = $state(-1)
  const setSort = (col: Col) => (sort = { col, desc: sort.col === col ? !sort.desc : false })
  const cols: [Col, string][] = [['title', 'Title'], ['artist', 'Artist'], ['album', 'Album'], ['duration', 'Time']]
</script>

<div class="songs">
  <VirtualList items={songs} rowHeight={settings.density === 'compact' ? 28 : 36} padTop={76}>
    {#snippet header()}
      <h1 class="t-title">Songs <span class="t-sub count">{plural(songs.length, 'song')}</span></h1>
      <div class="cols" role="row">
        {#each cols as [c, label] (c)}
          <span class="col {c}" role="columnheader" aria-sort={sort.col === c ? (sort.desc ? 'descending' : 'ascending') : 'none'}>
            <button onclick={() => setSort(c)}>
              {label}
              {#if sort.col === c}{#if sort.desc}<Icon name="CaretDown" size={10} weight="bold" />{:else}<Icon name="CaretUp" size={10} weight="bold" />{/if}{/if}
            </button>
          </span>
        {/each}
      </div>
    {/snippet}
    {#snippet row(t, i)}
      {@const on = queue.current?.id === t.id}
      {@const q = quality(t)}
      <div
        class="row"
        class:on
        class:sel={selected === i}
        class:odd={i % 2 === 1}
        role="row"
        tabindex="-1"
        aria-selected={selected === i}
        onpointerdown={() => (selected = i)}
        ondblclick={() => queue.play(songs, i)}
        onkeydown={(e) => e.key === 'Enter' && queue.play(songs, i)}
        oncontextmenu={(e) => openMenu(e, trackMenu([t]))}
      >
        <span class="title">
          <span class="mini"><Artwork art={t.art} size="26px" radius="3px" shadow={false} /></span>
          <span class="ellipsis">{t.title}</span>
          {#if on}<Playing paused={mpv.pause} />{/if}
          {#if q.badge}<span class="badge q">{q.badge === 'Hi-Res Lossless' ? 'Hi-Res' : q.badge}</span>{/if}
        </span>
        <span class="artist ellipsis">{t.artist}</span>
        <span class="album ellipsis">{t.album}</span>
        <span class="duration">{time(t.duration)}</span>
      </div>
    {/snippet}
  </VirtualList>
</div>

<style>
  .songs { position: absolute; inset: 0; }
  h1 { margin: 0 28px 16px; display: flex; align-items: baseline; gap: 12px; }
  .count { font-size: 13px; font-weight: 400; letter-spacing: 0; }
  .cols, .row { display: grid; grid-template-columns: minmax(200px, 2.2fr) 1.2fr 1.4fr 70px; gap: 16px; align-items: center; margin: 0 20px; padding: 0 12px; }
  .cols { height: 28px; border-bottom: 1px solid var(--sep); }
  .col { display: flex; }
  .col button { display: flex; align-items: center; gap: 4px; font-size: 11.5px; font-weight: 600; color: var(--text-2); }
  .col.duration, .duration { justify-content: flex-end; text-align: right; }
  .row { height: calc(36px * var(--density)); border-radius: var(--r-sm); }
  .row.odd { background: var(--fill); }
  .row:hover { background: var(--fill-2); }
  .row.sel { background: var(--accent); color: var(--on-accent); }
  .row.sel .artist, .row.sel .album, .row.sel .duration { color: inherit; }
  .row.on:not(.sel) .title { color: var(--accent); }
  .title { display: flex; align-items: center; gap: 10px; min-width: 0; }
  .mini { flex: none; }
  .q { flex: none; margin-left: auto; }
  .row.sel .q { color: inherit; border-color: currentColor; }
  .artist, .album, .duration { color: var(--text-2); }
  .duration { font-variant-numeric: tabular-nums; }
</style>
