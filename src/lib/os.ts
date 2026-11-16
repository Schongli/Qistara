// This machine's OS, from the webview's user agent, and the names people know its parts by.
import { osOf } from './signal'

export const OS = osOf(navigator.userAgent)
export const SYSTEM = { windows: 'Windows', macos: 'macOS', linux: 'Linux' }[OS]
export const FILE_MANAGER = { windows: 'File Explorer', macos: 'Finder', linux: 'Files' }[OS]
export const KEY_STORE = { windows: 'Windows Credential Manager', macos: 'the macOS Keychain', linux: 'the system keyring' }[OS]
