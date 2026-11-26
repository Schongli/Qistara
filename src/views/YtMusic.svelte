<script lang="ts">
  // YT Music: Home shelves to start from (yours when signed in), your Library, and search by songs, albums, artists
  // or playlists.
  import Icon from '../components/Icon.svelte'
  import TrackTable from '../components/TrackTable.svelte'
  import RemoteGrid from '../components/RemoteGrid.svelte'
  import { nav } from '../lib/nav.svelte'
  import { loadHome, loadLibrary, playItem, search, songTrack, ytm, type Filter, type Item, type Shelf } from '../lib/ytmusic.svelte'
  import { account } from '../lib/account.svelte'

  const FILTERS: [Filter, string][] = [['', 'All'], ['songs', 'Songs'], ['albums', 'Albums'], ['artists', 'Artists'], ['playlists', 'Playlists']]
  // the search stays put across Back and Forward (module state outlives the view)
  let q = $state(last.q)
  let filter = $state<Filter>(last.filter)
  let found = $state<Shelf[] | null>(last.found)
  let busy = $state(false)
  let err = $state('')
  // Home or your Library (signed in); a search shows its results instead
  let view = $state<'home' | 'library'>(last.view)
  $effect(() => {
    last.view = view
    if (view === 'library' && account.info) loadLibrary()
  })

  loadHome()

  async function run() {
    const query = q.trim()
    last.q = q
    last.filter = filter
    if (!query) return void (found = last.found = null)
    busy = true
    err = ''
    try {
      found = last.found = await search(query, filter)
    } catch (e) {
      err = String(e)
    } finally {
      busy = false
    }
  }
  const pick = (f: Filter) => ((filter = f), q.trim() && run())

  export function open(i: Item) {
    if (i.kind === 'song') return playItem(i)
    nav.go(i.kind === 'album' ? { name: 'ytmAlbum', id: i.id } : i.kind === 'artist' ? { name: 'ytmArtist', id: i.id } : { name: 'ytmPlaylist', id: i.id })
  }
  const library = $derived(view === 'library' && !!account.info && !found)
  const shelves = $derived(found ?? (library ? (ytm.library ?? []) : (ytm.home ?? [])))
  const songs = (s: Shelf) => s.items.every((i) => i.kind === 'song')
</script>

<script module lang="ts">
  const last = { q: '', filter: '' as Filter, found: null as Shelf[] | null, view: 'home' as 'home' | 'library' }
</script>

<div class="head">
  <h1 class="t-title">YT Music</h1>
  <button class="btn press" onclick={() => nav.go({ name: 'youtube' })} title="Search YouTube itself, open links, and your Liked Music">
    <Icon name="YoutubeLogo" size={15} />Links & Videos
  </button>
</div>

{#if account.info}
  <div class="views" role="tablist" aria-label="YT Music">
    <button class="view press" role="tab" aria-selected={view === 'home' || !!found} class:on={view === 'home' && !found} onclick={() => ((view = 'home'), (q = ''), run())}>Home</button>
    <button class="view press" role="tab" aria-selected={library} class:on={library} onclick={() => ((view = 'library'), (q = ''), run())}>Library</button>
  </div>
{/if}

<form class="search" onsubmit={(e) => (e.preventDefault(), run())}>
  <Icon name="MagnifyingGlass" size={15} />
  <input bind:value={q} type="search" placeholder="Songs, albums, artists, playlists" aria-label="Search YT Music" spellcheck="false" />
  {#if busy}<span class="t-sub">Searching…</span>{/if}
</form>
<div class="chips" role="radiogroup" aria-label="Show">
  {#each FILTERS as [f, label] (f)}
    <button class="chip press" role="radio" aria-checked={filter === f} class:on={filter === f} onclick={() => pick(f)}>{label}</button>
  {/each}
</div>

{#if err || (!found && !library && ytm.error) || (library && ytm.libraryError)}
  <p class="err">{err || (library ? ytm.libraryError : ytm.error)}</p>
{:else if !found && !library && ytm.loading}
  <p class="t-sub">Loading YT Music…</p>
{:else if library && !ytm.library}
  <p class="t-sub">Loading your library…</p>
{:else if found && !found.length}
  <p class="t-sub">Nothing found for “{q.trim()}”.</p>
{/if}

{#each shelves as s, n (n + s.title)}
  <section>
    <h2 class="t-h2">{s.title}</h2>
    {#if songs(s)}
      <TrackTable tracks={s.items.slice(0, found ? 50 : 12).map(songTrack)} remote />
    {:else}
      <RemoteGrid items={s.items} row={!found || s.items.length > 6} onopen={open} onplay={(i) => playItem(i)} />
    {/if}
  </section>
{/each}
<p class="t-sub note">
  {#if account.info}From YouTube Music, signed in as {account.info.name}.{:else}From YouTube Music, without signing in: <button class="link" onclick={() => nav.root({ name: 'providers' })}>sign in</button> for your Home and Library.{/if}
  Songs stream straight from YouTube through Qistara’s streaming engine.
</p>

<style>
  .head { display: flex; align-items: center; justify-content: space-between; gap: 12px; margin-bottom: 14px; }
  h1 { margin: 0; }
  .search { display: flex; align-items: center; gap: 8px; max-width: 560px; height: 38px; padding: 0 12px; border-radius: var(--r-lg); background: var(--fill); color: var(--text-2); }
  .search:focus-within { box-shadow: 0 0 0 2px var(--accent); }
  .search input { flex: 1; min-width: 0; border: 0; background: none; color: var(--text); font: inherit; font-size: 14px; outline: none; }
  .chips { display: flex; flex-wrap: wrap; gap: 6px; margin: 12px 0 22px; }
  .chip { height: 28px; padding: 0 12px; border-radius: 14px; background: var(--fill); font-weight: 600; font-size: 12.5px; }
  .chip.on { background: var(--accent); color: var(--on-accent); }
  section { margin-bottom: 30px; }
  section h2 { margin: 0 0 12px; }
  .err { color: var(--signal-veil); }
  .note { margin-top: 10px; }
  .views { display: flex; gap: 4px; margin-bottom: 14px; }
  .view { height: 30px; padding: 0 14px; border-radius: 15px; font-weight: 600; font-size: 13px; color: var(--text-2); }
  .view:hover { color: var(--text); background: var(--fill); }
  .view.on { color: var(--text); background: var(--fill-2); }
  .link { color: var(--accent); text-decoration: underline; text-underline-offset: 2px; }
</style>
