import { open } from '@tauri-apps/plugin-dialog'
import { getCurrentWebview } from '@tauri-apps/api/webview'
import { fileTrack, mediaExtensions, type Track } from './library.svelte'
import { queue } from './queue.svelte'

export async function playPaths(paths: string[]) {
  const tracks = (await Promise.all(paths.map(fileTrack))).filter((t): t is Track => !!t)
  if (tracks.length) await queue.play(tracks)
}

/** VLC-style "Open File": anything mpv can play, from anywhere on disk. */
export async function openFiles() {
  const { audio, video } = await mediaExtensions()
  const f = await open({
    multiple: true,
    title: 'Open media',
    filters: [{ name: 'Audio and video', extensions: [...audio, ...video] }, { name: 'All files', extensions: ['*'] }],
  })
  if (f) await playPaths(Array.isArray(f) ? f : [f])
}

// files dropped onto the window play right away (folders belong in Settings → Library)
getCurrentWebview().onDragDropEvent((e) => {
  if (e.payload.type === 'drop') playPaths(e.payload.paths)
})
