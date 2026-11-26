// Signing in to YouTube in Qistara's own window (account.rs): Google's sign-in page opens in a small private
// window, and once it's done the account plays, downloads and shows your YT Music. The cookies stay in Rust; the
// page knows only the name and picture.
import { invoke } from '@tauri-apps/api/core'
import { settings, settingsLoaded } from './settings.svelte'
import { toast } from './nav.svelte'

export interface Account { name: string; handle: string | null; photo: string | null }

export const account = $state({ info: null as Account | null, busy: false, error: '' })

/** Who is signed in. A sign-in the system no longer has (its key gone) counts as signed out. */
export async function loadAccount() {
  account.info = await invoke<Account | null>('account_status').catch(() => null)
  if (!account.info && settings.ytCookies === 'account') settings.ytCookies = ''
}

// known from the start: Providers, YT Music's Library and the sign-in setting all depend on it
settingsLoaded.then(loadAccount)

export async function signIn() {
  // already open: the window comes to the front
  if (account.busy) return void invoke('account_sign_in').catch(() => {})
  account.busy = true
  account.error = ''
  try {
    account.info = await invoke<Account>('account_sign_in')
    settings.ytCookies = 'account'
    toast(`Signed in to YouTube as ${account.info.name}.`)
  } catch (e) {
    const why = String(e)
    // closing the window is how you change your mind; "busy" means it's already open (and now in front)
    if (why !== 'closed' && why !== 'busy') account.error = why
  } finally {
    account.busy = false
  }
}

export async function signOut() {
  await invoke('account_sign_out').catch((e) => console.warn('YouTube sign-out:', e))
  account.info = null
  if (settings.ytCookies === 'account') settings.ytCookies = ''
}
