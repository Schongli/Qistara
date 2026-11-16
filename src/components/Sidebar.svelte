<script lang="ts" module>
  import Icon from './Icon.svelte'
  import { server } from '../lib/subsonic.svelte'
  import { apple, tidal } from '../lib/services.svelte'
  import Artwork from './Artwork.svelte'
  import { spotify } from '../lib/spotify.svelte'
  import { vocab } from '../lib/learn.svelte'
  import { createPlaylist, lib } from '../lib/library.svelte'
  import { nav, ui, type Route } from '../lib/nav.svelte'
  import { openMenu } from '../lib/menu.svelte'
  import { importPlaylist } from '../lib/playlistio'
  import { openFiles } from '../lib/open'
  import { pins, togglePin } from '../lib/pins.svelte'
  import { settings } from '../lib/settings.svelte'

  type Item = { id: string; label: string; route: Route; icon: string; when?: () => boolean }
  /** Every destination that Settings › Appearance can hide (by id). */
  export const NAV: { group: string; items: Item[] }[] = [
    { group: '', items: [
      { id: 'home', label: 'Home', route: { name: 'home' }, icon: 'House' },
      { id: 'search', label: 'Search', route: { name: 'search' }, icon: 'MagnifyingGlass' },
    ] },
    { group: 'Library', items: [
      { id: 'loved', label: 'Favorites', route: { name: 'loved' }, icon: 'Heart' },
      { id: 'albums', label: 'Albums', route: { name: 'albums' }, icon: 'SquaresFour' },
      { id: 'artists', label: 'Artists', route: { name: 'artists' }, icon: 'MicrophoneStage' },
      { id: 'songs', label: 'Songs', route: { name: 'songs' }, icon: 'MusicNote' },
      { id: 'recent', label: 'Recently Added', route: { name: 'recent' }, icon: 'Clock' },
      { id: 'genres', label: 'Genres', route: { name: 'genres' }, icon: 'Guitar' },
      { id: 'composers', label: 'Composers', route: { name: 'composers' }, icon: 'MusicNotesSimple' },
      { id: 'videos', label: 'Videos', route: { name: 'videos' }, icon: 'FilmStrip' },
    ] },
    { group: 'Sources', items: [
      { id: 'youtube', label: 'YouTube', route: { name: 'youtube' }, icon: 'YoutubeLogo' },
      { id: 'server', label: 'Server', route: { name: 'server' }, icon: 'HardDrives', when: () => !!server.account },
      { id: 'spotify', label: 'Spotify', route: { name: 'spotify' }, icon: 'SpotifyLogo', when: () => !!spotify.user },
      { id: 'apple', label: 'Apple Music', route: { name: 'apple' }, icon: 'AppleLogo', when: () => !!apple.s.user },
      { id: 'tidal', label: 'Tidal', route: { name: 'tidal' }, icon: 'Waves', when: () => !!tidal.s.user },
      { id: 'radio', label: 'Radio & Streams', route: { name: 'radio' }, icon: 'Radio' },
      { id: 'downloads', label: 'Downloads', route: { name: 'downloads' }, icon: 'DownloadSimple' },
      { id: 'providers', label: 'Providers', route: { name: 'providers' }, icon: 'PlugsConnected' },
    ] },
    { group: 'You', items: [
      { id: 'history', label: 'History', route: { name: 'recentPlayed' }, icon: 'ClockCounterClockwise' },
      { id: 'top', label: 'Your Top', route: { name: 'top' }, icon: 'ChartBar' },
      // appears once a word is saved from Learn mode
      { id: 'vocabulary', label: 'Vocabulary', route: { name: 'vocabulary' }, icon: 'Cards', when: () => vocab.items.length > 0 },
    ] },
  ]
</script>

<script lang="ts">
  import Logo from './Logo.svelte'
  const shown = (it: Item) => !settings.hiddenNav.includes(it.id) && (!it.when || it.when())

  const newMenu = (e: MouseEvent) =>
    openMenu(e, [
      { label: 'New Playlist', action: async () => nav.root({ name: 'playlist', id: await createPlaylist('Untitled Playlist') }) },
      { label: 'New Smart Playlist…', action: () => (ui.editor = { name: 'New Smart Playlist' }) },
      'sep',
      { label: 'Import Playlist (M3U)…', action: importPlaylist },
    ])
  // album pages belong to the Albums destination, etc.
  const section = $derived(nav.route.name === 'album' ? 'albums' : nav.route.name)
  const selected = (r: Route) => section === r.name
  let closed = $state<Record<string, boolean>>({})
</script>

<nav class="side" aria-label="Library">
  <div class="brand" data-tauri-drag-region>
    <Logo size={22} />
    <span data-tauri-drag-region>Qistara</span>
  </div>

  <div class="scroll">
    {#each NAV as g (g.group)}
      {@const items = g.items.filter(shown)}
      {#if items.length}
        {#if g.group}
          <button class="head press" aria-expanded={!closed[g.group]} onclick={() => (closed[g.group] = !closed[g.group])}>
            {g.group}<span class="caret" class:shut={closed[g.group]}><Icon name="CaretDown" size={10} weight="bold" /></span>
          </button>
        {/if}
        {#if !closed[g.group]}
          {#each items as it (it.id)}
            <button class="item press" class:sel={selected(it.route)} onclick={() => (nav.root(it.route), it.id === 'search' && ui.search++)}>
              <span class="ic"><Icon name={it.icon} size={17} /></span>{it.label}
            </button>
          {/each}
        {/if}
      {/if}
    {/each}

    <div class="head row">
      <button class="press grow" aria-expanded={!closed.Playlists} onclick={() => (closed.Playlists = !closed.Playlists)}>
        Playlists<span class="caret" class:shut={closed.Playlists}><Icon name="CaretDown" size={10} weight="bold" /></span>
      </button>
      <button class="add" aria-label="New playlist" onclick={newMenu}><Icon name="Plus" size={12} weight="bold" /></button>
    </div>
    {#if !closed.Playlists}
      {#each lib.playlists as p (p.id)}
        <button class="item press" class:sel={nav.route.name === 'playlist' && nav.route.id === p.id} onclick={() => nav.root({ name: 'playlist', id: p.id })}>
          <span class="ic">{#if p.rules}<Icon name="Sparkle" size={16} />{:else}<Icon name="Playlist" size={17} />{/if}</span><span class="ellipsis">{p.name}</span>
        </button>
      {/each}
    {/if}

    {#if pins.items.length}
      <div class="head">Pinned</div>
      {#each pins.items as p (p.key)}
        <button
          class="pin press"
          class:sel={nav.route.name === 'album' && nav.route.key === p.key}
          onclick={() => nav.root({ name: 'album', key: p.key })}
          oncontextmenu={(e) => openMenu(e, [{ label: 'Unpin from Sidebar', action: () => togglePin(p) }])}
        >
          <Artwork art={p.art} size="34px" radius="5px" shadow={false} />
          <span class="pt"><span class="ellipsis">{p.title}</span><span class="t-sub">Album</span></span>
        </button>
      {/each}
    {/if}
  </div>

  {#if lib.scan}
    <div class="scan" aria-live="polite">
      <div class="t-sub">Scanning {lib.scan.done.toLocaleString()} of {lib.scan.total.toLocaleString()}</div>
      <div class="bar"><div style:width="{(lib.scan.done / Math.max(1, lib.scan.total)) * 100}%"></div></div>
    </div>
  {/if}
  <div class="foot">
    <button class="item press" onclick={openFiles} title="Ctrl+O"><span class="ic"><Icon name="FolderOpen" size={17} /></span>Open File…</button>
    <button class="item press" class:sel={section === 'settings'} onclick={() => nav.root({ name: 'settings' })}><span class="ic"><Icon name="Gear" size={17} /></span>Settings</button>
  </div>
</nav>

<style>
  .side {
    width: 236px; flex: none; display: flex; flex-direction: column; min-height: 0;
    padding: 0 10px 10px; background: var(--sidebar); border-right: 1px solid var(--sep);
  }
  /* room for macOS's own traffic lights above the logo */
  :global(.os-macos) .side { padding-top: 30px; }
  .brand { display: flex; align-items: center; gap: 9px; height: var(--bar-h); padding: 0 8px; flex: none; }
  .brand span { font-weight: 700; font-size: 15px; letter-spacing: -0.02em; }
  .scroll { flex: 1; min-height: 0; overflow-y: auto; display: grid; align-content: start; gap: 1px; margin: 0 -4px; padding: 0 4px; }
  .head {
    display: flex; align-items: center; gap: 4px; width: 100%; font-size: 11px; font-weight: 650; color: var(--text-3);
    padding: 14px 10px 5px; text-align: left;
  }
  .head:hover { color: var(--text-2); }
  .caret { display: grid; transition: transform 160ms var(--ease-out); }
  .caret.shut { transform: rotate(-90deg); }
  .head.row { justify-content: space-between; padding-right: 4px; }
  .grow { display: flex; align-items: center; gap: 4px; }
  .item { display: flex; align-items: center; gap: 9px; height: 30px; padding: 0 10px; border-radius: var(--r); text-align: left; min-width: 0; }
  .item:hover, .pin:hover { background: var(--fill); }
  .item.sel, .pin.sel { background: var(--fill-2); font-weight: 550; }
  .ic { display: grid; color: var(--accent); }
  .add { width: 20px; height: 20px; border-radius: var(--r-xs); display: grid; place-items: center; color: var(--text-2); }
  .add:hover { background: var(--fill-2); color: var(--text); }
  .pin { display: flex; align-items: center; gap: 10px; padding: 4px 8px; border-radius: var(--r); text-align: left; min-width: 0; }
  .pt { display: grid; min-width: 0; line-height: 1.3; }
  .foot { display: grid; gap: 1px; padding-top: 6px; border-top: 1px solid var(--sep); margin-top: 6px; }
  .scan { padding: 8px 10px 6px; }
  .bar { height: 3px; margin-top: 6px; border-radius: 2px; background: var(--fill-2); overflow: hidden; }
  .bar div { height: 100%; background: var(--accent); transition: width 200ms; }
</style>
