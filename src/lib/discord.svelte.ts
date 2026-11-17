// Discord Rich Presence (discord.rs): what plays, shown on your Discord profile while the setting is on.
// A local file's cover can't reach Discord, so "Artwork for local files" looks its album up on MusicBrainz
// (artist and album only) and hands Discord the Cover Art Archive picture; otherwise the Qistara logo shows.
import { invoke } from '@tauri-apps/api/core'
import { untrack } from 'svelte'
import { mpv } from './player.svelte'
import { queue } from './queue.svelte'
import { settings } from './settings.svelte'
import { session } from './session.svelte'
import { searchReleases } from './musicbrainz'
import type { Track } from './library.svelte'

export const discord = $state({ ready: false, cover: null as string | null })
invoke<boolean>('discord_ready').then((r) => (discord.ready = r), () => {})

const covers = new Map<string, Promise<string | null>>()
function albumCover(t: Track): Promise<string | null> {
  const key = `${t.albumArtist || t.artist}\u0000${t.album}`.toLowerCase()
  if (!covers.has(key))
    covers.set(key, searchReleases(t.album, t.albumArtist || t.artist).then((r) => (r[0] ? `https://coverartarchive.org/release/${r[0].id}/front-250` : null), () => null))
  return covers.get(key)!
}

/** The cover Discord may show: a public https picture (YT Music, YouTube) or a looked-up album cover. */
export async function coverFor(t: Track): Promise<string | null> {
  if (t.art?.startsWith('https://')) return t.art
  if (t.id > 0 && t.album && settings.discordLocalArt) return albumCover(t)
  return null
}

let shown = false
async function update() {
  const t = queue.current
  if (!settings.discord || !discord.ready || !t || mpv.pause) {
    discord.cover = null
    if (shown) await invoke('discord_set', { activity: null }).catch(() => {})
    shown = false
    return
  }
  const cover = await coverFor(t)
  if (queue.current !== t) return
  discord.cover = cover
  const start = Date.now() - mpv.time * 1000
  const activity = {
    title: t.title, artist: t.artist, album: t.album, cover, start: Math.round(start),
    end: t.duration > 0 ? Math.round(start + t.duration * 1000) : null,
    party: session.role ? [session.guests.length + 1, 17] : null,
  }
  await invoke('discord_set', { activity }).then(() => (shown = true), (e) => console.warn('discord:', e))
}

$effect.root(() => {
  $effect(() => {
    void [settings.discord, settings.discordLocalArt, discord.ready, queue.current, mpv.pause, session.role, session.guests.length]
    untrack(() => void update())
  })
  // a seek moves the time bar on Discord too
  let prev = 0
  $effect(() => {
    const t = mpv.time
    if (shown && Math.abs(t - prev) > 3) untrack(() => void update())
    prev = t
  })
})
