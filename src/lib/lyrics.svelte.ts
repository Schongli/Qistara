// Finds lyrics for the playing track, asking the sources in the order set in Settings › Lyrics.
import { invoke } from '@tauri-apps/api/core'
import { untrack } from 'svelte'
import { parseLyrics, type Lyrics } from './lyrics'
import { firstLyrics, lookupSong, serverSongId, youtubeSongId, type LyricsSource } from './lyricsources'
import { queue } from './queue.svelte'
import { settings } from './settings.svelte'
import type { Track } from './library.svelte'

export const lyr = $state({ data: null as Lyrics | null, source: '' as LyricsSource | '', loading: false })

async function ask(t: Track, id: LyricsSource): Promise<Lyrics | null> {
  if (id === 'local') {
    const f = await invoke<{ kind: string; text: string } | null>('lyrics', { path: t.path })
    return f?.text.trim() ? parseLyrics(f.kind, f.text) : null
  }
  const sid = serverSongId(t.path)
  const yid = youtubeSongId(t.path)
  if (t.kind !== 'audio' || !t.title) return null
  if (id === 'subsonic' ? !sid : id === 'ytmusic' ? !yid : !t.artist) return null
  // a server song's address is signed afresh each session, so it is cached by its id instead
  // a listening session's song is a loopback URL with a new token each time: cache it by what it is instead
  const key = sid ? `sub:${sid}` : t.path.startsWith('http://127.0.0.1:') ? `session:${t.artist}|${t.title}` : t.path
  // a YouTube song is asked about by its song title, not the video's (lyricsources.ts lookupSong)
  const asked = yid ? lookupSong(t) : { title: t.title, artist: t.artist, album: t.album, duration: t.duration }
  const song = { ...asked, id: id === 'ytmusic' ? yid : sid }
  const text = await invoke<string>('lyrics_fetch', { source: id, key, song })
  return text.trim() ? parseLyrics('lrc', text) : null
}

let generation = 0
async function load(t: Track) {
  const mine = ++generation
  lyr.loading = true
  lyr.data = null
  lyr.source = ''
  try {
    const found = await firstLyrics($state.snapshot(settings.lyricsSources), (id) => ask(t, id), settings.preferWordSync)
    if (mine !== generation) return // the track or the source list changed while we were looking
    lyr.data = found?.lyrics ?? null
    lyr.source = found?.source ?? ''
  } finally {
    if (mine === generation) lyr.loading = false
  }
}

$effect.root(() => {
  $effect(() => {
    const t = queue.current
    // a reordered or switched source list looks again (online results are cached per source)
    settings.lyricsSources.map((s) => s.on)
    settings.preferWordSync
    if (t) untrack(() => load(t))
    else lyr.data = null
  })
})
