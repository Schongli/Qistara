// Up Next. The queue here is the truth; mpv's playlist only ever holds [current, next] so
// track changes are gapless, and every queue edit re-syncs the "next" slot.
import { untrack } from 'svelte'
import { cmd, load, mpv, seek, set } from './player.svelte'
import { autoplayPicks, indexAfterMove, insertAfter, move, nextIndex, shuffleRest, type Repeat } from './queue'
import { lib, type Track } from './library.svelte'
import { kvGet, kvSet } from './persist'

class Queue {
  items = $state.raw<Track[]>([])
  index = $state(-1)
  shuffle = $state(false)
  repeat = $state<Repeat>('off')
  autoplay = $state(true) // ∞: keep going with similar songs when Up Next runs out
  stopAfterCurrent = $state(false) // sleep timer: "end of current song"
  #order: number[] = [] // track ids in pre-shuffle order
  #restored = false // saving before restore() would overwrite the last session with an empty queue

  current = $derived(this.items[this.index] ?? null)
  upNext = $derived(this.items.slice(this.index + 1))

  constructor() {
    $effect.root(() => {
      // mpv moved on to the appended track by itself (gapless advance)
      $effect(() => {
        if (mpv.playlistPos === 1) untrack(() => this.#advanced())
      })
      $effect(() => {
        const { items, index, shuffle, repeat, autoplay } = this
        if (this.#restored) untrack(() => save(items, index, shuffle, repeat, autoplay))
      })
    })
  }

  async play(tracks: Track[], start = 0, shuffle = false) {
    if (!tracks.length) return
    this.#order = tracks.map((t) => t.id)
    this.shuffle = shuffle
    this.items = shuffle ? shuffleRest([tracks[start], ...tracks.filter((_, i) => i !== start)], 0) : tracks
    this.index = shuffle ? 0 : start
    await this.#start()
  }

  playNext(tracks: Track[]) {
    if (this.index < 0) return this.play(tracks)
    this.items = insertAfter(this.items, this.index, tracks)
    return this.#syncNext()
  }

  playLater(tracks: Track[]) {
    if (this.index < 0) return this.play(tracks)
    this.items = [...this.items, ...tracks]
    return this.#syncNext()
  }

  /** `paused`: load the next song without starting it (the sleep timer ends this way). */
  async next(paused = false) {
    const n = nextIndex(this.items.length, this.index, this.repeat, true)
    if (n < 0) return
    this.index = n
    await this.#start(paused)
  }

  async prev() {
    if (mpv.time > 3 || this.index <= 0) return seek(0)
    this.index--
    await this.#start()
  }

  async jump(i: number) {
    this.index = i
    await this.#start()
  }

  remove(i: number) {
    if (i === this.index) return
    this.items = this.items.filter((_, j) => j !== i)
    if (i < this.index) this.index--
    return this.#syncNext()
  }

  reorder(from: number, to: number) {
    this.items = move(this.items, from, to)
    this.index = indexAfterMove(this.index, from, to)
    return this.#syncNext()
  }

  clearUpNext() {
    this.items = this.items.slice(0, this.index + 1)
    return this.#syncNext()
  }

  toggleShuffle() {
    this.shuffle = !this.shuffle
    if (this.shuffle) {
      this.#order = this.items.map((t) => t.id)
      this.items = shuffleRest(this.items, this.index)
    } else {
      // upcoming tracks go back to their original order; tracks queued later stay first
      const pos = new Map(this.#order.map((id, i) => [id, i]))
      const rest = this.items.slice(this.index + 1).sort((a, b) => (pos.get(a.id) ?? -1) - (pos.get(b.id) ?? -1))
      this.items = [...this.items.slice(0, this.index + 1), ...rest]
    }
    return this.#syncNext()
  }

  toggleAutoplay() {
    this.autoplay = !this.autoplay
    return this.#syncNext()
  }

  stopAfter(on: boolean) {
    this.stopAfterCurrent = on
    return this.#syncNext()
  }

  cycleRepeat() {
    this.repeat = this.repeat === 'off' ? 'all' : this.repeat === 'all' ? 'one' : 'off'
    return this.#syncNext()
  }

  /** Restores the last session paused at its position, once the library has loaded. */
  async restore(byId: Map<number, Track>) {
    try {
      await this.#restore(byId)
    } finally {
      // only now may saving start: earlier it would store the half-restored state (time 0)
      this.#restored = true
    }
  }

  async #restore(byId: Map<number, Track>) {
    const s = await kvGet<Saved>('queue')
    if (!s) return
    const items = s.ids.map((id) => byId.get(id)).filter((t): t is Track => !!t)
    const index = items.findIndex((t) => t.id === s.ids[s.index])
    if (index < 0) return
    this.items = items
    this.index = index
    this.shuffle = s.shuffle
    this.repeat = s.repeat
    this.autoplay = s.autoplay ?? true
    this.#order = items.map((t) => t.id)
    await set('pause', true)
    await cmd('loadfile', items[index].path, 'replace', -1, `start=${Math.max(0, s.time - 2)}`)
    await this.#syncNext()
  }

  /** Reopens the current file where it is (pause kept), e.g. after switching exclusive mode. */
  async reload() {
    const t = this.current
    if (!t || mpv.idle) return
    const at = mpv.time
    await cmd('loadfile', t.path, 'replace', -1, `start=${at}`)
    await this.#syncNext()
  }

  /** Stops playback; a video remembers where it was so it resumes next time. */
  async stop() {
    await resumePoint()
    await cmd('stop')
    this.items = []
    this.index = -1
  }

  async #start(paused = false) {
    const t = this.current
    if (!t) return
    await resumePoint()
    if (paused) {
      await set('pause', true)
      await cmd('loadfile', t.path, 'replace')
    } else await load(t.path)
    await this.#syncNext()
  }

  async #advanced() {
    this.index = nextIndex(this.items.length, this.index, this.repeat, false)
    await cmd('playlist-remove', 0)
    await this.#syncNext()
  }

  async #syncNext() {
    const t = this.current
    if (!t) return
    await set('loop-file', this.repeat === 'one' ? 'inf' : 'no')
    await cmd('playlist-clear') // drops everything but the playing entry
    let n = nextIndex(this.items.length, this.index, this.repeat, false)
    if (n < 0 && this.autoplay && this.repeat === 'off' && t.kind === 'audio' && t.id > 0) {
      const picks = autoplayPicks(lib.tracks, t, new Set(this.items.map((x) => x.id)))
      if (picks.length) {
        this.items = [...this.items, ...picks]
        n = this.index + 1
      }
    }
    if (n >= 0 && this.repeat !== 'one' && !this.stopAfterCurrent) await load(this.items[n].path, 'append')
  }
}

// mpv's watch-later file holds the position, audio/subtitle choice and subtitle delay.
// A video watched to (nearly) the end starts over next time, as in VLC.
function resumePoint() {
  if (!mpv.path || mpv.idle || !mpv.tracks.some((x) => x.type === 'video' && !x.image)) return Promise.resolve()
  const nearEnd = mpv.duration > 0 && mpv.duration - mpv.time < Math.min(30, mpv.duration * 0.05)
  return cmd(nearEnd ? 'delete-watch-later-config' : 'write-watch-later-config').catch(() => {})
}

type Saved = { ids: number[]; index: number; shuffle: boolean; repeat: Repeat; autoplay?: boolean; time: number }

function save(items: Track[], index: number, shuffle: boolean, repeat: Repeat, autoplay: boolean) {
  // restore() drops ids that are gone from the library (ad-hoc opened files never come back)
  const ids = items.map((t) => t.id)
  kvSet('queue', { ids, index, shuffle, repeat, autoplay, time: mpv.time } satisfies Saved)
}

export const queue = new Queue()

// position survives a restart: save it every few seconds while playing
setInterval(() => {
  if (mpv.pause || !queue.current || queue.index < 0) return
  save(queue.items, queue.index, queue.shuffle, queue.repeat, queue.autoplay)
  if (queue.current.kind === 'video') resumePoint() // survives a crash or a closed window
}, 5000)
