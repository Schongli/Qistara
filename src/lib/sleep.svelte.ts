// Sleep timer: pause after N minutes, or at the end of the current song (the next one is cued, paused).
import { untrack } from 'svelte'
import { lock, mpv, set } from './player.svelte'
import { queue } from './queue.svelte'

export const sleep = $state({ until: 0, endOfSong: false, left: 0 })

export function setSleep(choice: number | 'song' | null) {
  sleep.until = typeof choice === 'number' ? Date.now() + choice * 60_000 : 0
  sleep.endOfSong = choice === 'song'
  queue.stopAfter(sleep.endOfSong)
  tick()
}

function tick() {
  sleep.left = sleep.until ? Math.max(0, Math.ceil((sleep.until - Date.now()) / 60_000)) : 0
  if (sleep.until && Date.now() >= sleep.until) {
    sleep.until = 0
    set('pause', true)
  }
}
setInterval(tick, 10_000)

$effect.root(() => {
  // the song ran out with nothing queued behind it: cue the next one, paused, and end the timer
  $effect(() => {
    // a listening session's guest doesn't choose when songs end
    if (mpv.eof && sleep.endOfSong && !lock.following) {
      untrack(async () => {
        sleep.endOfSong = false
        await queue.stopAfter(false)
        await queue.next(true)
      })
    }
  })
})

export const sleepLabel = () => (sleep.endOfSong ? 'End of song' : sleep.until ? `${sleep.left} min` : '')
