// M3U / M3U8 playlists (MusicBee, VLC, foobar2000, Winamp all read and write these).

export interface M3uEntry { path: string; title?: string; duration?: number }

const isAbsolute = (p: string) => /^[a-z]:[\\/]/i.test(p) || p.startsWith('\\\\') || p.startsWith('/') || /^[a-z][\w+.-]*:\/\//i.test(p)

/** Normalises Windows paths: forward slashes, `.` and `..` segments. URLs are kept as they are. */
export function normalize(p: string): string {
  if (/^[a-z][\w+.-]*:\/\//i.test(p) && !/^file:/i.test(p)) return p
  const unc = p.startsWith('\\\\') || p.startsWith('//')
  const out: string[] = []
  for (const part of p.replace(/\//g, '\\').split('\\')) {
    if (part === '' || part === '.') continue
    if (part === '..') out.pop()
    else out.push(part)
  }
  return (unc ? '\\\\' : '') + out.join('\\')
}

/** Parses an M3U/M3U8 file; relative entries resolve against the playlist's own folder. */
export function parseM3u(text: string, baseDir: string): M3uEntry[] {
  const entries: M3uEntry[] = []
  let pending: { title?: string; duration?: number } = {}
  for (const raw of text.replace(/^﻿/, '').split(/\r?\n/)) {
    const line = raw.trim()
    if (!line) continue
    if (line.startsWith('#')) {
      const m = line.match(/^#EXTINF:\s*(-?\d+(?:\.\d+)?)[^,]*,(.*)$/i)
      if (m) pending = { duration: Number(m[1]) > 0 ? Number(m[1]) : undefined, title: m[2].trim() || undefined }
      continue
    }
    let path = line.replace(/^file:\/\/\/?/i, '')
    if (/^file:/i.test(line)) path = decodeURIComponent(path)
    entries.push({ path: normalize(isAbsolute(path) ? path : `${baseDir}\\${path}`), ...pending })
    pending = {}
  }
  return entries
}

/** Writes an extended M3U8 (UTF-8) with absolute paths, which every player understands. */
export function buildM3u(tracks: { path: string; title: string; artist: string; duration: number }[]): string {
  const lines = ['#EXTM3U']
  for (const t of tracks) {
    lines.push(`#EXTINF:${Math.round(t.duration) || -1},${t.artist ? `${t.artist} - ` : ''}${t.title}`, t.path)
  }
  return lines.join('\r\n') + '\r\n'
}
