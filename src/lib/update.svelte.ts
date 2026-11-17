// Updates (update.rs): a check once a day, a few seconds after start, while Settings › General › Updates allows
// it; Check Now; and Install and Restart. Nothing is downloaded until you choose to install.
import { invoke } from '@tauri-apps/api/core'
import { listen } from '@tauri-apps/api/event'
import { settings } from './settings.svelte'
import { kvGet, kvSet } from './persist'
import { toast } from './nav.svelte'

export interface Release {
  version: string
  notes: string
  /** seconds since 1970 */
  date: number | null
}

const NOT_SET_UP = 'updates-not-set-up'
const DAY = 86_400_000

export const update = $state({
  /** false: this build has no updater key, so it can't update itself */
  ready: true,
  checking: false,
  installing: false,
  found: null as Release | null,
  /** bytes downloaded, of how many */
  progress: null as [number, number | null] | null,
  error: '',
  /** when the last check finished (ms), 0 = never */
  checked: 0,
})

export async function checkUpdate(quiet = false) {
  if (update.checking || update.installing) return
  update.checking = true
  update.error = ''
  try {
    update.found = await invoke<Release | null>('update_check')
    update.checked = Date.now()
    void kvSet('updateChecked', update.checked)
    if (quiet && update.found) toast(`Qistara ${update.found.version} is available: Settings › General › Updates.`)
  } catch (e) {
    if (String(e) === NOT_SET_UP) update.ready = false
    else if (!quiet) update.error = String(e)
  } finally {
    update.checking = false
  }
}

/** Downloads and installs what the last check found; Qistara restarts into the new version. */
export async function installUpdate() {
  if (update.installing) return
  update.installing = true
  update.error = ''
  const off = await listen<[number, number | null]>('update-progress', (e) => (update.progress = e.payload))
  try {
    await invoke('update_install')
  } catch (e) {
    update.error = String(e)
    update.installing = false
    update.progress = null
  } finally {
    off()
  }
}

kvGet<number>('updateChecked').then((t) => {
  update.checked = t ?? 0
  setTimeout(() => {
    if (settings.autoUpdate && Date.now() - update.checked > DAY) void checkUpdate(true)
  }, 8000)
})
