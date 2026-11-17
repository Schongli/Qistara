// Downloads with yt-dlp (ytdlp.rs): the tools, and this session's jobs with their progress.
import { isUnder } from './paths'
import { invoke } from '@tauri-apps/api/core'
import { listen } from '@tauri-apps/api/event'
import { audioDir, join } from '@tauri-apps/api/path'
import { addFolder, lib } from './library.svelte'
import { toast } from './nav.svelte'
import { settings } from './settings.svelte'

// What a download keeps (ytdlp.rs Choice): video up to a height, or audio as published or as MP3.
export type VideoQ = 'best' | '2160' | '1440' | '1080' | '720' | '480' | '360' | '240'
export type AudioQ = 'original' | 'm4a' | 'mp3-320' | 'mp3-192' | 'mp3-128'
export type Choice = { kind: 'video'; quality: VideoQ } | { kind: 'audio'; quality: AudioQ }
export const VIDEO_Q: VideoQ[] = ['best', '2160', '1440', '1080', '720', '480', '360', '240']
export const AUDIO_Q: { id: AudioQ; label: string; hint: string }[] = [
  { id: 'original', label: 'Original', hint: 'The best audio as published (Opus or AAC), not re-encoded.' },
  { id: 'm4a', label: 'M4A', hint: 'AAC as published, not re-encoded: plays on every device.' },
  { id: 'mp3-320', label: 'MP3 320', hint: 'Converted to MP3 at 320 kbps.' },
  { id: 'mp3-192', label: 'MP3 192', hint: 'Converted to MP3 at 192 kbps.' },
  { id: 'mp3-128', label: 'MP3 128', hint: 'Converted to MP3 at 128 kbps: small files.' },
]
export const videoLabel = (q: VideoQ) => (q === 'best' ? 'Best' : `${q}p`)
export const needsFfmpeg = (c: Choice) => c.kind === 'audio' && c.quality.startsWith('mp3')
/** The choice saved in Settings, which quick downloads use too. */
export const savedChoice = (): Choice =>
  settings.downloadKind === 'video' ? { kind: 'video', quality: settings.downloadVideo } : { kind: 'audio', quality: settings.downloadAudio }
/** "1080p MKV", "MP3 320 kbps", "Original audio": the Download button's summary. */
export function choiceLabel(c: Choice, ffmpeg: boolean): string {
  if (c.kind === 'video') return `${videoLabel(c.quality)} ${ffmpeg ? 'MKV' : 'video'}`
  if (c.quality === 'original') return 'Original audio'
  if (c.quality === 'm4a') return 'M4A audio'
  return `MP3 ${c.quality.slice(4)} kbps`
}

/** What one link offers (ytdlp.rs Probe): the sizes under the quality chips. */
export interface Probe {
  title: string
  uploader: string
  duration: number | null
  top: number | null
  thumbnail: string | null
  video: [VideoQ, number | null][]
  audio: [AudioQ, number | null][]
}
export const probe = (url: string) => invoke<Probe>('download_probe', { url, cookies: ytCookies() })

export interface Job {
  id: number
  urls: string[]
  choice: Choice
  status: 'queued' | 'running' | 'done' | 'failed' | 'cancelled'
  percent: number | null
  eta: string
  title: string
  files: string[]
  error: string
}
type Progress = { id: number; status: Job['status']; percent?: number; eta?: string; title?: string; file?: string; error?: string }

export const dl = $state({
  jobs: [] as Job[],
  // js: the JavaScript runtime YouTube uses; ffmpegDownload: false on macOS (Homebrew instead)
  tools: null as null | { ytdlp: string | null; ffmpeg: string | null; js: string | null; ffmpeg_download: boolean },
  installing: '' as '' | Tool,
  toolDone: 0,
  toolTotal: 0,
  error: '',
})

listen<Progress>('download-progress', ({ payload: p }) => {
  const j = dl.jobs.find((x) => x.id === p.id)
  if (!j) return
  j.status = p.status
  if (p.percent !== undefined) j.percent = p.percent
  if (p.eta !== undefined) j.eta = p.eta
  if (p.title) j.title = p.title
  if (p.file) j.files.push(p.file)
  if (p.error) j.error = p.error
})
listen<{ tool: string; done: number; total: number }>('tool-progress', ({ payload }) => {
  dl.toolDone = payload.done
  dl.toolTotal = payload.total
})

export async function refreshTools() {
  dl.tools = await invoke('tools_status')
}

export type Tool = 'ytdlp' | 'ffmpeg' | 'deno'

export async function install(tool: Tool) {
  dl.error = ''
  dl.installing = tool
  dl.toolDone = dl.toolTotal = 0
  try {
    await invoke('tool_install', { tool })
    // YouTube playback picks up the new runtime (mpv.rs builds the hook's options)
    if (tool === 'deno') await invoke('mpv_ytdl_sign_in', { cookies: settings.ytCookies }).catch(() => {})
    await refreshTools()
  } catch (e) {
    dl.error = String(e)
  } finally {
    dl.installing = ''
  }
}

export async function updateYtdlp() {
  dl.error = ''
  try {
    await invoke('ytdlp_update')
    await refreshTools()
  } catch (e) {
    dl.error = String(e)
  }
}

export const defaultFolder = async () => join(await audioDir(), 'Qistara Downloads')
/** Where downloads go: the folder picked in Downloads, kept in settings, or the default. */
export const downloadFolder = async () => settings.downloadFolder || defaultFolder()

const inLibrary = (dir: string) => lib.folders.some((f) => isUnder(dir, f))

/** Queues one job for all the links. The folder joins the library once the job is in (download_add
 *  creates it); its rescan picks up whatever is already saved, the watcher the rest. */
export async function add(urls: string[], choice: Choice, dest: string, addToLibrary: boolean) {
  dl.error = ''
  try {
    const id = await invoke<number>('download_add', { urls, choice, dest, cookies: ytCookies() })
    dl.jobs = [{ id, urls, choice, status: 'queued', percent: null, eta: '', title: '', files: [], error: '' }, ...dl.jobs]
    if (addToLibrary && !inLibrary(dest)) await addFolder(dest)
  } catch (e) {
    dl.error = String(e)
  }
}

export const cancel = (id: number) => invoke('download_cancel', { id })
export const clearFinished = () => (dl.jobs = dl.jobs.filter((j) => j.status === 'queued' || j.status === 'running'))

/** "Download" from a track's menu: the format and quality last chosen in Downloads, into its folder. */
export async function quickDownload(urls: string[]) {
  await add(urls, savedChoice(), await downloadFolder(), settings.downloadToLibrary)
  toast(dl.error || (urls.length > 1 ? `Downloading ${urls.length} songs…` : 'Downloading…'))
}

/** Settings › YouTube sign-in, as ytdlp.rs takes it. */
export const ytCookies = () =>
  settings.ytCookies === 'firefox' ? { kind: 'firefox' } : settings.ytCookies ? { kind: 'file', path: settings.ytCookies } : null
