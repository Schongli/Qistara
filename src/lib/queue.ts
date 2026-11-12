// Pure queue operations; queue.svelte.ts holds the state and talks to mpv.

export type Repeat = 'off' | 'all' | 'one'

export function insertAfter<T>(items: T[], index: number, add: T[]): T[] {
  return [...items.slice(0, index + 1), ...add, ...items.slice(index + 1)]
}

/** Shuffles only what comes after the current item, like Up Next in Apple Music. */
export function shuffleRest<T>(items: T[], index: number, rand = Math.random): T[] {
  const rest = items.slice(index + 1)
  for (let i = rest.length - 1; i > 0; i--) {
    const j = Math.floor(rand() * (i + 1))
    ;[rest[i], rest[j]] = [rest[j], rest[i]]
  }
  return [...items.slice(0, index + 1), ...rest]
}

/** Index to play after `index`, or -1 when the queue ends. `manual` = user pressed next. */
export function nextIndex(len: number, index: number, repeat: Repeat, manual: boolean): number {
  if (!len) return -1
  if (repeat === 'one' && !manual) return index
  if (index + 1 < len) return index + 1
  return repeat === 'off' ? -1 : 0
}

export function move<T>(items: T[], from: number, to: number): T[] {
  const out = items.slice()
  const [x] = out.splice(from, 1)
  out.splice(to, 0, x)
  return out
}

/** Keeps `current` pointing at the same item after moving `from` → `to`. */
export function indexAfterMove(current: number, from: number, to: number): number {
  if (from === current) return to
  if (from < current && to >= current) return current - 1
  if (from > current && to <= current) return current + 1
  return current
}

type Pickable = { id: number; kind: string; artist: string; albumArtist: string; genre: string; album: string; loved: boolean; rating: number; lastPlayed: number | null }

/**
 * Autoplay (Apple Music's ∞): songs to keep going with when Up Next runs out. Favours the same
 * artist and genre, loved and well-rated songs; avoids anything queued or played in the last day.
 */
export function autoplayPicks<T extends Pickable>(all: T[], seed: Pickable, exclude: Set<number>, n = 10, rand = Math.random, now = Date.now() / 1000): T[] {
  const artist = (seed.albumArtist || seed.artist).toLowerCase()
  const genre = seed.genre.toLowerCase()
  const scored = all
    .filter((t) => t.kind === 'audio' && t.id > 0 && !exclude.has(t.id))
    .map((t) => {
      let s = rand() * 2 // variety: equal candidates come out in a different order every time
      if (artist && (t.albumArtist || t.artist).toLowerCase() === artist) s += 3
      if (genre && t.genre.toLowerCase() === genre) s += 2
      if (t.album === seed.album) s -= 1.5 // prefer other albums over replaying this one
      if (t.loved) s += 1
      s += t.rating * 0.3
      if (t.lastPlayed && now - t.lastPlayed < 86400) s -= 6 // below even an unrelated song: no same-day repeats
      return { t, s }
    })
  return scored.sort((a, b) => b.s - a.s).slice(0, n).map((x) => x.t)
}
