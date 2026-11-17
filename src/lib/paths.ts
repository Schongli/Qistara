// Paths as the scanner reports them: "D:\Music\a.flac" or "\\nas\music\a.flac" on Windows, "/home/me/a.flac"
// on Linux and macOS. The path itself says which rules apply, so this works the same on every OS.

const posix = (p: string) => p.startsWith('/')

/** For comparing: Windows paths ignore case, POSIX paths don't. */
export const pathKey = (p: string) => (posix(p) ? p : p.toLowerCase())

export const joinPath = (dir: string, name: string) =>
  posix(dir) ? `${dir.replace(/\/+$/, '')}/${name}` : `${dir.replace(/[\\/]+$/, '')}\\${name}`

/** `path` is `folder` itself or inside it. */
export function isUnder(path: string, folder: string): boolean {
  const f = pathKey(folder).replace(posix(folder) ? /\/+$/ : /[\\/]+$/, '')
  const p = pathKey(path)
  return p === f || p.startsWith(f + '/') || (!posix(folder) && p.startsWith(f + '\\'))
}
