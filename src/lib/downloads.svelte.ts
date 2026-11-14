// Downloads with yt-dlp (ytdlp.rs): the tools, and this session's jobs with their progress.
import { invoke } from '@tauri-apps/api/core'
import { listen } from '@tauri-apps/api/event'
import { audioDir, join } from '@tauri-apps/api/path'
import { addFolder, lib } from './library.svelte'
import { toast } from './nav.svelte'
import { settings } from './settings.svelte'

export type Preset = 'audioOriginal' | 'audioMp3' | 'video1080' | 'videoBest'
export const PRESETS: { id: Preset; label: string; hint: string; ffmpeg?: boolean }[] = [
  { id: 'audioOriginal', label: 'Audio', hint: 'The best audio as published (Opus or AAC), not re-encoded.' },
  { id: 'audioMp3', label: 'MP3', hint: 'Converted to MP3 at the highest VBR quality.', ffmpeg: true },
  { id: 'video1080', label: 'Video 1080p', hint: 'Up to 1080p, with subtitles when ffmpeg is there.' },
  { id: 'videoBest', label: 'Video, best', hint: 'The highest resolution published, 4K included.' },
]

export interface Job {
  id: number
  urls: string[]
  preset: Preset
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
  tools: null as null | { ytdlp: string | null; ffmpeg: string | null },
  installing: '' as '' | 'ytdlp' | 'ffmpeg',
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

export async function install(tool: 'ytdlp' | 'ffmpeg') {
  dl.error = ''
  dl.installing = tool
  dl.toolDone = dl.toolTotal = 0
  try {
    await invoke('tool_install', { tool })
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

const inLibrary = (dir: string) => lib.folders.some((f) => dir.toLowerCase().startsWith(f.toLowerCase().replace(/[\\/]*$/, '\\')) || dir.toLowerCase() === f.toLowerCase())

/** Queues one job for all the links. The folder joins the library once the job is in (download_add
 *  creates it); its rescan picks up whatever is already saved, the watcher the rest. */
export async function add(urls: string[], preset: Preset, dest: string, addToLibrary: boolean) {
  dl.error = ''
  try {
    const id = await invoke<number>('download_add', { urls, preset, dest, cookies: ytCookies() })
    dl.jobs = [{ id, urls, preset, status: 'queued', percent: null, eta: '', title: '', files: [], error: '' }, ...dl.jobs]
    if (addToLibrary && !inLibrary(dest)) await addFolder(dest)
  } catch (e) {
    dl.error = String(e)
  }
}

export const cancel = (id: number) => invoke('download_cancel', { id })
export const clearFinished = () => (dl.jobs = dl.jobs.filter((j) => j.status === 'queued' || j.status === 'running'))

/** "Download" from a track's menu: best audio into the download folder. */
export async function quickDownload(urls: string[]) {
  await add(urls, 'audioOriginal', await downloadFolder(), settings.downloadToLibrary)
  toast(dl.error || (urls.length > 1 ? `Downloading ${urls.length} songs…` : 'Downloading…'))
}

/** Settings › YouTube sign-in, as ytdlp.rs takes it. */
export const ytCookies = () =>
  settings.ytCookies === 'firefox' ? { kind: 'firefox' } : settings.ytCookies ? { kind: 'file', path: settings.ytCookies } : null
