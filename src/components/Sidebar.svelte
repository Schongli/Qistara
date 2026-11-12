<script lang="ts">
  import Clock from 'phosphor-svelte/lib/Clock'
  import MicrophoneStage from 'phosphor-svelte/lib/MicrophoneStage'
  import SquaresFour from 'phosphor-svelte/lib/SquaresFour'
  import MusicNote from 'phosphor-svelte/lib/MusicNote'
  import Guitar from 'phosphor-svelte/lib/Guitar'
  import FilmStrip from 'phosphor-svelte/lib/FilmStrip'
  import Gear from 'phosphor-svelte/lib/Gear'
  import MagnifyingGlass from 'phosphor-svelte/lib/MagnifyingGlass'
  import FolderOpen from 'phosphor-svelte/lib/FolderOpen'
  import Radio from 'phosphor-svelte/lib/Radio'
  import SpotifyLogo from 'phosphor-svelte/lib/SpotifyLogo'
  import { spotify } from '../lib/spotify.svelte'
  import Cards from 'phosphor-svelte/lib/Cards'
  import { vocab } from '../lib/learn.svelte'
  import MusicNotesSimple from 'phosphor-svelte/lib/MusicNotesSimple'
  import Heart from 'phosphor-svelte/lib/Heart'
  import ClockCounterClockwise from 'phosphor-svelte/lib/ClockCounterClockwise'
  import ChartBar from 'phosphor-svelte/lib/ChartBar'
  import Playlist from 'phosphor-svelte/lib/Playlist'
  import Sparkle from 'phosphor-svelte/lib/Sparkle'
  import Plus from 'phosphor-svelte/lib/Plus'
  import { createPlaylist } from '../lib/library.svelte'
  import { ui } from '../lib/nav.svelte'
  import { openMenu } from '../lib/menu.svelte'
  import { importPlaylist } from '../lib/playlistio'
  import { openFiles } from '../lib/open'
  import { lib } from '../lib/library.svelte'
  import { nav, type Route } from '../lib/nav.svelte'

  const items: { label: string; route: Route; icon: typeof Clock }[] = [
    { label: 'Recently Added', route: { name: 'recent' }, icon: Clock },
    { label: 'Artists', route: { name: 'artists' }, icon: MicrophoneStage },
    { label: 'Albums', route: { name: 'albums' }, icon: SquaresFour },
    { label: 'Songs', route: { name: 'songs' }, icon: MusicNote },
    { label: 'Genres', route: { name: 'genres' }, icon: Guitar },
    { label: 'Composers', route: { name: 'composers' }, icon: MusicNotesSimple },
    { label: 'Videos', route: { name: 'videos' }, icon: FilmStrip },
    { label: 'Radio & Streams', route: { name: 'radio' }, icon: Radio },
  ]
  const mine: { label: string; route: Route; icon: typeof Clock }[] = [
    { label: 'Loved', route: { name: 'loved' }, icon: Heart },
    { label: 'Recently Played', route: { name: 'recentPlayed' }, icon: ClockCounterClockwise },
    { label: 'Your Top', route: { name: 'top' }, icon: ChartBar },
  ]
  const newMenu = (e: MouseEvent) =>
    openMenu(e, [
      { label: 'New Playlist', action: async () => nav.root({ name: 'playlist', id: await createPlaylist('Untitled Playlist') }) },
      { label: 'New Smart Playlist…', action: () => (ui.editor = { name: 'New Smart Playlist' }) },
      'sep',
      { label: 'Import Playlist (M3U)…', action: importPlaylist },
    ])
  // album pages belong to the Albums destination, etc.
  const section = $derived(nav.route.name === 'album' ? 'albums' : nav.route.name)

  let input: HTMLInputElement
  export const focusSearch = () => input.focus()
</script>

<nav class="side" aria-label="Library">
  <div class="drag" data-tauri-drag-region></div>
  <label class="search">
    <MagnifyingGlass size={14} weight="bold" />
    <input
      bind:this={input}
      bind:value={nav.query}
      placeholder="Search"
      spellcheck="false"
      oninput={() => nav.route.name !== 'search' && nav.go({ name: 'search' })}
      onkeydown={(e) => e.key === 'Escape' && ((nav.query = ''), input.blur(), nav.route.name === 'search' && nav.back())}
    />
  </label>

  <div class="head">Library</div>
  {#each items as it (it.label)}
    <button class="item press" class:sel={section === it.route.name} onclick={() => nav.root(it.route)}>
      <span class="ic"><it.icon size={17} /></span>{it.label}
    </button>
  {/each}
  {#if spotify.user}
    <button class="item press" class:sel={section === 'spotify'} onclick={() => nav.root({ name: 'spotify' })}>
      <span class="ic"><SpotifyLogo size={17} /></span>Spotify
    </button>
  {/if}

  <div class="head row">Playlists <button class="add" aria-label="New playlist" onclick={newMenu}><Plus size={12} weight="bold" /></button></div>
  {#each mine as it (it.label)}
    <button class="item press" class:sel={section === it.route.name} onclick={() => nav.root(it.route)}>
      <span class="ic"><it.icon size={17} /></span>{it.label}
    </button>
  {/each}
  <!-- appears once a word is saved from Learn mode in the lyrics -->
  {#if vocab.items.length}
    <button class="item press" class:sel={section === 'vocabulary'} onclick={() => nav.root({ name: 'vocabulary' })}>
      <span class="ic"><Cards size={17} /></span>Vocabulary
    </button>
  {/if}
  <div class="lists">
    {#each lib.playlists as p (p.id)}
      <button class="item press" class:sel={nav.route.name === 'playlist' && nav.route.id === p.id} onclick={() => nav.root({ name: 'playlist', id: p.id })}>
        <span class="ic">{#if p.rules}<Sparkle size={16} />{:else}<Playlist size={17} />{/if}</span><span class="ellipsis">{p.name}</span>
      </button>
    {/each}
  </div>

  <div class="spacer"></div>
  {#if lib.scan}
    <div class="scan" aria-live="polite">
      <div class="t-sub">Scanning {lib.scan.done.toLocaleString()} of {lib.scan.total.toLocaleString()}</div>
      <div class="bar"><div style:width="{(lib.scan.done / Math.max(1, lib.scan.total)) * 100}%"></div></div>
    </div>
  {/if}
  <button class="item press" onclick={openFiles} title="Ctrl+O">
    <span class="ic"><FolderOpen size={17} /></span>Open File…
  </button>
  <button class="item press" class:sel={section === 'settings'} onclick={() => nav.root({ name: 'settings' })}>
    <span class="ic"><Gear size={17} /></span>Settings
  </button>
</nav>

<style>
  .side {
    width: 232px; flex: none; display: flex; flex-direction: column; gap: 1px;
    padding: 0 10px 12px; background: var(--sidebar); border-right: 1px solid var(--sep);
  }
  .drag { height: 18px; flex: none; }
  .search {
    display: flex; align-items: center; gap: 7px; height: 30px; padding: 0 9px; margin: 22px 2px 14px;
    border-radius: 7px; background: var(--fill-2); color: var(--text-2);
  }
  .search:focus-within { box-shadow: 0 0 0 2px var(--accent); }
  input { flex: 1; min-width: 0; border: 0; background: none; outline: none; color: var(--text); }
  input::placeholder { color: var(--text-2); }
  .head { font-size: 11px; font-weight: 650; color: var(--text-3); padding: 10px 10px 5px; }
  .item { display: flex; align-items: center; gap: 9px; height: 30px; padding: 0 10px; border-radius: 7px; text-align: left; }
  .item:hover { background: var(--fill); }
  .item.sel { background: var(--fill-2); font-weight: 550; }
  .ic { display: grid; color: var(--accent); }
  .spacer { flex: 1; }
  .head.row { display: flex; justify-content: space-between; align-items: center; padding-top: 16px; }
  .add { width: 20px; height: 20px; border-radius: 5px; display: grid; place-items: center; color: var(--text-2); }
  .add:hover { background: var(--fill-2); color: var(--text); }
  .lists { display: grid; gap: 1px; overflow-y: auto; min-height: 0; }
  .item { flex: none; min-width: 0; }
  .scan { padding: 8px 10px 10px; }
  .bar { height: 3px; margin-top: 6px; border-radius: 2px; background: var(--fill-2); overflow: hidden; }
  .bar div { height: 100%; background: var(--accent); transition: width 200ms; }
</style>
