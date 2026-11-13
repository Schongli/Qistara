<script lang="ts">
  // Your music server: albums, artists, playlists and search, played straight from it.
  import Play from 'phosphor-svelte/lib/Play'
  import Shuffle from 'phosphor-svelte/lib/Shuffle'
  import CaretLeft from 'phosphor-svelte/lib/CaretLeft'
  import MagnifyingGlass from 'phosphor-svelte/lib/MagnifyingGlass'
  import Artwork from '../components/Artwork.svelte'
  import TrackTable from '../components/TrackTable.svelte'
  import { albumList, cover, getAlbum, getArtist, getArtists, getPlaylist, getPlaylists, search, server, toTracks, type SAlbum, type SArtist, type SPlaylist } from '../lib/subsonic.svelte'
  import { queue } from '../lib/queue.svelte'
  import { nav } from '../lib/nav.svelte'
  import type { Track } from '../lib/library.svelte'

  type Tab = 'newest' | 'alphabeticalByName' | 'frequent' | 'artists' | 'playlists' | 'search'
  const TABS: [Tab, string][] = [['newest', 'New'], ['alphabeticalByName', 'Albums A–Z'], ['frequent', 'Most Played'], ['artists', 'Artists'], ['playlists', 'Playlists']]
  let tab = $state<Tab>('newest')
  let albums = $state<SAlbum[]>([])
  let artists = $state<SArtist[]>([])
  let playlists = $state<SPlaylist[]>([])
  let found = $state<{ artists: SArtist[]; albums: SAlbum[]; songs: Track[] } | null>(null)
  let query = $state('')
  let error = $state('')
  let loading = $state(false)
  // an open album, artist or playlist
  let detail = $state<{ title: string; sub: string; coverArt?: string; tracks?: Track[]; albums?: SAlbum[] } | null>(null)

  async function guard(f: () => Promise<void>) {
    error = ''
    loading = true
    try {
      await f()
    } catch (e) {
      error = String(e)
    } finally {
      loading = false
    }
  }
  function show(t: Tab) {
    tab = t
    detail = null
    if (t === 'artists') guard(async () => void (artists = await getArtists()))
    else if (t === 'playlists') guard(async () => void (playlists = await getPlaylists()))
    else if (t !== 'search') guard(async () => void (albums = await albumList(t)))
  }
  const more = () => guard(async () => void (albums = [...albums, ...(await albumList(tab as 'newest', albums.length))]))
  const openAlbum = (a: SAlbum) =>
    guard(async () => {
      const full = await getAlbum(a.id)
      detail = { title: full.name, sub: [full.artist, full.year].filter(Boolean).join(' · '), coverArt: full.coverArt, tracks: await toTracks(full.song ?? []) }
    })
  const openArtist = (a: SArtist) =>
    guard(async () => {
      const full = await getArtist(a.id)
      detail = { title: full.name, sub: `${full.album?.length ?? 0} albums`, coverArt: full.coverArt, albums: full.album ?? [] }
    })
  const openPlaylist = (p: SPlaylist) =>
    guard(async () => {
      const full = await getPlaylist(p.id)
      detail = { title: full.name, sub: `${full.entry?.length ?? 0} songs`, coverArt: full.coverArt, tracks: await toTracks(full.entry ?? []) }
    })
  const find = () =>
    query.trim() &&
    guard(async () => {
      const r = await search(query)
      found = { artists: r.artists, albums: r.albums, songs: await toTracks(r.songs) }
      tab = 'search'
      detail = null
    })

  $effect(() => {
    if (server.account) show('newest')
  })
</script>

{#snippet grid(list: SAlbum[])}
  <div class="grid">
    {#each list as a (a.id)}
      <button class="tile press" onclick={() => openAlbum(a)}>
        {#await cover(a.coverArt)}<Artwork art={null} />{:then art}<Artwork {art} />{/await}
        <span class="name ellipsis">{a.name}</span>
        <span class="t-sub ellipsis">{a.artist ?? ''}{a.year ? ` · ${a.year}` : ''}</span>
      </button>
    {/each}
  </div>
{/snippet}

<h1 class="t-title">Server</h1>
{#if !server.checked}
  <p class="t-sub">…</p>
{:else if !server.account}
  <div class="card">
    <p>Play your own music server: Navidrome, Gonic, Airsonic, Ampache or anything else that speaks the Subsonic API. Lossless files stream as they are.</p>
    <button class="btn primary press" onclick={() => nav.root({ name: 'providers' })}>Sign In in Providers</button>
  </div>
{:else}
  <p class="t-sub sub">{server.account.user} on {server.account.server}</p>
  <div class="bar">
    <div class="tabs" role="tablist" aria-label="Browse">
      {#each TABS as [id, label] (id)}
        <button role="tab" class="tab press" class:on={tab === id && !detail} aria-selected={tab === id} onclick={() => show(id)}>{label}</button>
      {/each}
    </div>
    <form class="search" onsubmit={(e) => (e.preventDefault(), find())}>
      <MagnifyingGlass size={14} weight="bold" />
      <input bind:value={query} placeholder="Search the server" aria-label="Search the server" spellcheck="false" />
    </form>
  </div>
  {#if error}<p class="err">{error}</p>{/if}

  {#if detail}
    <button class="back press" onclick={() => (detail = null)}><CaretLeft size={14} weight="bold" />Back</button>
    <header class="head">
      {#await cover(detail.coverArt)}<Artwork art={null} size="180px" />{:then art}<Artwork {art} size="180px" />{/await}
      <div>
        <h2 class="t-title">{detail.title}</h2>
        <p class="t-sub">{detail.sub}</p>
        {#if detail.tracks?.length}
          <div class="actions">
            <button class="btn primary press" onclick={() => detail?.tracks && queue.play(detail.tracks)}><Play size={14} weight="fill" />Play</button>
            <button class="btn press" onclick={() => detail?.tracks && queue.play(detail.tracks, Math.floor(Math.random() * detail.tracks.length), true)}><Shuffle size={14} weight="bold" />Shuffle</button>
          </div>
        {/if}
      </div>
    </header>
    {#if detail.tracks}<TrackTable tracks={detail.tracks} remote />{/if}
    {#if detail.albums}{@render grid(detail.albums)}{/if}
  {:else if tab === 'artists'}
    <div class="names">
      {#each artists as a (a.id)}<button class="nm press" onclick={() => openArtist(a)}>{a.name}<span class="t-sub">{a.albumCount ?? ''}</span></button>{/each}
    </div>
  {:else if tab === 'playlists'}
    <div class="names">
      {#each playlists as p (p.id)}<button class="nm press" onclick={() => openPlaylist(p)}>{p.name}<span class="t-sub">{p.songCount ?? ''}</span></button>{:else}<p class="t-sub">No playlists on the server.</p>{/each}
    </div>
  {:else if tab === 'search' && found}
    {#if found.songs.length}<h2 class="t-h2">Songs</h2><TrackTable tracks={found.songs} remote />{/if}
    {#if found.albums.length}<h2 class="t-h2">Albums</h2>{@render grid(found.albums)}{/if}
    {#if found.artists.length}
      <h2 class="t-h2">Artists</h2>
      <div class="names">{#each found.artists as a (a.id)}<button class="nm press" onclick={() => openArtist(a)}>{a.name}</button>{/each}</div>
    {/if}
    {#if !found.songs.length && !found.albums.length && !found.artists.length}<p class="t-sub">Nothing found.</p>{/if}
  {:else}
    {@render grid(albums)}
    {#if albums.length && albums.length % 60 === 0}<button class="btn press more" onclick={more} disabled={loading}>Show More</button>{/if}
  {/if}
  {#if loading}<p class="t-sub">Loading…</p>{/if}
{/if}

<style>
  h1 { margin: 0 0 4px; }
  .sub { margin: 0 0 16px; }
  .card { display: grid; gap: 10px; justify-items: start; max-width: 560px; padding: 16px; border-radius: 12px; background: var(--fill); }
  .card p { margin: 0; line-height: 1.5; }
  .bar { display: flex; align-items: center; justify-content: space-between; gap: 12px; flex-wrap: wrap; margin-bottom: 18px; }
  .tabs { display: flex; gap: 4px; flex-wrap: wrap; }
  .tab { height: 28px; padding: 0 12px; border-radius: 14px; font-size: 12px; font-weight: 600; color: var(--text-2); background: var(--fill); }
  .tab.on { background: var(--accent); color: var(--on-accent); }
  .search { display: flex; align-items: center; gap: 6px; height: 30px; padding: 0 12px; border-radius: 15px; background: var(--fill-2); color: var(--text-2); }
  .search:focus-within { box-shadow: 0 0 0 2px var(--accent); }
  .search input { border: 0; background: none; outline: none; color: var(--text); width: 200px; }
  .grid { display: grid; grid-template-columns: repeat(auto-fill, minmax(160px, 1fr)); gap: 22px 20px; margin-bottom: 20px; }
  .tile { display: grid; gap: 6px; text-align: left; min-width: 0; }
  .name { font-weight: 550; }
  .back { display: inline-flex; align-items: center; gap: 4px; color: var(--text-2); margin-bottom: 12px; }
  .head { display: flex; gap: 24px; align-items: flex-end; margin-bottom: 20px; }
  .head h2 { margin: 0 0 4px; }
  .head p { margin: 0 0 14px; }
  .actions { display: flex; gap: 8px; }
  .names { display: grid; grid-template-columns: repeat(auto-fill, minmax(220px, 1fr)); gap: 4px; }
  .nm { display: flex; justify-content: space-between; gap: 8px; padding: 8px 12px; border-radius: 8px; background: var(--fill); text-align: left; min-width: 0; }
  .nm:hover { background: var(--fill-2); }
  h2.t-h2 { margin: 20px 0 10px; }
  .more { margin-bottom: 30px; }
  .btn:disabled { opacity: 0.6; }
  .err { color: #e5484d; }
</style>
