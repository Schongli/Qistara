// App state kept in the library database (written to disk immediately, unlike localStorage).
import { invoke } from '@tauri-apps/api/core'

export async function kvGet<T>(key: string): Promise<T | null> {
  try {
    const v = await invoke<string | null>('kv_get', { key })
    return v ? (JSON.parse(v) as T) : null
  } catch {
    return null
  }
}

/** For state that is saved back (settings, EQ, pins…): a failed read rejects instead of looking like "nothing
 *  saved", so the caller never marks itself loaded and never overwrites the stored value with defaults. */
export async function kvLoad<T>(key: string): Promise<T | null> {
  const v = await invoke<string | null>('kv_get', { key })
  try {
    return v ? (JSON.parse(v) as T) : null
  } catch (e) {
    console.warn(`stored ${key} is unreadable, starting over:`, e)
    return null
  }
}

export function kvSet(key: string, value: unknown) {
  return invoke('kv_set', { key, value: JSON.stringify(value) }).catch((e) => console.warn(`could not save ${key}:`, e))
}
