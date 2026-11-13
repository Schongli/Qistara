// Importing a playlist from Spotify, Apple Music or Tidal as a playlist of your own matching files.
import { createPlaylist, lib, setPlaylistItems } from './library.svelte'
import { kvGet, kvSet } from './persist'
import { matchTracks, type RemoteTrack } from './spotify'

export interface ImportResult { found: number; total: number; missing: RemoteTrack[]; playlist?: number; error?: string }

/** `mapKey` (kv): remote playlist id → our playlist id, so importing again updates the same playlist;
 *  each service's sign-out clears its map. */
export async function importInto(mapKey: string, id: string, name: string, songs: RemoteTrack[]): Promise<ImportResult> {
  const { ids, missing } = matchTracks(songs, lib.tracks)
  const map = (await kvGet<Record<string, number>>(mapKey)) ?? {}
  // update the earlier import unless it was deleted (or turned into something else)
  const earlier = lib.playlists.find((p) => p.id === map[id] && !p.rules)
  if (earlier) await setPlaylistItems(earlier.id, ids)
  else map[id] = await createPlaylist(name, null, ids)
  await kvSet(mapKey, map)
  return { found: ids.length, total: songs.length, missing, playlist: map[id] }
}
