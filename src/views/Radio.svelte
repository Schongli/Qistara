<script lang="ts">
  import Radio from 'phosphor-svelte/lib/Radio'
  import Play from 'phosphor-svelte/lib/Play'
  import Trash from 'phosphor-svelte/lib/Trash'
  import Playing from '../components/Playing.svelte'
  import MagnifyingGlass from 'phosphor-svelte/lib/MagnifyingGlass'
  import Plus from 'phosphor-svelte/lib/Plus'
  import Check from 'phosphor-svelte/lib/Check'
  import { addStation, isUrl, playUrl, radio, removeStation, searchStations, type FoundStation } from '../lib/radio.svelte'
  import { queue } from '../lib/queue.svelte'
  import { mpv } from '../lib/player.svelte'

  let name = $state('')
  let url = $state('')
  const valid = $derived(isUrl(url))
  const icy = $derived(mpv.metadata?.['icy-title'] ?? '')

  let q = $state('')
  let found = $state<FoundStation[]>([])
  let searching = $state('')
  async function find(e: SubmitEvent) {
    e.preventDefault()
    if (!q.trim()) return
    searching = 'Searching…'
    try {
      found = await searchStations(q)
      searching = found.length ? '' : 'No stations found.'
    } catch (err) {
      searching = String(err)
    }
  }
  const saved = (url: string) => radio.stations.some((s) => s.url === url)

  function add(e: SubmitEvent) {
    e.preventDefault()
    if (!valid) return
    addStation(name, url)
    name = url = ''
  }
</script>

<h1 class="t-title">Radio & Streams</h1>
<p class="t-sub">Play internet radio (Icecast, SHOUTcast, HLS) or any stream address: the same URLs VLC opens.</p>

<form class="add" onsubmit={add}>
  <input bind:value={name} placeholder="Station name (optional)" aria-label="Station name" />
  <input bind:value={url} placeholder="https://… stream address" aria-label="Stream address" class:bad={url && !valid} spellcheck="false" />
  <button class="btn press" type="button" disabled={!valid} onclick={() => playUrl(url, name)}><Play size={14} weight="fill" />Play</button>
  <button class="btn primary press" type="submit" disabled={!valid}>Save Station</button>
</form>

<h2 class="t-h2">Find Stations</h2>
<form class="find" onsubmit={find}>
  <label class="search"><MagnifyingGlass size={14} weight="bold" /><input bind:value={q} placeholder="Station, genre or city" aria-label="Find stations" /></label>
  <button class="btn press" type="submit" disabled={!q.trim()}>Search</button>
  <span class="t-sub">from radio-browser.info</span>
</form>
{#if searching}<p class="t-sub">{searching}</p>{/if}
{#if found.length}
  <div class="list results">
    {#each found as s (s.id)}
      {@const on = queue.current?.path === s.url}
      <div class="row" class:on role="listitem">
        <span class="ic">{#if on}<Playing paused={mpv.pause} />{:else}<Radio size={18} />{/if}</span>
        <button class="txt" onclick={() => playUrl(s.url, s.name)}>
          <span class="ellipsis">{s.name}</span>
          <span class="t-sub ellipsis">{[s.country, s.codec && `${s.codec}${s.bitrate ? ` ${s.bitrate} kbps` : ''}`, s.tags.split(',').slice(0, 3).join(', ')].filter(Boolean).join(' · ')}</span>
        </button>
        <button class="icon-btn" aria-label={saved(s.url) ? `${s.name} is saved` : `Save ${s.name}`} disabled={saved(s.url)} onclick={() => addStation(s.name, s.url)}>
          {#if saved(s.url)}<Check size={15} weight="bold" />{:else}<Plus size={15} weight="bold" />{/if}
        </button>
      </div>
    {/each}
  </div>
{/if}

<h2 class="t-h2 mine">Your Stations</h2>
<div class="list">
  {#each radio.stations as s (s.url)}
    {@const on = queue.current?.path === s.url}
    <div class="row" class:on role="listitem">
      <span class="ic">{#if on}<Playing paused={mpv.pause} />{:else}<Radio size={18} />{/if}</span>
      <button class="txt" onclick={() => playUrl(s.url, s.name)}>
        <span class="ellipsis">{s.name}</span>
        <span class="t-sub ellipsis">{on && icy ? `Now: ${icy}` : s.url}</span>
      </button>
      <button class="icon-btn" aria-label="Remove {s.name}" onclick={() => removeStation(s.url)}><Trash size={15} /></button>
    </div>
  {:else}
    <p class="t-sub empty">No saved stations yet. Paste a stream address above.</p>
  {/each}
</div>

<style>
  h1 { margin: 0 0 4px; }
  .add { display: grid; grid-template-columns: 1fr 2fr auto auto; gap: 8px; margin: 18px 0 20px; max-width: 900px; }
  input { height: 30px; padding: 0 10px; border-radius: 7px; border: 1px solid var(--sep); background: var(--bg-2); color: var(--text); min-width: 0; }
  input:focus { border-color: var(--accent); }
  input.bad { border-color: #e5484d; }
  .btn:disabled { opacity: 0.5; }
  .list { max-width: 900px; border-top: 1px solid var(--sep); }
  .row { display: flex; align-items: center; gap: 12px; padding: 6px 8px; border-radius: 8px; border-bottom: 1px solid var(--sep); }
  .row:hover { background: var(--fill); }
  .row.on .txt > :first-child { color: var(--accent); }
  .ic { width: 22px; display: grid; place-items: center; color: var(--accent); }
  .txt { flex: 1; min-width: 0; display: grid; text-align: left; line-height: 18px; }
  .empty { padding: 14px 4px; }
  h2 { margin: 8px 0 10px; }
  h2.mine { margin-top: 26px; }
  .find { display: flex; align-items: center; gap: 10px; margin-bottom: 10px; max-width: 900px; }
  .search { flex: 1; display: flex; align-items: center; gap: 7px; height: 30px; padding: 0 10px; border-radius: 7px; background: var(--fill-2); color: var(--text-2); }
  .search:focus-within { box-shadow: 0 0 0 2px var(--accent); }
  .search input { flex: 1; border: 0; background: none; height: auto; padding: 0; outline: none; }
  .results { max-height: 46vh; overflow-y: auto; }
</style>
