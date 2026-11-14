<script lang="ts">
  import Icon from '../components/Icon.svelte'
  import TrackTable from '../components/TrackTable.svelte'
  import { likedMusic, lookUp, yt } from '../lib/youtube.svelte'
  import { dl, quickDownload, refreshTools } from '../lib/downloads.svelte'
  import { queue } from '../lib/queue.svelte'
  import { nav } from '../lib/nav.svelte'
  import { settings } from '../lib/settings.svelte'

  let text = $state('')
  if (!dl.tools) refreshTools()
  const signedIn = $derived(settings.ytCookies === 'firefox' ? 'Firefox' : settings.ytCookies ? 'a cookies file' : '')
  const urls = () => yt.tracks.map((t) => t.path.replace(/^ytdl:\/\//, ''))
</script>

<header class="head">
  <h1 class="t-title">YouTube</h1>
  <p class="t-sub">
    Songs and videos from YouTube, played through yt-dlp. {signedIn ? `Signed in with ${signedIn}.` : 'As a guest.'}
    <button class="link" onclick={() => nav.root({ name: 'providers' })}>{signedIn ? 'Change' : 'Sign in for Liked Music and Premium audio'}</button>
  </p>
</header>

{#if dl.tools && !dl.tools.ytdlp}
  <div class="card">
    <p>YouTube plays through yt-dlp, which isn't installed yet.</p>
    <button class="btn primary press" onclick={() => nav.root({ name: 'downloads' })}>Get yt-dlp in Downloads</button>
  </div>
{:else}
  <form class="search" onsubmit={(e) => (e.preventDefault(), text.trim() && lookUp(text))}>
    <Icon name="MagnifyingGlass" size={15} weight="bold" />
    <input bind:value={text} placeholder="Search YouTube, or paste a video, playlist or album link" aria-label="Search YouTube" spellcheck="false" />
    <button class="btn primary press" disabled={!text.trim() || yt.loading}>{yt.loading ? 'Searching…' : 'Search'}</button>
  </form>
  {#if signedIn}
    <button class="btn press liked" onclick={likedMusic} disabled={yt.loading}><Icon name="Heart" size={14} weight="fill" />Liked Music</button>
  {/if}

  {#if yt.error}<p class="err">{yt.error}</p>{/if}
  {#if yt.tracks.length}
    <div class="bar">
      <h2 class="t-h2">{yt.title}</h2>
      <div class="actions">
        <button class="btn primary press" onclick={() => queue.play(yt.tracks)}><Icon name="Play" size={14} weight="fill" />Play</button>
        <button class="btn press" onclick={() => quickDownload(urls())}><Icon name="DownloadSimple" size={14} weight="bold" />Download All</button>
      </div>
    </div>
    <TrackTable tracks={yt.tracks} remote />
  {:else if !yt.loading && yt.title}
    <p class="t-sub">Nothing found.</p>
  {/if}
{/if}

<style>
  .head h1 { margin: 0 0 4px; }
  .head p { margin: 0 0 18px; }
  .link { color: var(--accent); text-decoration: underline; text-underline-offset: 2px; }
  .card { display: grid; gap: 10px; justify-items: start; max-width: 560px; padding: 16px; border-radius: var(--r-lg); background: var(--fill); }
  .card p { margin: 0; }
  .search { display: flex; align-items: center; gap: 8px; max-width: 760px; height: 40px; padding: 0 4px 0 14px; border-radius: 20px; background: var(--fill-2); color: var(--text-2); }
  .search:focus-within { box-shadow: 0 0 0 2px var(--accent); }
  .search input { flex: 1; min-width: 0; border: 0; background: none; outline: none; color: var(--text); }
  .search .btn { border-radius: 16px; height: 32px; }
  .btn:disabled { opacity: 0.55; }
  .liked { margin-top: 10px; }
  .bar { display: flex; align-items: center; justify-content: space-between; gap: 12px; margin: 22px 0 10px; }
  .bar h2 { margin: 0; min-width: 0; }
  .actions { display: flex; gap: 8px; }
  .err { color: #e5484d; }
</style>
