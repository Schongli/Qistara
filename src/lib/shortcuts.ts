// Keyboard shortcuts: one table for App's key handler, the Keyboard Shortcuts sheet (Ctrl+/) and the user guide.
// Keys are written the Windows way ("Ctrl+Shift+L"); on macOS Ctrl shows, and is pressed, as ⌘.

export type Group = 'Playback' | 'Library' | 'Window' | 'Video'
export interface Shortcut { keys: string; label: string; group: Group }

export const SHORTCUTS: Shortcut[] = [
  { keys: 'Space', label: 'Play / pause', group: 'Playback' },
  { keys: 'Ctrl+Right', label: 'Next song', group: 'Playback' },
  { keys: 'Ctrl+Left', label: 'Previous song', group: 'Playback' },
  { keys: 'Shift+Right', label: 'Forward 5 seconds', group: 'Playback' },
  { keys: 'Shift+Left', label: 'Back 5 seconds', group: 'Playback' },
  { keys: 'Ctrl+Up', label: 'Volume up', group: 'Playback' },
  { keys: 'Ctrl+Down', label: 'Volume down', group: 'Playback' },
  { keys: 'Ctrl+S', label: 'Shuffle on / off', group: 'Playback' },
  { keys: 'Ctrl+R', label: 'Repeat: off, all, one', group: 'Playback' },
  { keys: 'Ctrl+L', label: 'Love the playing song', group: 'Playback' },
  { keys: 'Ctrl+Shift+V', label: 'Show the playing song’s video', group: 'Playback' },
  { keys: 'Ctrl+F', label: 'Search (in Settings: search settings)', group: 'Library' },
  { keys: 'Ctrl+N', label: 'New playlist', group: 'Library' },
  { keys: 'Ctrl+O', label: 'Open files', group: 'Library' },
  { keys: 'Ctrl+I', label: 'Get Info for the playing song', group: 'Library' },
  { keys: 'Alt+Left', label: 'Back', group: 'Window' },
  { keys: 'Alt+Right', label: 'Forward', group: 'Window' },
  { keys: 'Ctrl+Shift+L', label: 'Lyrics pane', group: 'Window' },
  { keys: 'Ctrl+Shift+Q', label: 'Up Next pane', group: 'Window' },
  { keys: 'Ctrl+Shift+M', label: 'MiniPlayer', group: 'Window' },
  { keys: 'Ctrl+,', label: 'Settings', group: 'Window' },
  { keys: 'Ctrl+/', label: 'Keyboard shortcuts', group: 'Window' },
  { keys: 'Esc', label: 'Close Now Playing, a sheet or the side pane', group: 'Window' },
  // handled by VideoView while a video plays
  { keys: 'Left', label: 'Back 5 s (Shift: 30 s)', group: 'Video' },
  { keys: 'Right', label: 'Forward 5 s (Shift: 30 s)', group: 'Video' },
  { keys: 'Up', label: 'Volume up', group: 'Video' },
  { keys: 'Down', label: 'Volume down', group: 'Video' },
  { keys: 'F', label: 'Full screen (F11 too)', group: 'Video' },
  { keys: 'M', label: 'Mute', group: 'Video' },
  { keys: 'V', label: 'Next subtitle', group: 'Video' },
  { keys: 'B', label: 'Next audio track', group: 'Video' },
  { keys: '[', label: 'Slower', group: 'Video' },
  { keys: ']', label: 'Faster', group: 'Video' },
  { keys: '=', label: 'Normal speed', group: 'Video' },
  { keys: ',', label: 'One frame back', group: 'Video' },
  { keys: '.', label: 'One frame forward', group: 'Video' },
  { keys: 'L', label: 'A-B loop', group: 'Video' },
  { keys: 'I', label: 'File info', group: 'Video' },
  { keys: 'Ctrl+S', label: 'Screenshot', group: 'Video' },
]

const NAMES: Record<string, string> = { ' ': 'Space', ArrowUp: 'Up', ArrowDown: 'Down', ArrowLeft: 'Left', ArrowRight: 'Right', Escape: 'Esc' }

type Keyish = Pick<KeyboardEvent, 'key' | 'ctrlKey' | 'metaKey' | 'shiftKey' | 'altKey'>

/**
 * The combination a key press makes, in the table's spelling: "Ctrl+Shift+L", "Shift+Right", "?". ⌘ counts as Ctrl
 * (macOS). Shift is left out for keys that need it to be typed at all (?), so "?" stays "?".
 */
export function combo(e: Keyish): string {
  const key = NAMES[e.key] ?? (e.key.length === 1 ? e.key.toUpperCase() : e.key)
  const typedWithShift = e.key.length === 1 && !/[a-z]/i.test(e.key) && e.shiftKey
  return [e.ctrlKey || e.metaKey ? 'Ctrl' : '', e.altKey ? 'Alt' : '', e.shiftKey && !typedWithShift ? 'Shift' : '', key].filter(Boolean).join('+')
}

const ARROWS: Record<string, string> = { Up: '↑', Down: '↓', Left: '←', Right: '→' }

/** How the keys read on this OS: arrows as ← →, and ⌘ instead of Ctrl on macOS. */
export const display = (keys: string, mac = false) =>
  keys
    .split('+')
    .map((k) => ARROWS[k] ?? (mac && k === 'Ctrl' ? '⌘' : mac && k === 'Alt' ? '⌥' : mac && k === 'Shift' ? '⇧' : k))
    .join(mac ? '' : '+')
