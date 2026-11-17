<script lang="ts">
  import { tick } from 'svelte'
  import Sidebar from './components/Sidebar.svelte'
  import TopBar from './components/TopBar.svelte'
  import PlayerBar from './components/PlayerBar.svelte'
  import RightPanel from './components/RightPanel.svelte'
  import Caps from './components/Caps.svelte'
  import Home from './views/Home.svelte'
  import Downloads from './views/Downloads.svelte'
  import YouTube from './views/YouTube.svelte'
  import YtMusic from './views/YtMusic.svelte'
  import YtmPage from './views/YtmPage.svelte'
  import Providers from './views/Providers.svelte'
  import Server from './views/Server.svelte'
  import ServiceView from './views/ServiceView.svelte'
  import ContextMenu from './components/ContextMenu.svelte'
  import NowPlaying from './components/NowPlaying.svelte'
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
  import SurahPicker from './components/SurahPicker.svelte'
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
  import './lib/appearance.svelte'
  import './lib/discord.svelte'
  import './lib/session.svelte'
  import { capture, restore } from './lib/scrollmemo'

  startMpv().then(() => lib.load()).then(() => queue.restore(lib.byId))

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
    // a stream (YouTube) may list video tracks it isn't playing: only a selected one counts
    !mpv.idle && !!queue.current && (queue.current.kind === 'video' || mpv.tracks.some((x) => x.type === 'video' && !x.image && x.selected)),
  )

  // Back/Forward return to where the page was; a new page starts at the top
  nav.capture = () => (content ? capture(content) : undefined)
  $effect(() => {
    r
    const to = nav.restore
    tick().then(() => content && restore(content, to))
  })

  function keys(e: KeyboardEvent) {
    // sliders keep arrows for themselves, but Space still plays/pauses
    const typing = (e.target as HTMLElement).closest('input, select, textarea')
    if (e.ctrlKey && e.key === 'f') return e.preventDefault(), nav.route.name === 'settings' ? ui.settingsSearch++ : ui.search++
    if (e.ctrlKey && e.key === 'o') return e.preventDefault(), openFiles()
    if (e.ctrlKey && e.shiftKey && e.key.toLowerCase() === 'm') return e.preventDefault(), ui.mini ? exitMini() : enterMini()
    if (e.ctrlKey && e.key === 'i' && queue.current && queue.current.id > 0 && !video) return e.preventDefault(), (ui.tagEditor = [queue.current])
    if (typing) return
    if (e.key === ' ') {
      e.preventDefault()
      if (queue.current) togglePause()
    } else if (e.ctrlKey && e.key === 'ArrowRight') queue.next()
    else if (e.ctrlKey && e.key === 'ArrowLeft') queue.prev()
    else if (e.key === 'Escape') ui.nowPlaying ? (ui.nowPlaying = false) : (ui.panel = '')
    else if (e.altKey && e.key === 'ArrowLeft') nav.back()
  }
</script>

<svelte:window onkeydown={keys} onmouseup={(e) => e.button === 3 && nav.back()} />

{#if ui.mini}<MiniPlayer />{/if}
<div class="app" class:video class:hidden={ui.mini}>
  <div class="row">
  <Sidebar />
  <main class="main">
    <TopBar caps={!ui.panel} />
    <div class="content" class:fixed bind:this={content}>
      {#if !lib.loaded}
        <!-- library loads in a few ms; avoid a flash of the welcome screen -->
      {:else if !lib.folders.length && r.name === 'home'}
        <Welcome />
      {:else if r.name === 'home'}
        <Home />
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
        <Settings tab={r.tab} />
      {:else if r.name === 'playlist'}
        {#key r.id}<PlaylistView id={r.id} />{/key}
      {:else if r.name === 'loved'}
        <PlaylistView title="Loved" builtin={LOVED} subtitle="Songs you've loved, newest first." />
      {:else if r.name === 'recentPlayed'}
        <PlaylistView title="History" builtin={RECENT} subtitle="The last 100 songs you played." />
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
      {:else if r.name === 'downloads'}
        <Downloads />
      {:else if r.name === 'youtube'}
        <YouTube />
      {:else if r.name === 'ytm'}
        <YtMusic />
      {:else if r.name === 'ytmAlbum' || r.name === 'ytmArtist' || r.name === 'ytmPlaylist'}
        {#key r.id}<YtmPage kind={r.name === 'ytmAlbum' ? 'album' : r.name === 'ytmArtist' ? 'artist' : 'playlist'} id={r.id} />{/key}
      {:else if r.name === 'providers'}
        <Providers />
      {:else if r.name === 'server'}
        <Server />
      {:else if r.name === 'apple' || r.name === 'tidal'}
        {#key r.name}<ServiceView kind={r.name} />{/key}
      {/if}
    </div>
  </main>
  {#if ui.panel}<RightPanel />{/if}
  </div>
  <PlayerBar />
  <!-- over the docked pane's dark backdrop the caption glyphs turn white -->
  <Caps light={!!ui.panel} />
</div>
{#if ui.mini}{:else if video}<VideoView />{:else}<NowPlaying />{/if}
{#if ui.editor}
  <SmartEditor name={ui.editor.name} rules={ui.editor.rules} onsave={saveSmart} oncancel={() => (ui.editor = null)} />
{/if}
{#if ui.tagEditor}
  <TagEditor tracks={ui.tagEditor} onclose={() => (ui.tagEditor = null)} />
{/if}
{#if ui.quranPick}
  <SurahPicker track={ui.quranPick} onclose={() => (ui.quranPick = null)} />
{/if}
<ContextMenu />
{#if ui.toast}<div class="toast" role="status">{ui.toast}</div>{/if}

<style>
  .app { position: relative; height: 100vh; display: flex; flex-direction: column; }
  .row { flex: 1; min-height: 0; display: flex; }
  .app.hidden { display: none; }
  .toast {
    position: fixed; z-index: 90; left: 50%; bottom: calc(var(--player-h) + 16px); transform: translateX(-50%); max-width: 70vw;
    padding: 9px 16px; border-radius: var(--r-lg); background: var(--bar); backdrop-filter: var(--blur);
    border: 1px solid var(--sep); box-shadow: var(--shadow-pop); animation: rise 220ms var(--ease-out);
  }
  @keyframes rise { from { opacity: 0; transform: translate(-50%, 8px); } }
  .main { position: relative; flex: 1; min-width: 0; background: var(--bg); }
  .content { position: absolute; inset: 0; overflow-y: auto; padding: calc(var(--bar-h) + 28px) 28px 48px; }
  .content.fixed { overflow: hidden; padding: 0; }

  /* video: mpv draws under the whole window; the library steps aside for the player */
  .app.video { visibility: hidden; }
</style>
