<script lang="ts">
  import FilmStrip from 'phosphor-svelte/lib/FilmStrip'
  import Play from 'phosphor-svelte/lib/Play'
  import { lib, type Track } from '../lib/library.svelte'
  import { queue } from '../lib/queue.svelte'
  import { openMenu, trackMenu } from '../lib/menu.svelte'
  import { plural, time } from '../lib/format'

  let { videos = lib.videos, title = 'Videos' }: { videos?: Track[]; title?: string } = $props()
  const folder = (p: string) => p.split(/[\\/]/).slice(-2, -1)[0] ?? ''
</script>

<h1 class="t-title">{title} <span class="t-sub count">{plural(videos.length, 'video')}</span></h1>
{#if !videos.length}
  <p class="t-sub">Video files in your library folders show up here: MKV, MP4, MOV, AVI, WebM, TS and more.</p>
{/if}
<div class="grid">
  {#each videos as v, i (v.id)}
    <button class="tile press" onclick={() => queue.play(videos, i)} oncontextmenu={(e) => openMenu(e, trackMenu([v]))}>
      <div class="frame">
        <FilmStrip size={34} weight="light" />
        <span class="play"><Play size={16} weight="fill" /></span>
        {#if v.duration}<span class="dur">{time(v.duration)}</span>{/if}
      </div>
      <div class="name ellipsis">{v.title}</div>
      <div class="t-sub ellipsis">{folder(v.path)}</div>
    </button>
  {/each}
</div>

<style>
  h1 { margin: 0 0 20px; display: flex; align-items: baseline; gap: 12px; }
  .count { font-size: 13px; font-weight: 400; letter-spacing: 0; }
  .grid { display: grid; grid-template-columns: repeat(auto-fill, minmax(230px, 1fr)); gap: 24px 20px; }
  .tile { text-align: left; min-width: 0; content-visibility: auto; contain-intrinsic-size: auto 190px; }
  .frame {
    position: relative; aspect-ratio: 16 / 9; border-radius: var(--r-art); margin-bottom: 8px; display: grid; place-items: center;
    color: var(--text-3); background: linear-gradient(150deg, var(--fill), var(--fill-2)); box-shadow: var(--shadow-art);
  }
  .play {
    position: absolute; left: 10px; bottom: 10px; width: 30px; height: 30px; border-radius: 50%; display: grid; place-items: center;
    color: #fff; background: rgba(0, 0, 0, 0.45); opacity: 0; transition: opacity 140ms;
  }
  .tile:hover .play { opacity: 1; }
  .dur { position: absolute; right: 8px; bottom: 8px; font-size: 11px; padding: 1px 5px; border-radius: 4px; background: rgba(0, 0, 0, 0.55); color: #fff; font-variant-numeric: tabular-nums; }
  .name { font-weight: 550; }
</style>
