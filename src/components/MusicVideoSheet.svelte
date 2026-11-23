<script lang="ts">
  // "Music Video…" on a song: link a video to it, from YouTube (a pasted link or a search) or from your library.
  // The song keeps playing from its own file; the video shows over it (musicvideo.svelte.ts).
  import { invoke } from '@tauri-apps/api/core'
  import Artwork from './Artwork.svelte'
  import { lib, type Track } from '../lib/library.svelte'
  import { linkFor, setLink } from '../lib/musicvideo.svelte'
  import { parseVideoId } from '../lib/videolink'
  import { ytCookies } from '../lib/downloads.svelte'
  import { time } from '../lib/format'

  let { track, onclose }: { track: Track; onclose: () => void } = $props()
  type Video = { id: string; title: string; channel: string; duration: number | null }

  const link = $derived(linkFor(track))
  let pasted = $state('')
  let found = $state<Video[] | null>(null)
  let busy = $state(false)
  let error = $state('')
  const pastedId = $derived(parseVideoId(pasted))
  let sheet = $state<HTMLDivElement>()
  $effect(() => sheet?.focus())

  function pick(l: { yt?: string; video?: number }) {
    setLink(track, { ...l, offset: link?.offset ?? 0 })
    onclose()
  }

  async function find() {
    busy = true
    error = ''
    try {
      const q = `${track.artist} ${track.title} official music video`
      found = (await invoke<Video[]>('youtube_search', { query: q, cookies: ytCookies() })).slice(0, 5)
    } catch (e) {
      error = String(e)
    } finally {
      busy = false
    }
  }
</script>

<div class="scrim" role="presentation" onclick={onclose}></div>
<div class="sheet" role="dialog" aria-modal="true" aria-labelledby="mv-title" tabindex="-1" bind:this={sheet} onkeydown={(e) => e.key === 'Escape' && onclose()}>
  <h2 class="t-h2" id="mv-title">Music Video</h2>
  <p class="t-sub">
    For “{track.title}”. The song plays from your own file, as it always does, and the video shows over it without its
    own sound. Set the sync in the video's controls if it starts earlier or later.
  </p>

  {#if link}
    <div class="now">
      <span>Linked: {link.yt ? 'a YouTube video' : (lib.byId.get(link.video ?? 0)?.title ?? 'a video no longer in your library')}{link.offset ? ` · sync ${link.offset > 0 ? '+' : ''}${link.offset} s` : ''}</span>
      <button class="btn press" onclick={() => { setLink(track, null); onclose() }}>Remove</button>
    </div>
  {/if}

  <h3>From YouTube</h3>
  <div class="row">
    <input class="field" placeholder="Paste a YouTube link" aria-label="YouTube link" bind:value={pasted} onkeydown={(e) => e.key === 'Enter' && pastedId && pick({ yt: pastedId })} />
    <button class="btn primary press" disabled={!pastedId} onclick={() => pastedId && pick({ yt: pastedId })}>Use Link</button>
  </div>
  <button class="btn press find" disabled={busy} onclick={find}>{busy ? 'Searching…' : `Find “${track.title}” on YouTube`}</button>
  {#if error}<p class="t-sub bad">{error}</p>{/if}
  {#if found}
    <div class="list">
      {#each found as v (v.id)}
        <button class="hit" onclick={() => pick({ yt: v.id })}>
          <Artwork art={`https://i.ytimg.com/vi/${v.id}/mqdefault.jpg`} size="64px" radius="4px" shadow={false} />
          <span class="txt"><span class="ellipsis">{v.title}</span><span class="t-sub ellipsis">{v.channel}{v.duration ? ` · ${time(v.duration)}` : ''}</span></span>
        </button>
      {:else}
        <p class="t-sub">Nothing found.</p>
      {/each}
    </div>
  {/if}

  {#if lib.videos.length}
    <h3>From your videos</h3>
    <select aria-label="A video from your library" onchange={(e) => e.currentTarget.value && pick({ video: Number(e.currentTarget.value) })}>
      <option value="">Choose a video…</option>
      {#each lib.videos as v (v.id)}<option value={v.id}>{v.title}</option>{/each}
    </select>
  {/if}
</div>

<style>
  .scrim { position: fixed; inset: 0; z-index: 80; background: var(--sheet-dim); animation: fade 200ms; }
  .sheet {
    position: fixed; z-index: 81; top: 0; left: 50%; width: min(560px, 94vw); max-height: 88vh; overflow-y: auto;
    transform: translateX(-50%); padding: 20px 22px 18px; border-radius: 0 0 var(--r-xl) var(--r-xl);
    background: var(--bg-2); border: 1px solid var(--sep); border-top: 0; box-shadow: var(--shadow-pop);
    animation: drop 280ms var(--ease-out); outline: none;
  }
  @keyframes drop { from { transform: translate(-50%, -100%); } }
  @keyframes fade { from { opacity: 0; } }
  h2 { margin: 0; }
  p { margin: 4px 0 12px; line-height: 1.45; }
  h3 { margin: 16px 0 8px; font-size: 12px; font-weight: 650; letter-spacing: 0.04em; text-transform: uppercase; color: var(--text-2); }
  .now { display: flex; align-items: center; justify-content: space-between; gap: 12px; padding: 8px 10px 8px 12px; border-radius: var(--r); background: var(--fill); }
  .row { display: flex; gap: 8px; }
  .field { flex: 1; min-width: 0; height: 30px; padding: 0 10px; border-radius: var(--r); border: 1px solid var(--sep); background: var(--bg); color: var(--text); }
  .field:focus { border-color: var(--accent); outline: none; }
  .find { margin-top: 8px; }
  .list { margin-top: 8px; display: grid; gap: 2px; }
  .hit { display: grid; grid-template-columns: 64px 1fr; gap: 12px; align-items: center; padding: 6px; border-radius: var(--r); text-align: left; }
  .hit:hover, .hit:focus-visible { background: var(--fill-2); outline: none; }
  .txt { display: grid; min-width: 0; gap: 2px; }
  select { width: 100%; height: 30px; border-radius: var(--r); border: 1px solid var(--sep); background: var(--bg); color: var(--text); padding: 0 8px; }
  .bad { color: var(--signal-veil); }
  @media (prefers-reduced-motion: reduce) { .sheet, .scrim { animation: none; } }
</style>
