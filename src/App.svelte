<script lang="ts">
  import Sidebar from './components/Sidebar.svelte'
  import TopBar from './components/TopBar.svelte'
  import ContextMenu from './components/ContextMenu.svelte'
  import NowPlaying from './components/NowPlaying.svelte'
  import QueuePanel from './components/QueuePanel.svelte'
  import VideoView from './components/VideoView.svelte'
  import MiniPlayer from './components/MiniPlayer.svelte'
  import { enterMini, exitMini } from './lib/mini'
  import AlbumGrid from './views/AlbumGrid.svelte'
  import AlbumView from './views/AlbumView.svelte'
  import Songs from './views/Songs.svelte'
  import Artists from './views/Artists.svelte'
  import Genres from './views/Genres.svelte'
  import Videos from './views/Videos.svelte'
  import Search from './views/Search.svelte'
  import Settings from './views/Settings.svelte'
  import Welcome from './views/Welcome.svelte'
  import PlaylistView from './views/PlaylistView.svelte'
  import Top from './views/Top.svelte'
  import EqView from './views/EqView.svelte'
  import RadioView from './views/Radio.svelte'
  import SpotifyView from './views/Spotify.svelte'
  import Vocabulary from './views/Vocabulary.svelte'
  import Composers from './views/Composers.svelte'
  import Duplicates from './views/Duplicates.svelte'
  import SmartEditor from './components/SmartEditor.svelte'
  import TagEditor from './components/TagEditor.svelte'
  import { createPlaylist, updatePlaylist } from './lib/library.svelte'
  import type { Rules } from './lib/smart'
  import { lib } from './lib/library.svelte'
  import { nav, ui } from './lib/nav.svelte'
  import { queue } from './lib/queue.svelte'
  import { mpv, startMpv, togglePause } from './lib/player.svelte'
  import { openFiles } from './lib/open'
  import './lib/media.svelte'
  import './lib/settings.svelte'
  import './lib/lyrics.svelte'
  import './lib/stats.svelte'
  import './lib/eq.svelte'
  import './lib/lastfm.svelte'
  import './lib/sleep.svelte'

  startMpv().then(() => lib.load()).then(() => queue.restore(lib.byId))

  let sidebar: Sidebar
  let content = $state<HTMLDivElement>()
  const r = $derived(nav.route)
  // full-height views manage their own scrolling (virtual lists, split panes)
  const fixed = $derived(r.name === 'songs' || r.name === 'artists' || r.name === 'composers')
  const LOVED: Rules = { match: 'all', rules: [{ field: 'loved', op: 'isTrue', value: '' }], limit: { count: 100000, by: 'recentlyAdded' } }
  const RECENT: Rules = { match: 'all', rules: [{ field: 'lastPlayed', op: 'inLast', value: 3650, unit: 'days' }], limit: { count: 100, by: 'recentlyPlayed' } }

  async function saveSmart(name: string, rules: Rules) {
    const e = ui.editor
    ui.editor = null
    if (e?.id) await updatePlaylist(e.id, name, JSON.stringify(rules))
    else nav.root({ name: 'playlist', id: await createPlaylist(name, JSON.stringify(rules)) })
  }

  const video = $derived(
    !mpv.idle && !!queue.current && (queue.current.kind === 'video' || mpv.tracks.some((x) => x.type === 'video' && !x.image)),
  )

  $effect(() => {
    r
    content?.scrollTo({ top: 0 })
  })

  function keys(e: KeyboardEvent) {
    // sliders keep arrows for themselves, but Space still plays/pauses
    const typing = (e.target as HTMLElement).closest('input, select, textarea')
    if (e.ctrlKey && e.key === 'f') return e.preventDefault(), sidebar.focusSearch()
    if (e.ctrlKey && e.key === 'o') return e.preventDefault(), openFiles()
    if (e.ctrlKey && e.shiftKey && e.key.toLowerCase() === 'm') return e.preventDefault(), ui.mini ? exitMini() : enterMini()
    if (e.ctrlKey && e.key === 'i' && queue.current && queue.current.id > 0 && !video) return e.preventDefault(), (ui.tagEditor = [queue.current])
    if (typing) return
    if (e.key === ' ') {
      e.preventDefault()
      if (queue.current) togglePause()
    } else if (e.ctrlKey && e.key === 'ArrowRight') queue.next()
    else if (e.ctrlKey && e.key === 'ArrowLeft') queue.prev()
    else if (e.key === 'Escape') ui.nowPlaying ? (ui.nowPlaying = false) : (ui.queue = false)
    else if (e.altKey && e.key === 'ArrowLeft') nav.back()
  }
</script>

<svelte:window onkeydown={keys} onmouseup={(e) => e.button === 3 && nav.back()} />

{#if ui.mini}<MiniPlayer />{/if}
<div class="app" class:video class:hidden={ui.mini}>
  <Sidebar bind:this={sidebar} />
  <main class="main">
    <TopBar />
    <div class="content" class:fixed bind:this={content}>
      {#if !lib.loaded}
        <!-- library loads in a few ms; avoid a flash of the welcome screen -->
      {:else if !lib.folders.length}
        <Welcome />
      {:else if r.name === 'recent'}
        <AlbumGrid albums={lib.recent} title="Recently Added" />
      {:else if r.name === 'albums'}
        <AlbumGrid albums={lib.albums} title="Albums" />
      {:else if r.name === 'album'}
        <AlbumView key={r.key} />
      {:else if r.name === 'songs'}
        <Songs />
      {:else if r.name === 'artists'}
        <Artists artist={r.artist} />
      {:else if r.name === 'genres'}
        <Genres genre={r.genre} />
      {:else if r.name === 'videos'}
        <Videos />
      {:else if r.name === 'search'}
        <Search />
      {:else if r.name === 'settings'}
        <Settings />
      {:else if r.name === 'playlist'}
        {#key r.id}<PlaylistView id={r.id} />{/key}
      {:else if r.name === 'loved'}
        <PlaylistView title="Loved" builtin={LOVED} subtitle="Songs you've loved, newest first." />
      {:else if r.name === 'recentPlayed'}
        <PlaylistView title="Recently Played" builtin={RECENT} subtitle="The last 100 songs you played." />
      {:else if r.name === 'top'}
        <Top />
      {:else if r.name === 'eq'}
        <EqView />
      {:else if r.name === 'radio'}
        <RadioView />
      {:else if r.name === 'spotify'}
        <SpotifyView />
      {:else if r.name === 'composers'}
        <Composers composer={r.composer} />
      {:else if r.name === 'duplicates'}
        <Duplicates />
      {:else if r.name === 'vocabulary'}
        <Vocabulary />
      {/if}
    </div>
    {#if !ui.nowPlaying}<QueuePanel />{/if}
  </main>
</div>
{#if ui.mini}{:else if video}<VideoView />{:else}<NowPlaying />{/if}
{#if ui.editor}
  <SmartEditor name={ui.editor.name} rules={ui.editor.rules} onsave={saveSmart} oncancel={() => (ui.editor = null)} />
{/if}
{#if ui.tagEditor}
  <TagEditor tracks={ui.tagEditor} onclose={() => (ui.tagEditor = null)} />
{/if}
<ContextMenu />
{#if ui.toast}<div class="toast" role="status">{ui.toast}</div>{/if}

<style>
  .app { height: 100vh; display: flex; }
  .app.hidden { display: none; }
  .toast {
    position: fixed; z-index: 90; left: 50%; bottom: 24px; transform: translateX(-50%); max-width: 70vw;
    padding: 9px 16px; border-radius: 10px; background: var(--bar); backdrop-filter: blur(30px) saturate(180%);
    border: 1px solid var(--sep); box-shadow: var(--shadow-pop); animation: rise 220ms var(--ease-out);
  }
  @keyframes rise { from { opacity: 0; transform: translate(-50%, 8px); } }
  .main { position: relative; flex: 1; min-width: 0; background: var(--bg); }
  .content { position: absolute; inset: 0; overflow-y: auto; padding: calc(var(--bar-h) + 28px) 28px 48px; }
  .content.fixed { overflow: hidden; padding: 0; }

  /* video: mpv draws under the whole window; the library steps aside for the player */
  .app.video { visibility: hidden; }
</style>
