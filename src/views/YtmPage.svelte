<script lang="ts">
  // A YT Music album, playlist or artist: the header with Play, Shuffle and Radio, the songs, then the shelves.
  import Icon from '../components/Icon.svelte'
  import Artwork from '../components/Artwork.svelte'
  import TrackTable from '../components/TrackTable.svelte'
  import RemoteGrid from '../components/RemoteGrid.svelte'
  import { nav } from '../lib/nav.svelte'
  import { queue } from '../lib/queue.svelte'
  import { page, playItem, songTrack, startRadio, type Item, type Page } from '../lib/ytmusic.svelte'

  let { kind, id }: { kind: 'album' | 'artist' | 'playlist'; id: string } = $props()
  let p = $state<Page | null>(null)
  let err = $state('')
  $effect(() => {
    p = null
    err = ''
    const want = id
    page(kind, want).then((r) => id === want && (p = r), (e) => id === want && (err = String(e)))
  })
  const tracks = $derived(p?.tracks.map(songTrack) ?? [])
  const LABEL = { album: 'Album', playlist: 'Playlist', artist: 'Artist' }

  function open(i: Item) {
    if (i.kind === 'song') return playItem(i)
    nav.go(i.kind === 'album' ? { name: 'ytmAlbum', id: i.id } : i.kind === 'artist' ? { name: 'ytmArtist', id: i.id } : { name: 'ytmPlaylist', id: i.id })
  }
  const artistLink = (a: { id: string | null }) => a.id && nav.go({ name: 'ytmArtist', id: a.id })
</script>

{#if err}
  <p class="err">{err}</p>
{:else if !p}
  <p class="t-sub">Loading…</p>
{:else}
  <header class="hero" class:artist={kind === 'artist'}>
    <Artwork art={p.thumb} size="200px" radius={kind === 'artist' ? '50%' : undefined} />
    <div class="meta">
      <span class="kind">{LABEL[kind]}</span>
      <h1 class="t-title">{p.title}</h1>
      {#if p.artists.length && kind !== 'artist'}
        <div class="by">
          {#each p.artists as a, n (n)}{#if n}, {/if}{#if a.id}<button class="link" onclick={() => artistLink(a)}>{a.name}</button>{:else}{a.name}{/if}{/each}
        </div>
      {/if}
      {#if p.subtitle}<div class="t-sub">{p.subtitle}</div>{/if}
      <div class="acts">
        <button class="btn primary press" disabled={!tracks.length} onclick={() => queue.play(tracks)}><Icon name="Play" size={15} weight="fill" />Play</button>
        <button class="btn press" disabled={!tracks.length} onclick={() => queue.play(tracks, Math.floor(Math.random() * tracks.length), true)}><Icon name="Shuffle" size={15} />Shuffle</button>
        {#if tracks.length}<button class="btn press" onclick={() => startRadio(tracks[0])}><Icon name="Radio" size={15} />Radio</button>{/if}
      </div>
    </div>
  </header>

  {#if tracks.length}
    <section>
      {#if kind === 'artist'}<h2 class="t-h2">Top songs</h2>{/if}
      <TrackTable {tracks} remote />
    </section>
  {/if}
  {#each p.shelves as s, n (n + s.title)}
    <section>
      <h2 class="t-h2">{s.title}</h2>
      <RemoteGrid items={s.items} row onopen={open} onplay={(i) => playItem(i)} />
    </section>
  {/each}
{/if}

<style>
  .hero { display: flex; align-items: flex-end; gap: 24px; margin-bottom: 28px; flex-wrap: wrap; }
  .meta { display: grid; gap: 6px; min-width: 0; flex: 1 1 280px; }
  .kind { font-size: 11px; font-weight: 650; letter-spacing: 0.06em; text-transform: uppercase; color: var(--text-2); }
  h1 { margin: 0; overflow-wrap: anywhere; }
  .by { font-weight: 600; }
  .link:hover { text-decoration: underline; text-underline-offset: 2px; }
  .acts { display: flex; flex-wrap: wrap; gap: 8px; margin-top: 10px; }
  .btn:disabled { opacity: 0.5; }
  section { margin-bottom: 30px; }
  section h2 { margin: 0 0 12px; }
  .err { color: var(--signal-veil); }
</style>
