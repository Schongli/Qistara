// Keeps the Windows media overlay in sync and routes media keys back to the queue.
import { invoke } from '@tauri-apps/api/core'
import { listen } from '@tauri-apps/api/event'
import { untrack } from 'svelte'
import { mpv, onMpvEvent, seek, set, togglePause } from './player.svelte'
import { queue } from './queue.svelte'

const publish = () => invoke('media_state', { playing: !mpv.pause && !!queue.current, position: mpv.time })

$effect.root(() => {
  $effect(() => {
    const t = queue.current
    if (t) invoke('media_track', { path: t.path, title: t.title, artist: t.artist, album: t.album, duration: t.duration })
  })
  // on play/pause only; seeks publish below. Never per tick.
  $effect(() => {
    mpv.pause
    untrack(publish)
  })
})
onMpvEvent((e) => e.event === 'playback-restart' && publish())

listen<string>('media-key', ({ payload }) => {
  if (!queue.current) return
  if (payload === 'toggle') togglePause()
  else if (payload === 'play') set('pause', false)
  else if (payload === 'pause' || payload === 'stop') set('pause', true)
  else if (payload === 'next') queue.next()
  else if (payload === 'previous') queue.prev()
})
listen<number>('media-seek', ({ payload }) => seek(payload))
