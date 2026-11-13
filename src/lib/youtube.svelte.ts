// YouTube through yt-dlp (ytdlp.rs): search results, pasted links and Liked Music become tracks whose
// path is ytdl://<watch url>, which mpv's ytdl hook plays with our yt-dlp (player.svelte.ts).
import { invoke } from '@tauri-apps/api/core'
import { remoteTrack } from './radio.svelte'
import { ytCookies } from './downloads.svelte'
import { mpv, set } from './player.svelte'
import { settings } from './settings.svelte'
import type { Track } from './library.svelte'

type Video = { id: string; title: string; channel: string; duration: number | null }

export const yt = $state({ tracks: [] as Track[], title: '', loading: false, error: '' })

export const ytTrack = (v: Video) =>
  remoteTrack({
    path: `ytdl://https://www.youtube.com/watch?v=${v.id}`, title: v.title, artist: v.channel, album: 'YouTube',
    duration: v.duration ?? 0, codec: 'YouTube', art: `https://i.ytimg.com/vi/${v.id}/mqdefault.jpg`,
  })

async function run(title: string, cmd: string, args: Record<string, string>) {
  yt.loading = true
  yt.error = ''
  try {
    yt.tracks = (await invoke<Video[]>(cmd, { ...args, cookies: ytCookies() })).map(ytTrack)
    yt.title = title
  } catch (e) {
    yt.error = String(e)
  } finally {
    yt.loading = false
  }
}

/** Searches, or opens the text as a link when it is one. */
export const lookUp = (text: string) =>
  /^https?:\/\//i.test(text.trim()) ? run('From your link', 'youtube_list', { url: text.trim() }) : run(`Results for “${text.trim()}”`, 'youtube_search', { query: text })
export const likedMusic = () => run('Liked Music', 'youtube_list', { url: 'liked' })

// mpv's length-prefixed quoting, so commas or '=' in a path can't split the option list
const quote = (s: string) => `%${new TextEncoder().encode(s).length}%${s}`

// the same sign-in for playback as for search and downloads
$effect.root(() => {
  $effect(() => {
    const c = settings.ytCookies
    if (!mpv.ready) return
    // a cookies file reaches yt-dlp as a checked copy (ytdlp.rs cookie_file), which it may write back to
    const file = c && c !== 'firefox' ? invoke<string>('youtube_cookies', { path: c }).catch(() => '') : Promise.resolve('')
    file.then((copy) => {
      const raw = ['js-runtimes=node', c === 'firefox' ? 'cookies-from-browser=firefox' : copy ? `cookies=${quote(copy)}` : ''].filter(Boolean).join(',')
      set('ytdl-raw-options', raw)
    })
  })
})
