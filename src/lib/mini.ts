// MiniPlayer: shrinks the one app window (mpv lives in it) instead of opening a second one.
import { getCurrentWindow, LogicalSize, PhysicalPosition } from '@tauri-apps/api/window'
import { ui } from './nav.svelte'

const win = getCurrentWindow()
let before: { w: number; h: number; x: number; y: number; maximized: boolean } | null = null

export async function enterMini() {
  const maximized = await win.isMaximized()
  if (maximized) await win.unmaximize()
  // the window's normal size and place (after unmaximizing), so it comes back exactly there
  const size = (await win.innerSize()).toLogical(await win.scaleFactor())
  const pos = await win.outerPosition()
  before = { w: size.width, h: size.height, x: pos.x, y: pos.y, maximized }
  ui.nowPlaying = false
  ui.queue = false
  ui.mini = true
  await win.setMinSize(new LogicalSize(300, 96))
  await win.setSize(new LogicalSize(400, 104))
  await win.setAlwaysOnTop(true)
}

export async function exitMini() {
  await win.setAlwaysOnTop(false)
  await win.setMinSize(new LogicalSize(940, 600))
  await win.setSize(new LogicalSize(Math.max(940, before?.w ?? 1280), Math.max(600, before?.h ?? 820)))
  if (before) await win.setPosition(new PhysicalPosition(before.x, before.y))
  else await win.center()
  if (before?.maximized) await win.maximize()
  ui.mini = false
}
