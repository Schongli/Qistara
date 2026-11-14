<script lang="ts">
  // Get Info: edit tags for one or many songs. Fields that differ show "Mixed" and stay as they are
  // unless you type in them. MusicBrainz fills album fields, per-track titles/numbers and artwork.
  import { untrack } from 'svelte'
  import { convertFileSrc, invoke } from '@tauri-apps/api/core'
  import { open } from '@tauri-apps/plugin-dialog'
  import Icon from './Icon.svelte'
  import Artwork from './Artwork.svelte'
  import { lib, type Track } from '../lib/library.svelte'
  import { coverUrl, matchTracks, release, searchReleases, type Release } from '../lib/musicbrainz'
  import { plural, quality, time } from '../lib/format'

  let { tracks, onclose }: { tracks: Track[]; onclose: () => void } = $props()
  const files = untrack(() => tracks)
  const single = files.length === 1

  type F = 'title' | 'artist' | 'albumArtist' | 'album' | 'genre' | 'composer' | 'year' | 'track' | 'disc'
  const all: [F, string][] = [
    ['title', 'Title'], ['artist', 'Artist'], ['albumArtist', 'Album Artist'], ['album', 'Album'], ['genre', 'Genre'],
    ['composer', 'Composer'], ['year', 'Year'], ['track', 'Track'], ['disc', 'Disc'],
  ]
  const shown = all.filter(([f]) => single || (f !== 'title' && f !== 'track'))
  const common = (f: F) => {
    const vals = new Set(files.map((t) => String(t[f] ?? '')))
    return vals.size === 1 ? [...vals][0] : null
  }
  let values = $state(Object.fromEntries(all.map(([f]) => [f, common(f) ?? ''])) as Record<F, string>)
  const mixed = Object.fromEntries(all.map(([f]) => [f, common(f) === null])) as Record<F, boolean>
  let dirty = $state<F[]>([])
  const touch = (f: F) => !dirty.includes(f) && dirty.push(f)

  let perTrack = $state<Record<string, Partial<Record<F, string>>>>({})
  let cover = $state<string | null>(null) // staged image path; '' = remove artwork
  let tab = $state<'details' | 'artwork' | 'lookup' | 'file'>('details')
  let busy = $state(false)
  let errors = $state<string[]>([])

  async function chooseImage() {
    const f = await open({ multiple: false, filters: [{ name: 'Images', extensions: ['jpg', 'jpeg', 'png'] }] })
    if (typeof f !== 'string') return
    try {
      cover = await invoke<string>('stage_image', { path: f })
    } catch (e) {
      errors = [String(e)]
    }
  }

  let results = $state<Release[]>([])
  let looking = $state('')
  async function lookup() {
    looking = 'Searching MusicBrainz…'
    try {
      results = await searchReleases(values.album || files[0].album, values.albumArtist || values.artist || files[0].artist)
      looking = results.length ? '' : 'No releases found. Edit the album or artist name and search again.'
    } catch (e) {
      looking = String(e)
    }
  }
  async function pick(r: Release) {
    looking = `Loading “${r.title}”…`
    try {
      const d = await release(r.id)
      const set = (f: F, v: string) => v && ((values[f] = v), touch(f))
      set('album', d.title)
      set('albumArtist', d.artist)
      set('year', d.year)
      set('genre', d.genre)
      const m = matchTracks(files, d.tracklist)
      perTrack = Object.fromEntries(files.map((t, i) => [t.path, m[i] ? { title: m[i]!.title, artist: m[i]!.artist, track: String(m[i]!.position), disc: String(m[i]!.disc) } : {}]))
      looking = `Matched ${plural(m.filter(Boolean).length, 'track')}. Fetching artwork…`
      cover = await invoke<string>('fetch_cover', { url: coverUrl(r.id) }).catch(() => cover)
      looking = `Applied “${d.title}”. Review, then Save.`
      tab = 'details'
    } catch (e) {
      looking = String(e)
    }
  }

  async function save() {
    busy = true
    const edits = files.map((t) => {
      const edit: Record<string, string> = Object.fromEntries(dirty.map((f) => [f, values[f]]))
      Object.assign(edit, perTrack[t.path] ?? {})
      if (cover !== null) edit.cover = cover
      return { path: t.path, edit }
    })
    errors = await invoke<string[]>('tag_write', { edits }).catch((e) => [String(e)])
    busy = false
    await lib.load()
    if (!errors.length) onclose()
  }
  const changes = $derived(dirty.length + Object.keys(perTrack).length + (cover !== null ? 1 : 0))
  const q = quality(files[0])
</script>

<div class="scrim" role="presentation" onclick={onclose}></div>
<div class="sheet" role="dialog" aria-modal="true" aria-label="Get info" tabindex="-1" onkeydown={(e) => e.key === 'Escape' && onclose()}>
  <header>
    <span class="cov">
      {#if cover}<Artwork art={null} src={convertFileSrc(cover)} size="56px" radius="6px" />{:else}<Artwork art={cover === '' ? null : files[0].art} size="56px" radius="6px" />{/if}
    </span>
    <div class="ttl">
      <div class="t-h2 ellipsis">{single ? files[0].title : `${plural(files.length, 'song')}`}</div>
      <div class="t-sub ellipsis">{single ? `${files[0].artist} — ${files[0].album}` : values.album || 'Multiple albums'}</div>
    </div>
  </header>
  <div class="tabs" role="tablist">
    {#each [['details', 'Details'], ['artwork', 'Artwork'], ['lookup', 'MusicBrainz'], ['file', 'File']] as [k, label] (k)}
      <button role="tab" aria-selected={tab === k} class:sel={tab === k} onclick={() => { tab = k as typeof tab; if (k === 'lookup' && !results.length && !looking) lookup() }}>{label}</button>
    {/each}
  </div>

  <div class="body">
    {#if tab === 'details'}
      <div class="grid">
        {#each shown as [f, label] (f)}
          <label class:short={f === 'year' || f === 'track' || f === 'disc'}>
            <span>{label}</span>
            <input
              bind:value={values[f]}
              placeholder={mixed[f] && !dirty.includes(f) ? 'Mixed' : ''}
              inputmode={f === 'year' || f === 'track' || f === 'disc' ? 'numeric' : 'text'}
              oninput={() => touch(f)}
            />
          </label>
        {/each}
      </div>
      {#if Object.keys(perTrack).length}<p class="t-sub">Titles and track numbers come from MusicBrainz for each song.</p>{/if}
    {:else if tab === 'artwork'}
      <div class="art">
        {#if cover}<Artwork art={null} src={convertFileSrc(cover)} size="220px" />{:else if cover === ''}<Artwork art={null} size="220px" />{:else}<Artwork art={files[0].art} size="220px" />{/if}
        <div class="col">
          <button class="btn press" onclick={chooseImage}>Choose Image…</button>
          <button class="btn press" onclick={() => (cover = '')}>Remove Artwork</button>
          <p class="t-sub">The image is embedded in every selected file as the front cover.</p>
        </div>
      </div>
    {:else if tab === 'lookup'}
      <div class="lookup">
        <button class="btn press" onclick={lookup}><Icon name="MagnifyingGlass" size={14} />Search “{values.album || files[0].album}”</button>
        {#if looking}<p class="t-sub">{looking}</p>{/if}
        <div class="results">
          {#each results as r (r.id)}
            <button class="result" onclick={() => pick(r)}>
              <span class="ellipsis"><b>{r.title}</b> — {r.artist}</span>
              <span class="t-sub">{[r.date, r.country, r.format, plural(r.tracks, 'track')].filter(Boolean).join(' · ')}</span>
            </button>
          {/each}
        </div>
        <p class="t-sub credit">Data from MusicBrainz and the Cover Art Archive.</p>
      </div>
    {:else}
      <dl>
        {#each files.slice(0, 1) as t (t.id)}
          <dt>Path</dt><dd class="path">{t.path}</dd>
          <dt>Format</dt><dd>{q.detail || t.codec}{t.channels ? ` · ${t.channels} ch` : ''}</dd>
          <dt>Duration</dt><dd>{time(t.duration)}</dd>
          <dt>Plays</dt><dd>{t.plays} · {t.skips} skips</dd>
        {/each}
      </dl>
    {/if}
  </div>

  {#if errors.length}
    <div class="errors" role="alert">{#each errors as e, i (i)}<div>{e}</div>{/each}</div>
  {/if}
  <footer>
    <span class="t-sub">Files are written safely: if anything fails, the original stays as it was.</span>
    <button class="btn press" onclick={onclose}>Cancel</button>
    <button class="btn primary press" onclick={save} disabled={busy || !changes}>{busy ? 'Saving…' : 'Save'}</button>
  </footer>
</div>

<style>
  .scrim { position: fixed; inset: 0; z-index: 80; background: var(--sheet-dim); animation: fade 200ms; }
  .sheet {
    position: fixed; z-index: 81; top: 0; left: 50%; width: min(640px, 92vw); transform: translateX(-50%);
    padding: 18px 20px 16px; border-radius: 0 0 var(--r-xl) var(--r-xl); background: var(--bg-2); border: 1px solid var(--sep); border-top: 0;
    box-shadow: var(--shadow-pop); animation: drop 280ms var(--ease-out);
  }
  @keyframes drop { from { transform: translate(-50%, -100%); } }
  @keyframes fade { from { opacity: 0; } }
  header { display: flex; gap: 14px; align-items: center; }
  .ttl { min-width: 0; }
  .tabs { display: flex; gap: 2px; margin: 14px 0 12px; padding: 2px; border-radius: var(--r); background: var(--fill-2); width: fit-content; }
  .tabs button { height: 26px; padding: 0 14px; border-radius: var(--r-sm); color: var(--text-2); font-weight: 550; }
  .tabs button.sel { background: var(--bg); color: var(--text); box-shadow: 0 1px 3px rgba(0, 0, 0, 0.12); }
  .body { min-height: 260px; }
  .grid { display: grid; grid-template-columns: repeat(6, 1fr); gap: 10px 12px; }
  .grid label { grid-column: span 6; display: grid; grid-template-columns: 96px 1fr; align-items: center; gap: 10px; }
  .grid label.short { grid-column: span 2; grid-template-columns: auto 1fr; }
  .grid span { color: var(--text-2); text-align: right; }
  input { height: 28px; padding: 0 8px; border-radius: var(--r-sm); border: 1px solid var(--sep); background: var(--bg); color: var(--text); min-width: 0; }
  input:focus { border-color: var(--accent); }
  input::placeholder { color: var(--text-3); font-style: italic; }
  .art { display: flex; gap: 20px; align-items: flex-start; }
  .col { display: grid; gap: 8px; justify-items: start; }
  .lookup { display: grid; gap: 10px; }
  .lookup .btn { width: fit-content; }
  .results { display: grid; gap: 2px; max-height: 220px; overflow-y: auto; }
  .result { display: grid; text-align: left; padding: 7px 10px; border-radius: var(--r); }
  .result:hover { background: var(--fill-2); }
  .credit { font-size: 11px; }
  dl { display: grid; grid-template-columns: 80px 1fr; gap: 6px 12px; margin: 0; }
  dt { color: var(--text-2); }
  dd { margin: 0; }
  .path { word-break: break-all; user-select: text; }
  .errors { margin-top: 10px; padding: 8px 10px; border-radius: var(--r); background: rgba(229, 72, 77, 0.12); color: #e5484d; font-size: 12px; }
  footer { display: flex; align-items: center; gap: 10px; margin-top: 16px; }
  footer .t-sub { flex: 1; font-size: 11px; }
  .btn:disabled { opacity: 0.5; }
  @media (prefers-reduced-motion: reduce) { .sheet { animation: fade 150ms; } }
</style>
