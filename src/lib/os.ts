// This machine's OS, from the webview's user agent, and the names people know its parts by.
import { osOf } from './signal'

export const OS = osOf(navigator.userAgent)
export const SYSTEM = { windows: 'Windows', macos: 'macOS', linux: 'Linux' }[OS]
export const FILE_MANAGER = { windows: 'File Explorer', macos: 'Finder', linux: 'Files' }[OS]
export const KEY_STORE = { windows: 'Windows Credential Manager', macos: 'the macOS Keychain', linux: 'the system keyring' }[OS]
/**
 * Video inside the window: mpv draws under the page through the window's handle. The mpv plugin can't do that on
 * Linux and it's untried on macOS, so there video files play their sound and the video features stay hidden.
 */
export const SHOWS_VIDEO = OS === 'windows'
/** A phone or tablet build (Tauri mobile): no window to minimise or close, so no caption buttons. */
export const PHONE = typeof navigator !== 'undefined' && /Android|iPhone|iPad/.test(navigator.userAgent)
