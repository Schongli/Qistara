// This machine's OS, from the webview's user agent, and the names people know its parts by.
import { osOf } from './signal'

export const OS = osOf(navigator.userAgent)
export const SYSTEM = { windows: 'Windows', macos: 'macOS', linux: 'Linux' }[OS]
export const FILE_MANAGER = { windows: 'File Explorer', macos: 'Finder', linux: 'Files' }[OS]
export const KEY_STORE = { windows: 'Windows Credential Manager', macos: 'the macOS Keychain', linux: 'the system keyring' }[OS]
/** A phone or tablet build (Tauri mobile): no window to minimise or close, so no caption buttons. */
export const PHONE = typeof navigator !== 'undefined' && /Android|iPhone|iPad/.test(navigator.userAgent)
