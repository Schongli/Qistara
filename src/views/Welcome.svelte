<script lang="ts">
  import { open } from '@tauri-apps/plugin-dialog'
  import Icon from '../components/Icon.svelte'
  import { addFolder } from '../lib/library.svelte'

  async function pick() {
    const dir = await open({ directory: true, multiple: false, title: 'Choose your music folder' })
    if (typeof dir === 'string') await addFolder(dir)
  }
</script>

<div class="welcome">
  <img class="mark" src="/favicon.svg" alt="" width="84" height="84" />
  <h1 class="t-title">Your music, in true balance.</h1>
  <p class="t-sub">Add the folder where your music and videos live. Qistara plays FLAC, ALAC, WAV, AIFF, APE, WavPack, MP3, AAC, Opus and nearly every video format, and sends lossless audio to your DAC bit-perfect, in exclusive mode. Your Navidrome server, YouTube Music and playlists from Spotify, Apple Music and Tidal join it from Providers.</p>
  <button class="btn primary press" onclick={pick}><Icon name="FolderSimplePlus" size={16} />Add Music Folder…</button>
</div>

<style>
  .welcome { max-width: 460px; margin: 12vh auto 0; text-align: center; display: grid; justify-items: center; gap: 14px; }
  .mark { border-radius: 19px; box-shadow: var(--shadow-art); }
  h1, p { margin: 0; }
  p { font-size: 13px; line-height: 1.55; }
  .btn { margin-top: 8px; height: 34px; padding: 0 18px; }
</style>
