// Up Next. The queue here is the truth; mpv's playlist only ever holds [current, next] so
// track changes are gapless, and every queue edit re-syncs the "next" slot.
import { untrack } from 'svelte'
import { cmd, engine, loadTrack, lock, mpv, seek, set, warm } from './player.svelte'
import { autoplayPicks, indexAfterMove, insertAfter, move, nextIndex, shuffleRest, type Repeat } from './queue'
import { lib, type Track } from './library.svelte'
import { kvGet, kvSet } from './persist'
import { invoke } from '@tauri-apps/api/core'
import { toast } from './nav.svelte'

class Queue {
  items = $state.raw<Track[]>([])
  index = $state(-1)
  shuffle = $state(false)
  repeat = $state<Repeat>('off')
  autoplay = $state(true) // ∞: keep going with similar songs when Up Next runs out
  stopAfterCurrent = $state(false) // sleep timer: "end of current song"
  #order: number[] = [] // track ids in pre-shuffle order
  #restored = false // saving before restore() would overwrite the last session with an empty queue

  /** What mpv really has lined up after the current song (repeat-one, stop-after and autoplay included). */
  appended = $state.raw<Track | null>(null)

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
        // a guest's own queue waits, saved as it was, while it follows a listening session
        if (this.#restored && !lock.following) untrack(() => save(items, index, shuffle, repeat, autoplay))
      })
    })
  }

  /** True (and says so) while this player follows a listening session's host. */
  #following() {
    if (lock.following) toast("You're listening along. Leave the session to choose what plays.")
    return lock.following
  }

  /** A listening session: play the host's song from `at`, with the host's next one lined up. */
  async follow(cur: Track, next: Track | null, at: number) {
    this.items = next ? [cur, next] : [cur]
    this.index = 0
    await loadTrack(cur, 'replace', at)
    await set('pause', false)
    await this.#syncNext()
  }

  /** A listening session changed what comes after the current song. */
  followNext(next: Track | null) {
    const cur = this.current
    if (!cur) return
    this.items = next ? [cur, next] : [cur]
    return this.#syncNext()
  }

  /** Leaving a session: back to your own queue, paused where you left it. */
  async unfollow() {
    lock.following = false
    await cmd('stop').catch(() => {})
    this.items = []
    this.index = -1
    await this.#restore(new Map(lib.tracks.map((t) => [t.id, t])))
  }

  async play(tracks: Track[], start = 0, shuffle = false) {
    if (!tracks.length || this.#following()) return
    this.#order = tracks.map((t) => t.id)
    this.shuffle = shuffle
    this.items = shuffle ? shuffleRest([tracks[start], ...tracks.filter((_, i) => i !== start)], 0) : tracks
    this.index = shuffle ? 0 : start
    await this.#start()
  }

  playNext(tracks: Track[]) {
    if (this.#following()) return
    if (this.index < 0) return this.play(tracks)
    this.items = insertAfter(this.items, this.index, tracks)
    return this.#syncNext()
  }

  playLater(tracks: Track[]) {
    if (this.#following()) return
    if (this.index < 0) return this.play(tracks)
    this.items = [...this.items, ...tracks]
    return this.#syncNext()
  }

  /** `paused`: load the next song without starting it (the sleep timer ends this way). */
  async next(paused = false) {
    if (this.#following()) return
    const n = nextIndex(this.items.length, this.index, this.repeat, true)
    if (n < 0) return
    this.index = n
    await this.#start(paused)
  }

  async prev() {
    if (this.#following()) return
    if (mpv.time > 3 || this.index <= 0) return seek(0)
    this.index--
    await this.#start()
  }

  async jump(i: number) {
    if (this.#following()) return
    this.index = i
    await this.#start()
  }

  remove(i: number) {
    if (this.#following()) return
    if (i === this.index) return
    this.items = this.items.filter((_, j) => j !== i)
    if (i < this.index) this.index--
    return this.#syncNext()
  }

  reorder(from: number, to: number) {
    if (this.#following()) return
    this.items = move(this.items, from, to)
    this.index = indexAfterMove(this.index, from, to)
    return this.#syncNext()
  }

  clearUpNext() {
    if (this.#following()) return
    this.items = this.items.slice(0, this.index + 1)
    return this.#syncNext()
  }

  toggleShuffle() {
    if (this.#following()) return
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
    if (this.#following()) return
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
    await loadTrack(items[index], 'replace', s.time - 2)
    await this.#syncNext()
  }

  /** Reopens the current file where it is (pause kept), e.g. after switching exclusive mode. */
  async reload() {
    const t = this.current
    if (!t || mpv.idle) return
    const at = mpv.time
    await loadTrack(t, 'replace', at)
    await this.#syncNext()
  }

  /** Plays another version of the current song from the same moment (a YouTube song with or without its video). */
  async swap(t: Track) {
    if (this.#following() || this.index < 0) return
    const at = mpv.time
    this.items = this.items.map((x, i) => (i === this.index ? t : x))
    await loadTrack(t, 'replace', at)
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
    // mpv's ytdl hook (YouTube videos, and songs the streaming engine couldn't open) needs yt-dlp and a JavaScript
    // runtime: say what's missing rather than fail without a sound
    if (t.path.startsWith('ytdl://') && (t.kind === 'video' || engine.viaHook.has(t.path))) {
      const missing = await invoke('youtube_ready').then(() => '', String)
      if (missing) return toast(missing)
    }
    await resumePoint()
    if (paused) {
      await set('pause', true)
      await loadTrack(t, 'replace')
    } else {
      await loadTrack(t, 'replace')
      // keep-open pauses at the end of a file, and the pause carries over to the next load
      await set('pause', false)
    }
    await this.#syncNext()
  }

  async #advanced() {
    // following a session, the next item is simply the next one: the guest's own repeat mode doesn't apply
    this.index = lock.following ? Math.min(this.index + 1, this.items.length - 1) : nextIndex(this.items.length, this.index, this.repeat, false)
    await cmd('playlist-remove', 0)
    await this.#syncNext()
  }

  /** Autoplay ∞ for streamed songs (YT Music radio): more songs that follow on from `t`. Set by ytmusic.svelte.ts. */
  remoteAutoplay: ((t: Track) => Promise<Track[]>) | null = null
  #extending = 0

  /** Up Next ran out on a streamed song: fetch its radio in the background, then line the next one up. */
  #extend(t: Track) {
    if (!this.remoteAutoplay || this.#extending === t.id) return
    this.#extending = t.id
    this.remoteAutoplay(t)
      .then((picks) => {
        if (this.current?.id !== t.id || this.index !== this.items.length - 1) return
        const have = new Set(this.items.map((x) => x.id))
        const fresh = picks.filter((p) => !have.has(p.id)).slice(0, 25)
        if (fresh.length) (this.items = [...this.items, ...fresh]), this.#syncNext()
      })
      .catch((e) => console.warn('autoplay:', e))
      .finally(() => (this.#extending = 0))
  }

  async #syncNext() {
    const t = this.current
    if (!t) return
    if (lock.following) {
      // the host decides what's next: no repeat, autoplay or stop-after of our own
      await set('loop-file', 'no')
      await cmd('playlist-clear')
      this.appended = this.items[this.index + 1] ?? null
      if (this.appended) await loadTrack(this.appended, 'append')
      return
    }
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
    if (n < 0 && this.autoplay && this.repeat === 'off' && t.id <= 0) this.#extend(t)
    this.appended = n >= 0 && this.repeat !== 'one' && !this.stopAfterCurrent ? this.items[n] : null
    if (this.appended) await loadTrack(this.appended, 'append')
    // the song after that, fetched ahead too (loading the next one already does it for that one)
    if (this.appended) warm([this.items[n + 1]])
  }

  /** Opens the current song again from the start, e.g. a YouTube song through mpv's hook once the streaming
   *  engine couldn't open it (youtube.svelte.ts). */
  async retry() {
    const t = this.current
    if (!t) return
    await loadTrack(t, 'replace')
    await set('pause', false)
    await this.#syncNext()
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
  if (mpv.pause || !queue.current || queue.index < 0 || lock.following) return
  save(queue.items, queue.index, queue.shuffle, queue.repeat, queue.autoplay)
  if (queue.current.kind === 'video') resumePoint() // survives a crash or a closed window
}, 5000)
