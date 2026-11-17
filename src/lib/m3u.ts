// M3U / M3U8 playlists (MusicBee, VLC, foobar2000, Winamp all read and write these).

export interface M3uEntry { path: string; title?: string; duration?: number }

const URL_RE = /^[a-z][\w+.-]*:\/\//i
/** A playlist kept in a folder like "/home/…" lists POSIX paths; "D:\…" or "\\nas\…" lists Windows ones. */
export const isPosixDir = (dir: string) => dir.startsWith('/')

/** Normalises a path: `.` and `..` segments, one kind of slash. `..` never climbs above the drive or share. URLs are kept. */
export function normalize(p: string, posix = false): string {
  if (URL_RE.test(p) && !/^file:/i.test(p)) return p
  if (posix) {
    const out: string[] = []
    for (const part of p.split('/')) {
      if (part === '' || part === '.') continue
      if (part === '..') out.pop()
      else out.push(part)
    }
    return '/' + out.join('/')
  }
  const unc = p.startsWith('\\\\') || p.startsWith('//')
  const parts = p.replace(/\//g, '\\').split('\\').filter((x) => x !== '' && x !== '.')
  // the drive (D:) or the server and share (\\nas\music) stay put
  const root = unc ? parts.splice(0, 2) : /^[a-z]:$/i.test(parts[0] ?? '') ? parts.splice(0, 1) : []
  const out: string[] = []
  for (const part of parts) {
    if (part === '..') out.pop()
    else out.push(part)
  }
  return unc ? '\\\\' + [...root, ...out].join('\\') : [...root, ...out].join('\\') + (root.length && !out.length ? '\\' : '')
}

const decode = (s: string) => {
  try {
    return decodeURIComponent(s)
  } catch {
    return s // a stray "%" in a file name
  }
}

/** A file:// URL as a path: file:///D:/a, file://localhost/home/a, file://nas/share/a (a Windows share). */
function fromFileUrl(u: string, posix: boolean): string | null {
  const rest = decode(u.replace(/^file:/i, ''))
  if (rest.startsWith('///')) return posix ? rest.slice(2) : rest.slice(3)
  const m = rest.match(/^\/\/([^/]*)(\/.*)?$/)
  if (!m) return null // "file:x" isn't a file URL
  const [, host, path = ''] = m
  if (!host || host.toLowerCase() === 'localhost') return posix ? path : path.slice(1)
  return posix ? `//${host}${path}` : `\\\\${host}${path}`
}

/** Parses an M3U/M3U8 file; relative entries resolve against the playlist's own folder. */
export function parseM3u(text: string, baseDir: string): M3uEntry[] {
  const posix = isPosixDir(baseDir)
  const entries: M3uEntry[] = []
  let pending: { title?: string; duration?: number } = {}
  for (const raw of text.replace(/^\uFEFF/, '').split(/\r\n|\r|\n/)) {
    const line = raw.trim()
    if (!line) continue
    if (line.startsWith('#')) {
      const m = line.match(/^#EXTINF:\s*(-?\d+(?:\.\d+)?)[^,]*,(.*)$/i)
      if (m) pending = { duration: Number(m[1]) > 0 ? Number(m[1]) : undefined, title: m[2].trim() || undefined }
      continue
    }
    let path: string | null
    if (/^file:/i.test(line)) path = fromFileUrl(line, posix)
    else if (URL_RE.test(line)) path = line
    else if (posix) path = line.startsWith('/') ? line : `${baseDir}/${line}`
    else if (/^[a-z]:[\\/]/i.test(line) || /^(\\\\|\/\/)/.test(line)) path = line
    // "\Music\a.flac": the root of the playlist's own drive
    else if (/^[\\/]/.test(line)) path = (baseDir.match(/^[a-z]:/i)?.[0] ?? '') + line
    else path = `${baseDir}\\${line}`
    const p = path && normalize(path, posix)
    if (p && p !== '/' && p !== '\\\\') entries.push({ path: p, ...pending })
    pending = {}
  }
  return entries
}

/** Writes an extended M3U8 (UTF-8) with absolute paths, which every player understands. */
export function buildM3u(tracks: { path: string; title: string; artist: string; duration: number }[]): string {
  const lines = ['#EXTM3U']
  // a tag holding a line break would start a bogus entry
  const one = (s: string) => s.replace(/[\r\n]+/g, ' ')
  for (const t of tracks) {
    lines.push(`#EXTINF:${Math.round(t.duration) || -1},${t.artist ? `${one(t.artist)} - ` : ''}${one(t.title)}`, one(t.path))
  }
  return lines.join('\r\n') + '\r\n'
}
