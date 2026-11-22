<script lang="ts">
  import { activate } from '../lib/touch'
  import Icon from '../components/Icon.svelte'
  import Artwork from '../components/Artwork.svelte'
  import Playing from '../components/Playing.svelte'
  import { lib } from '../lib/library.svelte'
  import { nav } from '../lib/nav.svelte'
  import { queue } from '../lib/queue.svelte'
  import { mpv } from '../lib/player.svelte'
  import { openMenu, trackMenu } from '../lib/menu.svelte'
  import { plural, quality, time, totalTime } from '../lib/format'

  let { key }: { key: string } = $props()
  const a = $derived(lib.albums.find((x) => x.key === key))
  const discs = $derived(new Set(a?.tracks.map((t) => t.disc ?? 1)).size > 1)
  const mixed = $derived(a ? a.tracks.some((t) => t.artist && t.artist !== a.artist) : false)
  // the album's best quality, as Apple shows it under the track list
  const q = $derived(a ? quality(a.tracks.reduce((b, t) => ((t.sampleRate ?? 0) > (b.sampleRate ?? 0) ? t : b), a.tracks[0])) : null)
</script>

{#if a}
  <header class="head">
    <Artwork art={a.art} size="220px" />
    <div class="info">
      <div class="kicker">Album{#if q?.badge}<span class="badge">{q.badge}</span>{/if}</div>
      <h1 class="t-title">{a.title}</h1>
      <div class="meta">
        <button class="artist" onclick={() => nav.go({ name: 'artists', artist: a.artist })}>{a.artist}</button>
        {#each [a.genre, a.year, plural(a.tracks.length, 'song'), totalTime(a.duration)].filter(Boolean) as m, i (i)}<span class="dot">{m}</span>{/each}
      </div>
      <div class="actions">
        <button class="btn primary press" onclick={() => queue.play(a.tracks)}><Icon name="Play" size={14} weight="fill" />Play</button>
        <button class="btn press" onclick={() => queue.play(a.tracks, Math.floor(Math.random() * a.tracks.length), true)}><Icon name="Shuffle" size={14} weight="bold" />Shuffle</button>
        <button class="icon-btn press" aria-label="More for {a.title}" onclick={(e) => openMenu(e, trackMenu(a.tracks, { album: a }))}><Icon name="DotsThree" size={20} weight="bold" /></button>
      </div>
    </div>
  </header>

  <div class="list" role="list">
    {#each a.tracks as t, i (t.id)}
      {#if discs && (i === 0 || t.disc !== a.tracks[i - 1].disc)}<div class="disc t-sub">Disc {t.disc ?? 1}</div>{/if}
      {@const on = queue.current?.id === t.id}
      <div
        class="row"
        class:on
        role="listitem"
        {...activate(() => queue.play(a.tracks, i))}
        oncontextmenu={(e) => openMenu(e, trackMenu([t], { from: { list: a.tracks, index: i } }))}
      >
        <span class="num">{#if on}<Playing paused={mpv.pause} />{:else}{t.track ?? ''}{/if}</span>
        <span class="ttl ellipsis">{t.title}{#if mixed && t.artist !== a.artist}<span class="t-sub"> — {t.artist}</span>{/if}</span>
        <span class="tm">{time(t.duration)}</span>
        <button class="more icon-btn" aria-label="More for {t.title}" onclick={(e) => openMenu(e, trackMenu([t], { from: { list: a.tracks, index: i } }))}><Icon name="DotsThree" size={18} weight="bold" /></button>
      </div>
    {/each}
  </div>
  {#if q?.detail}<p class="t-sub foot">{q.detail}</p>{/if}
{:else}
  <p class="t-sub">This album is no longer in your library.</p>
{/if}

<style>
  .head { display: flex; flex-wrap: wrap; gap: 28px; align-items: flex-end; margin-bottom: 26px; }
  @media (max-width: 600px) { .head { gap: 16px; } .head > :first-child { width: min(200px, 60vw); } }
  .info { min-width: 0; padding-bottom: 4px; }
  .kicker { display: flex; gap: 8px; align-items: center; font-size: 11px; font-weight: 700; letter-spacing: 0.08em; text-transform: uppercase; color: var(--text-2); }
  .kicker .badge { letter-spacing: 0.02em; }
  h1 { margin: 6px 0 0; font-size: clamp(28px, 4vw, 44px); }
  .meta { display: flex; flex-wrap: wrap; align-items: center; margin: 8px 0 18px; font-size: 13px; color: var(--text-2); }
  .artist { color: var(--text); font-weight: 650; }
  .artist:hover { text-decoration: underline; text-underline-offset: 3px; }
  .dot::before { content: '·'; margin: 0 7px; color: var(--text-3); }
  .actions { display: flex; gap: 10px; align-items: center; }
  .actions .btn { min-width: 110px; justify-content: center; }

  .list { border-top: 1px solid var(--sep); }
  .disc { padding: 16px 12px 6px; font-weight: 600; }
  .row {
    display: grid; grid-template-columns: 34px 1fr auto 32px; align-items: center; height: calc(40px * var(--density));
    padding: 0 4px 0 12px; border-radius: var(--r-sm); border-bottom: 1px solid var(--sep);
  }
  .row:hover { background: var(--fill); }
  .row.on { color: var(--accent); }
  .num { color: var(--text-2); font-variant-numeric: tabular-nums; display: flex; }
  .ttl { min-width: 0; }
  .tm { color: var(--text-2); font-variant-numeric: tabular-nums; padding-right: 6px; }
  .more { opacity: 0; width: 28px; height: 28px; }
  .row:hover .more, .more:focus-visible { opacity: 1; }
  .foot { margin: 14px 12px; }
</style>
