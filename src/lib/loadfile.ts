// loadfile's arguments (player.svelte.ts loadTrack). mpv 0.38 put an index before the per-file options
// (`<url> <flags> <index> <options>`); 0.37 and older (Ubuntu 24.04's libmpv2, Debian 12's) read `<url> <flags> <options>`.

/** mpv's [major, minor] from its `mpv-version` ("mpv 0.37.0", "mpv v0.38.0-617-g1a2b3c4"); null for a build named
 *  only by its git hash, or before mpv has said. */
export function mpvRelease(version: string | null): [number, number] | null {
  const m = version?.match(/^mpv v?(\d+)\.(\d+)\b/)
  return m ? [Number(m[1]), Number(m[2])] : null
}

/** An unknown version gets 0.38's form: Windows' and Homebrew's mpv are current. */
export function loadfileArgs(path: string, mode: 'replace' | 'append', opts: string, version: string | null): (string | number)[] {
  if (!opts) return [path, mode]
  const r = mpvRelease(version)
  return r && r[0] === 0 && r[1] < 38 ? [path, mode, opts] : [path, mode, -1, opts]
}
